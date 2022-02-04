#!/usr/bin/python
# Copyright (C) 2019 Jelmer Vernooij <jelmer@jelmer.uk>
#
# This program is free software; you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation; either version 2 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program; if not, write to the Free Software
# Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA

"""Serve the janitor site."""

import asyncio
import functools
import logging
import os
import re
import shutil
import tempfile
import time
import uuid

import aiozipkin
from aiohttp.web_urldispatcher import (
    URL,
)
from aiohttp import web, ClientSession, ClientConnectorError
from aiohttp_openmetrics import metrics, metrics_middleware
from aiohttp.web import middleware
from aiohttp.web_middlewares import normalize_path_middleware
import gpg

from .. import state
from ..config import get_suite_config
from ..logs import get_log_manager
from ..pubsub import pubsub_reader, pubsub_handler, Topic
from ..vcs import get_vcs_manager

from . import (
    html_template,
    is_admin,
    render_template_for_request,
    check_qa_reviewer,
)


FORWARD_CLIENT_TIMEOUT = 30 * 60


def create_background_task(fn, title):
    loop = asyncio.get_event_loop()
    task = loop.create_task(fn)

    def log_result(future):
        try:
            future.result()
        except BaseException:
            logging.exception('%s failed', title)
        except asyncio.CancelledError:
            logging.debug('%s cancelled', title)
        else:
            logging.debug('%s succeeded', title)
    task.add_done_callback(log_result)
    return task


@middleware
async def openid_middleware(request, handler):
    session_id = request.cookies.get("session_id")
    if session_id is not None:
        async with request.app.database.acquire() as conn:
            row = await conn.fetchrow(
                "SELECT userinfo FROM site_session WHERE id = $1",
                session_id)
            if row is not None:
                (userinfo,) = row
            else:
                # Session expired?
                userinfo = None
    else:
        userinfo = None
    request['user'] = userinfo
    resp = await handler(request)
    return resp


def setup_debsso(app):
    app.middlewares.insert(0, openid_middleware)


async def get_credentials(session, publisher_url):
    url = URL(publisher_url) / "credentials"
    async with session.get(url=url) as resp:
        if resp.status != 200:
            raise Exception("unexpected response")
        return await resp.json()


async def handle_simple(templatename, request):
    vs = {}
    return web.Response(
        content_type="text/html",
        text=await render_template_for_request(templatename, request, vs),
        headers={"Cache-Control": "max-age=3600"},
    )


@html_template("generic-start.html")
async def handle_generic_start(request):
    return {"suite": request.match_info["suite"]}


@html_template("generic-candidates.html", headers={"Cache-Control": "max-age=3600"})
async def handle_generic_candidates(request):
    from .common import generate_candidates

    return await generate_candidates(
        request.app.database, suite=request.match_info["suite"]
    )


@html_template("merge-proposals.html", headers={"Cache-Control": "max-age=60"})
async def handle_merge_proposals(request):
    from .merge_proposals import write_merge_proposals

    suite = request.match_info["suite"]
    return await write_merge_proposals(request.app.database, suite)


async def handle_apt_repo(request):
    suite = request.match_info["suite"]
    from .apt_repo import get_published_packages

    async with request.app.database.acquire() as conn:
        vs = {
            "packages": await get_published_packages(conn, suite),
            "suite": suite,
            "suite_config": get_suite_config(request.app['config'], suite),
        }
        text = await render_template_for_request(suite + ".html", request, vs)
        return web.Response(
            content_type="text/html",
            text=text,
            headers={"Cache-Control": "max-age=60"},
        )


@html_template("history.html", headers={"Cache-Control": "max-age=10"})
async def handle_history(request):
    limit = int(request.query.get("limit", "100"))
    offset = int(request.query.get("offset", "0"))

    query = """\
SELECT finish_time, package, suite, worker_link,
worker as worker_name, finish_time - start_time AS duration,
result_code, id, description FROM run
ORDER BY finish_time DESC"""
    if offset:
        query += ' OFFSET %d' % offset
    if limit:
        query += ' LIMIT %d' % limit
    async with request.app.database.acquire() as conn:
        runs = await conn.fetch(query)
    return {
        "count": limit,
        "history": runs
    }


@html_template("credentials.html", headers={"Cache-Control": "max-age=10"})
async def handle_credentials(request):
    try:
        credentials = await get_credentials(
            request.app.http_client_session, request.app.publisher_url
        )
    except ClientConnectorError:
        return web.Response(status=500, text='Unable to retrieve credentials')
    pgp_fprs = []
    for keydata in credentials["pgp_keys"]:
        result = request.app.gpg.key_import(keydata.encode("utf-8"))
        pgp_fprs.extend([i.fpr for i in result.imports])

    pgp_validity = {
        gpg.constants.VALIDITY_FULL: "full",
        gpg.constants.VALIDITY_MARGINAL: "marginal",
        gpg.constants.VALIDITY_NEVER: "never",
        gpg.constants.VALIDITY_ULTIMATE: "ultimate",
        gpg.constants.VALIDITY_UNDEFINED: "undefined",
        gpg.constants.VALIDITY_UNKNOWN: "unknown",
    }

    return {
        "format_pgp_date": lambda ts: time.strftime("%Y-%m-%d", time.localtime(ts)),
        "pgp_validity": pgp_validity.get,
        "pgp_algo": gpg.core.pubkey_algo_name,
        "ssh_keys": credentials["ssh_keys"],
        "pgp_keys": request.app.gpg.keylist("\0".join(pgp_fprs)),
        "hosting": credentials["hosting"],
    }


async def handle_ssh_keys(request):
    credentials = await get_credentials(
        request.app.http_client_session, request.app.publisher_url
    )
    return web.Response(
        text="\n".join(credentials["ssh_keys"]), content_type="text/plain"
    )


async def handle_pgp_keys(request):
    credentials = await get_credentials(
        request.app.http_client_session, request.app.publisher_url
    )
    armored = request.match_info["extension"] == ".asc"
    if armored:
        return web.Response(
            text="\n".join(credentials["pgp_keys"]),
            content_type="application/pgp-keys",
        )
    else:
        fprs = []
        for keydata in credentials["pgp_keys"]:
            result = request.app.gpg.key_import(keydata.encode("utf-8"))
            fprs.extend([i.fpr for i in result.imports])
        return web.Response(
            body=request.app.gpg.key_export_minimal("\0".join(fprs)),
            content_type="application/pgp-keys",
        )


@html_template("publish-history.html", headers={"Cache-Control": "max-age=10"})
async def handle_publish_history(request):
    limit = int(request.query.get("limit", "100"))
    from .publish import write_history

    async with request.app.database.acquire() as conn:
        return await write_history(conn, limit=limit)


@html_template("queue.html", headers={"Cache-Control": "max-age=10"})
async def handle_queue(request):
    limit = int(request.query.get("limit", "100"))
    from .queue import write_queue

    return await write_queue(
        request.app.http_client_session,
        request.app.database,
        queue_status=request.app['runner_status'],
        limit=limit,
    )


@html_template("maintainer-stats.html", headers={"Cache-Control": "max-age=60"})
async def handle_cupboard_maintainer_stats(request):
    from .stats import write_maintainer_stats

    async with request.app.database.acquire() as conn:
        return await write_maintainer_stats(conn)


@html_template("maintainer-overview.html", headers={"Cache-Control": "max-age=60"})
async def handle_maintainer_overview(request):
    from .stats import write_maintainer_overview

    async with request.app.database.acquire() as conn:
        return await write_maintainer_overview(
            conn, request.match_info["maintainer"]
        )


@html_template("never-processed.html", headers={"Cache-Control": "max-age=60"})
async def handle_never_processed(request):
    suite = request.query.get("suite")
    if suite is not None and suite.lower() == "_all":
        suite = None
    suites = [suite] if suite else None
    async with request.app.database.acquire() as conn:
        query = """\
        select c.package, c.suite from candidate c
        where not exists (
            SELECT FROM run WHERE run.package = c.package AND c.suite = suite)
        """
        args = []
        if suites:
            query += " AND suite = ANY($1::text[])"
            args.append(suites)
        return {"never_processed": await conn.fetch(query, *args)}


@html_template("result-code-index.html", headers={"Cache-Control": "max-age=60"})
async def handle_result_codes(request):
    suite = request.query.get("suite")
    if suite is not None and suite.lower() == "_all":
        suite = None
    all_suites = [s.name for s in request.app['config'].suite] + [
                  c.name for c in request.app['config'].campaign]
    async with request.app.database.acquire() as conn:
        query = """\
    (select (
            case when result_code = 'nothing-new-to-do' then 'success'
            else result_code end), count(result_code) from last_runs
        where suite = ANY($1::text[]) group by 1)
    union
    select 'never-processed', count(*) from candidate c
        where not exists (
            SELECT FROM run WHERE run.package = c.package AND c.suite = suite)
        and suite = ANY($1::text[]) order by 2 desc
    """
        return {
            "result_codes": await conn.fetch(
                query, [suite] if suite else all_suites),
            "suite": suite, "all_suites": all_suites}


@html_template("result-code.html", headers={"Cache-Control": "max-age=60"})
async def handle_result_code(request):
    suite = request.query.get("suite")
    if suite is not None and suite.lower() == "_all":
        suite = None
    code = request.match_info.get("code")
    query = ('SELECT * FROM last_runs '
             'WHERE result_code = ANY($1::text[]) AND suite = ANY($2::text[])')
    codes = [code]
    all_suites = [s.name for s in request.app['config'].suite] + [
                  c.name for c in request.app['config'].campaign]
    async with request.app.database.acquire() as conn:
        return {
            "code": code,
            "runs": await conn.fetch(query, codes, [suite] if suite else all_suites),
            "suite": suite,
            "all_suites": all_suites}


async def handle_login(request):
    state = str(uuid.uuid4())
    callback_path = request.app.router["oauth2-callback"].url_for()
    if not request.app['openid_config']:
        raise web.HTTPNotFound(text='login is disabled on this instance')
    location = URL(request.app['openid_config']["authorization_endpoint"]).with_query(
        {
            "client_id": request.app['config'].oauth2_provider.client_id or os.environ['OAUTH2_CLIENT_ID'],
            "redirect_uri": str(request.app['external_url'].join(callback_path)),
            "response_type": "code",
            "scope": "openid",
            "state": state,
        }
    )
    response = web.HTTPFound(location)
    response.set_cookie(
        "state", state, max_age=60, path=callback_path, httponly=True, secure=True
    )
    if "url" in request.query:
        try:
            response.set_cookie("back_url", str(URL(request.query["url"]).relative()))
        except ValueError:
            # 'url' is not a URL
            raise web.HTTPBadRequest(text='invalid url')
    return response


async def handle_static_file(path, request):
    return web.FileResponse(path)


@html_template("package-name-list.html", headers={"Cache-Control": "max-age=600"})
async def handle_pkg_list(request):
    # TODO(jelmer): The javascript plugin thingy should just redirect to
    # the right URL, not rely on query parameters here.
    pkg = request.query.get("package")
    if pkg:
        async with request.app.database.acquire() as conn:
            if not await conn.fetchrow('SELECT 1 FROM package WHERE name = $1', pkg):
                raise web.HTTPNotFound(text="No package with name %s" % pkg)
        return web.HTTPFound(pkg)

    async with request.app.database.acquire() as conn:
        packages = [
            row['name']
            for row in await conn.fetch(
                'SELECT name, maintainer_email FROM package WHERE NOT removed ORDER BY name')]
    return {'packages': packages}


@html_template(
    "by-maintainer-package-list.html", headers={"Cache-Control": "max-age=600"})
async def handle_maintainer_list(request):
    from .pkg import generate_maintainer_list

    async with request.app.database.acquire() as conn:
        packages = [
            (row['name'], row['maintainer_email'])
            for row in await conn.fetch(
                'SELECT name, maintainer_email FROM package WHERE NOT removed')]
    return await generate_maintainer_list(packages)


@html_template("maintainer-index.html", headers={"Cache-Control": "max-age=600"})
async def handle_maintainer_index(request):
    if request['user']:
        email = request['user'].get("email")
    else:
        email = request.query.get("email")
    if email and "/" in email:
        raise web.HTTPBadRequest(text="invalid maintainer email")
    if email:
        raise web.HTTPFound(
            request.app.router["maintainer-overview-short"].url_for(
                maintainer=email
            )
        )
    return {}


@html_template("package-overview.html", headers={"Cache-Control": "max-age=600"})
async def handle_pkg(request):
    from .pkg import generate_pkg_file

    span = aiozipkin.request_span(request)

    package_name = request.match_info["pkg"]
    async with request.app.database.acquire() as conn:
        with span.new_child('sql:package'):
            package = await conn.fetchrow(
                'SELECT name, vcswatch_status, maintainer_email, vcs_type, '
                'vcs_url, branch_url, vcs_browse, removed FROM package WHERE name = $1', package_name)
        if package is None:
            raise web.HTTPNotFound(text="No package with name %s" % package_name)
        with span.new_child('sql:merge-proposals'):
            merge_proposals = await conn.fetch("""\
SELECT DISTINCT ON (merge_proposal.url)
merge_proposal.url AS url, merge_proposal.status AS status, run.suite AS suite
FROM
merge_proposal
LEFT JOIN run
ON merge_proposal.revision = run.revision AND run.result_code = 'success'
WHERE run.package = $1
ORDER BY merge_proposal.url, run.finish_time DESC
""", package['name'])
        with span.new_child('sql:publishable-suites'):
            available_suites = await state.iter_publishable_suites(conn, package_name)
    with span.new_child('sql:runs'):
        async with request.app.database.acquire() as conn:
            runs = await conn.fetch(
                "SELECT id, finish_time, result_code, suite FROM run "
                "LEFT JOIN debian_build ON run.id = debian_build.run_id "
                "WHERE package = $1 ORDER BY finish_time DESC", package['name'])
    return await generate_pkg_file(
        request.app.database, request.app['config'], package, merge_proposals, runs,
        available_suites, span
    )


@html_template("vcs-regressions.html", headers={"Cache-Control": "max-age=600"})
async def handle_vcs_regressions(request):
    async with request.app.database.acquire() as conn:
        query = """\
select
package.name,
run.suite,
run.id,
run.result_code,
package.vcswatch_status
from
last_runs run left join package on run.package = package.name
where
result_code in (
'branch-missing',
'branch-unavailable',
'401-unauthorized',
'hosted-on-alioth',
'missing-control-file'
)
and
vcswatch_status in ('old', 'new', 'commits', 'ok')
"""
        return {"regressions": await conn.fetch(query)}


@html_template(
    "broken-merge-proposals.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_broken_mps(request):
    async with request.app.database.acquire() as conn:
        broken_mps = await conn.fetch(
            """\
select
url,
last_run.suite,
last_run.package,
last_run.id,
last_run.result_code,
last_run.finish_time,
last_run.description
from
(select
 distinct on (url) url, run.suite, run.package, run.finish_time,
 merge_proposal.revision as current_revision
from merge_proposal join run on
 merge_proposal.revision = run.revision where status = 'open')
as current_run left join last_runs last_run
on
current_run.suite = last_run.suite and
current_run.package = last_run.package
where
last_run.result_code not in ('success', 'nothing-to-do', 'nothing-new-to-do')
order by url, last_run.finish_time desc
"""
        )

    return {"broken_mps": broken_mps}


@html_template("run.html", headers={"Cache-Control": "max-age=3600"})
async def handle_run(request):
    from .common import get_run
    from .pkg import generate_run_file

    span = aiozipkin.request_span(request)
    run_id = request.match_info["run_id"]
    pkg = request.match_info.get("pkg")
    async with request.app.database.acquire() as conn:
        with span.new_child('sql:run'):
            run = await get_run(conn, run_id)
            if run is None:
                raise web.HTTPNotFound(text="No run with id %r" % run_id)
    if pkg is not None and pkg != run['package']:
        if run is None:
            raise web.HTTPNotFound(text="No run with id %r" % run_id)
    return await generate_run_file(
        request.app.database,
        request.app.http_client_session,
        request.app['config'],
        request.app.differ_url,
        request.app.logfile_manager,
        run,
        request.app['vcs_manager'],
        is_admin=is_admin(request),
        span=span,
    )


async def handle_result_file(request):
    pkg = request.match_info["pkg"]
    filename = request.match_info["filename"]
    run_id = request.match_info["run_id"]
    if not re.match("^[a-z0-9+-\\.]+$", pkg) or len(pkg) < 2:
        raise web.HTTPNotFound(text="Invalid package %s for run %s" % (pkg, run_id))
    if not re.match("^[a-z0-9-]+$", run_id) or len(run_id) < 5:
        raise web.HTTPNotFound(text="Invalid run run id %s" % (run_id,))
    if filename.endswith(".log") or re.match(r".*\.log\.[0-9]+", filename):
        if not re.match("^[+a-z0-9\\.]+$", filename) or len(filename) < 3:
            raise web.HTTPNotFound(
                text="No log file %s for run %s" % (filename, run_id)
            )

        try:
            logfile = await request.app.logfile_manager.get_log(pkg, run_id, filename)
        except FileNotFoundError:
            raise web.HTTPNotFound(
                text="No log file %s for run %s" % (filename, run_id)
            )
        else:
            with logfile as f:
                text = f.read().decode("utf-8", "replace")
        return web.Response(
            content_type="text/plain",
            text=text,
            headers={"Cache-Control": "max-age=3600"},
        )
    else:
        try:
            artifact = await request.app['artifact_manager'].get_artifact(
                run_id, filename
            )
        except FileNotFoundError:
            raise web.HTTPNotFound(text="No artifact %s for run %s" % (filename, run_id))
        with artifact as f:
            return web.Response(
                body=f.read(), headers={"Cache-Control": "max-age=3600"}
            )


@html_template("ready-list.html", headers={"Cache-Control": "max-age=60"})
async def handle_ready_proposals(request):
    from .pkg import generate_ready_list

    suite = request.match_info.get("suite")
    review_status = request.query.get("review_status")
    return await generate_ready_list(request.app.database, suite, review_status)


@html_template("generic-package.html", headers={"Cache-Control": "max-age=600"})
async def handle_generic_pkg(request):
    from .common import generate_pkg_context

    # TODO(jelmer): Handle Accept: text/diff
    pkg = request.match_info["pkg"]
    run_id = request.match_info.get("run_id")
    return await generate_pkg_context(
        request.app.database,
        request.app['config'],
        request.match_info["suite"],
        request.app.policy,
        request.app.http_client_session,
        request.app.differ_url,
        request.app['vcs_manager'],
        pkg,
        aiozipkin.request_span(request),
        run_id,
    )


@html_template("rejected.html")
async def handle_rejected(request):
    from .review import generate_rejected

    suite = request.query.get("suite")
    async with request.app.database.acquire() as conn:
        return await generate_rejected(conn, request.app['config'], suite=suite)


async def handle_review_post(request):
    from .review import generate_review, store_review
    publishable_only = request.query.get("publishable_only", "true") == "true"
    check_qa_reviewer(request)

    post = await request.post()
    async with request.app.database.acquire() as conn:
        run = await conn.fetchrow(
            'SELECT package, suite FROM run WHERE id = $1',
            post["run_id"])
        review_status = post["review_status"].lower()
        if review_status == "reschedule":
            review_status = "rejected"
            from ..schedule import do_schedule

            await do_schedule(
                conn,
                run['package'],
                run['suite'],
                refresh=True,
                requestor="reviewer",
                bucket="default",
            )
        review_comment = post.get("review_comment")
        await store_review(conn, post["run_id"], review_comment, review_status, request['user'])
        text = await generate_review(
            conn,
            request,
            request.app.http_client_session,
            request.app.differ_url,
            request.app['vcs_manager'],
            suites=post.getall("suite", None),
            publishable_only=publishable_only,
        )
        return web.Response(
            content_type="text/html",
            text=text,
            headers={"Cache-Control": "no-cache"},
        )


async def handle_review(request):
    from .review import generate_review
    publishable_only = request.query.get("publishable_only", "true") == "true"

    suites = request.query.getall("suite", None)
    async with request.app.database.acquire() as conn:
        text = await generate_review(
            conn,
            request,
            request.app.http_client_session,
            request.app.differ_url,
            request.app['vcs_manager'],
            suites=suites,
            publishable_only=publishable_only,
        )
    return web.Response(
        content_type="text/html", text=text, headers={"Cache-Control": "no-cache"}
    )


@html_template("repo-list.html")
async def handle_repo_list(request):
    vcs = request.match_info["vcs"]
    url = request.app['vcs_manager'].base_urls[vcs]
    async with request.app.http_client_session.get(url) as resp:
        return {"vcs": vcs, "repositories": await resp.json()}


async def handle_oauth_callback(request):
    code = request.query.get("code")
    state_code = request.query.get("state")
    if request.cookies.get("state") != state_code:
        return web.Response(status=400, text="state variable mismatch")
    if not request.app['openid_config']:
        raise web.HTTPNotFound(text='login disabled')
    token_url = URL(request.app['openid_config']["token_endpoint"])
    redirect_uri = (request.app['external_url'] or request.url).join(
        request.app.router["oauth2-callback"].url_for()
    )
    params = {
        "code": code,
        "client_id": request.app['config'].oauth2_provider.client_id or os.environ['OAUTH2_CLIENT_ID'],
        "client_secret": request.app['config'].oauth2_provider.client_secret or os.environ['OAUTH2_CLIENT_SECRET'],
        "grant_type": "authorization_code",
        "redirect_uri": str(redirect_uri),
    }
    async with request.app.http_client_session.post(
        token_url, params=params
    ) as resp:
        if resp.status != 200:
            return web.json_response(
                status=resp.status, data={
                    "error": "token-error",
                    "message": "received response %d" % resp.status,
                    })
        resp = await resp.json()
        if resp["token_type"] != "Bearer":
            return web.Response(
                status=500,
                text="Expected bearer token, got %s" % resp["token_type"],
            )
        refresh_token = resp["refresh_token"]  # noqa: F841
        access_token = resp["access_token"]

    try:
        back_url = request.cookies["back_url"]
    except KeyError:
        back_url = "/"

    async with request.app.http_client_session.get(
        request.app['openid_config']["userinfo_endpoint"],
        headers={"Authorization": "Bearer %s" % access_token},
    ) as resp:
        if resp.status != 200:
            raise Exception(
                "unable to get user info (%s): %s"
                % (resp.status, await resp.read())
            )
        userinfo = await resp.json()
    session_id = str(uuid.uuid4())
    async with request.app.database.acquire() as conn:
        await conn.execute("""
INSERT INTO site_session (id, userinfo) VALUES ($1, $2)
ON CONFLICT (id) DO UPDATE SET userinfo = EXCLUDED.userinfo
""", session_id, userinfo)

    # TODO(jelmer): Store access token / refresh token?

    resp = web.HTTPFound(back_url)

    resp.del_cookie("state")
    resp.del_cookie("back_url")
    resp.set_cookie("session_id", session_id, secure=True, httponly=True)
    return resp


async def handle_health(request):
    return web.Response(text='ok')


async def create_app(
        config, policy_config, minified=False,
        external_url=None, debugtoolbar=None,
        runner_url=None, publisher_url=None,
        archiver_url=None, vcs_manager=None,
        differ_url=None,
        listen_address=None, port=None):
    if minified:
        minified_prefix = ""
    else:
        minified_prefix = "min."

    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[trailing_slash_redirect])
    private_app = web.Application(middlewares=[trailing_slash_redirect])

    app.middlewares.insert(0, metrics_middleware)
    private_app.middlewares.insert(0, metrics_middleware)
    private_app.router.add_get("/metrics", metrics, name="metrics")
    private_app.router.add_get("/health", handle_health, name="health")

    app.topic_notifications = Topic("notifications")
    app.router.add_get(
        "/ws/notifications",
        functools.partial(pubsub_handler, app.topic_notifications),  # type: ignore
        name="ws-notifications",
    )

    endpoint = aiozipkin.create_endpoint("janitor.site", ipv4=listen_address, port=port)
    if config.zipkin_address:
        tracer = await aiozipkin.create(config.zipkin_address, endpoint, sample_rate=1.0)
    else:
        tracer = await aiozipkin.create_custom(endpoint)
    trace_configs = [aiozipkin.make_trace_config(tracer)]

    aiozipkin.setup(private_app, tracer, skip_routes=[
        private_app.router['metrics'],
        ])
    aiozipkin.setup(app, tracer, skip_routes=[
        app.router['ws-notifications'],
        ])

    async def setup_client_session(app):
        app.http_client_session = ClientSession(trace_configs=trace_configs)

    async def close_client_session(app):
        await app.http_client_session.close()

    app.on_startup.append(setup_client_session)
    app.on_cleanup.append(close_client_session)

    async def start_gpg_context(app):
        gpg_home = tempfile.TemporaryDirectory()
        gpg_context = gpg.Context(home_dir=gpg_home.name)
        app.gpg = gpg_context.__enter__()

        async def cleanup_gpg(app):
            gpg_context.__exit__(None, None, None)
            shutil.rmtree(gpg_home)

        app.on_cleanup.append(cleanup_gpg)

    async def discover_openid_config(app):
        url = URL(app['config'].oauth2_provider.base_url).join(
            URL("/.well-known/openid-configuration")
        )
        async with app.http_client_session.get(url) as resp:
            if resp.status != 200:
                # TODO(jelmer): Fail? Set flag?
                logging.warning(
                    "Unable to find openid configuration (%s): %s",
                    resp.status,
                    await resp.read(),
                )
                return
            app['openid_config'] = await resp.json()

    async def start_pubsub_forwarder(app):
        async def listen_to_publisher_publish(app):
            url = URL(app.publisher_url) / "ws/publish"
            async for msg in pubsub_reader(app.http_client_session, url):
                app.topic_notifications.publish(["publish", msg])

        async def listen_to_publisher_mp(app):
            url = URL(app.publisher_url) / "ws/merge-proposal"
            async for msg in pubsub_reader(app.http_client_session, url):
                app.topic_notifications.publish(["merge-proposal", msg])

        app['runner_status'] = None

        async def listen_to_queue(app):
            url = URL(app.runner_url) / "ws/queue"
            async for msg in pubsub_reader(app.http_client_session, url):
                app['runner_status'] = msg
                app.topic_notifications.publish(["queue", msg])

        async def listen_to_result(app):
            url = URL(app.runner_url) / "ws/result"
            async for msg in pubsub_reader(app.http_client_session, url):
                app.topic_notifications.publish(["result", msg])

        for cb, title in [
            (listen_to_publisher_publish, 'publisher publish listening'),
            (listen_to_publisher_mp, 'merge proposal listening'),
            (listen_to_queue, 'queue listening'),
            (listen_to_result, 'result listening'),
        ]:
            listener = create_background_task(cb(app), title)

            async def stop_listener(app):
                listener.cancel()
                await listener

            app.on_cleanup.append(stop_listener)

    for path, templatename in [
        ("/", "index"),
        ("/contact", "contact"),
        ("/about", "about"),
        ("/apt", "apt"),
        ("/cupboard/", "cupboard"),
    ]:
        app.router.add_get(
            path,
            functools.partial(handle_simple, templatename + ".html"),
            name=templatename,
        )
    app.router.add_get("/credentials", handle_credentials, name="credentials")
    app.router.add_get("/ssh_keys", handle_ssh_keys, name="ssh-keys")
    app.router.add_get(
        r"/pgp_keys{extension:(\.asc)?}", handle_pgp_keys, name="pgp-keys"
    )
    from .lintian_fixes import register_lintian_fixes_endpoints
    register_lintian_fixes_endpoints(app.router)
    from .multiarch_hints import register_multiarch_hints_endpoints
    register_multiarch_hints_endpoints(app.router)
    from .orphan import register_orphan_endpoints
    register_orphan_endpoints(app.router)
    from .debianize import register_debianize_endpoints
    register_debianize_endpoints(app.router)
    from .scrub_obsolete import register_scrub_obsolete_endpoints
    register_scrub_obsolete_endpoints(app.router)
    from .new_upstream import register_new_upstream_endpoints
    register_new_upstream_endpoints(app.router)
    SUITE_REGEX = "|".join([re.escape(suite.name) for suite in config.suite] + [re.escape(campaign.name) for campaign in config.campaign])
    app.router.add_get(
        "/{suite:%s}/merge-proposals" % SUITE_REGEX,
        handle_merge_proposals,
        name="suite-merge-proposals",
    )
    app.router.add_get(
        "/{suite:%s}/ready" % SUITE_REGEX, handle_ready_proposals, name="suite-ready"
    )
    app.router.add_get(
        "/{suite:%s}/maintainer" % SUITE_REGEX,
        handle_maintainer_list,
        name="suite-maintainer-list",
    )
    app.router.add_get(
        "/{suite:%s}/pkg/" % SUITE_REGEX, handle_pkg_list, name="suite-package-list"
    )
    app.router.add_get(
        "/{vcs:git|bzr}/", handle_repo_list, name="repo-list")
    app.router.add_get("/{suite:unchanged}", handle_apt_repo, name="unchanged-start")
    app.router.add_get("/cupboard/history", handle_history, name="history")
    app.router.add_get("/cupboard/queue", handle_queue, name="queue")
    app.router.add_get(
        "/cupboard/result-codes/", handle_result_codes, name="result-code-list"
    )
    app.router.add_get(
        "/cupboard/result-codes/{code}", handle_result_code, name="result-code"
    )
    app.router.add_get(
        "/cupboard/never-processed", handle_never_processed, name="never-processed"
    )
    app.router.add_get(
        "/cupboard/maintainer-stats",
        handle_cupboard_maintainer_stats,
        name="cupboard-maintainer-stats",
    )
    app.router.add_get(
        "/cupboard/maintainer", handle_maintainer_list, name="maintainer-list"
    )
    app.router.add_get(
        "/cupboard/maintainer/{maintainer}",
        handle_maintainer_overview,
        name="cupboard-maintainer-overview",
    )
    app.router.add_get(
        "/maintainer/{maintainer}",
        handle_maintainer_overview,
        name="maintainer-overview",
    )
    app.router.add_get("/m/", handle_maintainer_index, name="maintainer-index-short")
    app.router.add_get(
        "/m/{maintainer}", handle_maintainer_overview, name="maintainer-overview-short"
    )
    app.router.add_get(
        "/cupboard/publish", handle_publish_history, name="publish-history"
    )
    app.router.add_get("/cupboard/ready", handle_ready_proposals, name="cupboard-ready")
    app.router.add_get("/cupboard/pkg/", handle_pkg_list, name="package-list")
    app.router.add_get("/cupboard/pkg/{pkg}/", handle_pkg, name="cupboard-package")
    app.router.add_get("/cupboard/pkg/{pkg}/{run_id}/", handle_run, name="cupboard-run")
    app.router.add_get("/cupboard/review", handle_review, name="cupboard-review")
    app.router.add_get("/cupboard/rejected", handle_rejected, name="cupboard-rejected")
    app.router.add_post(
        "/cupboard/review", handle_review_post, name="cupboard-review-post"
    )
    app.router.add_get(
        "/cupboard/pkg/{pkg}/{run_id}/{filename:.+}",
        handle_result_file,
        name="cupboard-result-file",
    )
    app.router.add_get(
        "/{suite:" + SUITE_REGEX + "}/pkg/{pkg}/{run_id}/{filename:.+}",
        handle_result_file,
        name="result-file",
    )
    app.router.add_get(
        "/{suite:" + SUITE_REGEX + "}/", handle_generic_start, name="generic-start"
    )
    app.router.add_get(
        "/{suite:" + SUITE_REGEX + "}/candidates",
        handle_generic_candidates,
        name="generic-candidates",
    )
    app.router.add_get(
        "/{suite:" + SUITE_REGEX + "}/pkg/{pkg}/",
        handle_generic_pkg,
        name="generic-package",
    )
    app.router.add_get(
        "/{suite:" + SUITE_REGEX + "}/pkg/{pkg}/{run_id}",
        handle_generic_pkg,
        name="generic-run",
    )
    app.router.add_get(
        "/cupboard/vcs-regressions/", handle_vcs_regressions, name="vcs-regressions"
    )
    app.router.add_get(
        "/cupboard/broken-merge-proposals", handle_broken_mps, name="broken-mps"
    )
    app.router.add_get("/login", handle_login, name="login")
    for entry in os.scandir(os.path.join(os.path.dirname(__file__), "_static")):
        app.router.add_get(
            "/_static/%s" % entry.name,
            functools.partial(handle_static_file, entry.path),
        )
    app.router.add_static(
        "/_static/images/datatables", "/usr/share/javascript/jquery-datatables/images"
    )
    for (name, kind, basepath) in [
        ("chart", "js", "/usr/share/javascript/chart.js/Chart"),
        ("chart", "css", "/usr/share/javascript/chart.js/Chart"),
        ("jquery", "js", "/usr/share/javascript/jquery/jquery"),
        (
            "jquery.typeahead",
            "js",
            "/usr/share/javascript/jquery-typeahead/jquery.typeahead",
        ),
        (
            "jquery.datatables",
            "js",
            "/usr/share/javascript/jquery-datatables/jquery.dataTables",
        ),
        ("moment", "js", "/usr/share/javascript/moment/moment"),
    ]:
        if not os.path.exists(basepath + "." + kind):
            continue
        app.router.add_get(
            "/_static/%s.%s" % (name, kind),
            functools.partial(
                handle_static_file, "%s.%s%s" % (basepath, minified_prefix, kind)
            ),
        )
    app.router.add_get("/oauth/callback", handle_oauth_callback, name="oauth2-callback")

    from .api import create_app as create_api_app
    from .webhook import process_webhook, is_webhook_request

    async def handle_post_root(request):
        if is_webhook_request(request):
            return await process_webhook(request, request.app.database)
        raise web.HTTPMethodNotAllowed(method='POST', allowed_methods=['GET', 'HEAD'])

    app.runner_url = runner_url
    app.archiver_url = archiver_url
    app.differ_url = differ_url
    app.policy = policy_config
    app.publisher_url = publisher_url
    app['vcs_manager'] = vcs_manager
    app['openid_config'] = None
    if config.oauth2_provider and config.oauth2_provider.base_url:
        app.on_startup.append(discover_openid_config)
    app.on_startup.append(start_pubsub_forwarder)
    app.on_startup.append(start_gpg_context)
    if external_url:
        app['external_url'] = URL(external_url)
    else:
        app['external_url'] = None
    database = state.Database(config.database_location)
    app.database = database
    from .stats import stats_app

    app.add_subapp("/cupboard/stats", stats_app(database, config, app['external_url']))
    app['config'] = config
    from janitor.site import env

    app['jinja_env'] = env
    from janitor.artifacts import get_artifact_manager

    async def startup_artifact_manager(app):
        app['artifact_manager'] = get_artifact_manager(
            config.artifact_location, trace_configs=trace_configs)
        await app['artifact_manager'].__aenter__()

    async def turndown_artifact_manager(app):
        await app['artifact_manager'].__aexit__(None, None, None)

    app.on_startup.append(startup_artifact_manager)
    app.on_cleanup.append(turndown_artifact_manager)
    setup_debsso(app)
    app.router.add_post("/", handle_post_root, name="root-post")
    app.add_subapp(
        "/api",
        create_api_app(
            app.database,
            publisher_url,
            runner_url,  # type: ignore
            vcs_manager,
            differ_url,
            config,
            policy_config,
            external_url=(
                app['external_url'].join(URL("api")) if app['external_url'] else None
            ),
            trace_configs=trace_configs,
        ),
    )
    import aiohttp_apispec
    app.router.add_static('/static/swagger', os.path.join(os.path.dirname(aiohttp_apispec.__file__), "static"))

    if debugtoolbar:
        import aiohttp_debugtoolbar
        # install aiohttp_debugtoolbar
        aiohttp_debugtoolbar.setup(app, hosts=debugtoolbar)

    async def setup_logfile_manager(app):
        app.logfile_manager = get_log_manager(config.logs_location, trace_configs=trace_configs)

    app.on_startup.append(setup_logfile_manager)
    return private_app, app


async def main(argv=None):
    import argparse
    import os
    from janitor.config import read_config
    from janitor.policy import read_policy

    parser = argparse.ArgumentParser()
    parser.add_argument("--debugtoolbar", type=str, action="append", help="IP to allow debugtoolbar queries from.")
    parser.add_argument("--host", type=str, help="Host to listen on")
    parser.add_argument("--port", type=int, help="Port to listen on", default=8080)
    parser.add_argument(
        "--public-port", type=int, help="Public port to listen on", default=8090)
    parser.add_argument(
        "--publisher-url",
        type=str,
        default="http://localhost:9912/",
        help="URL for publisher.",
    )
    parser.add_argument(
        "--vcs-store-url",
        type=str,
        default="http://localhost:9921/",
        help="URL for VCS store.",
    )
    parser.add_argument(
        "--runner-url",
        type=str,
        default="http://localhost:9911/",
        help="URL for runner.",
    )
    parser.add_argument(
        "--archiver-url",
        type=str,
        default="http://localhost:9914/",
        help="URL for runner.",
    )
    parser.add_argument(
        "--differ-url",
        type=str,
        default="http://localhost:9920/",
        help="URL for differ.",
    )
    parser.add_argument(
        "--policy",
        help="Policy file to read.",
        type=str,
        default=os.path.join(os.path.dirname(__file__), "..", "..", "policy.conf"),
    )
    parser.add_argument(
        "--config", type=str, default="janitor.conf", help="Path to configuration."
    )
    parser.add_argument(
        "--debug",
        action="store_true",
        help="Enable debugging mode. For example, avoid minified JS.",
    )
    parser.add_argument("--gcp-logging", action='store_true', help='Use Google cloud logging.')
    parser.add_argument("--external-url", type=str, default=None, help="External URL")

    args = parser.parse_args()

    if args.gcp_logging:
        import google.cloud.logging
        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    else:
        logging.basicConfig(level=logging.INFO)

    with open(args.config, "r") as f:
        config = read_config(f)

    with open(args.policy, "r") as f:
        policy_config = read_policy(f)

    private_app, public_app = await create_app(
        config, policy_config, minified=args.debug,
        external_url=args.external_url,
        debugtoolbar=args.debugtoolbar,
        runner_url=args.runner_url,
        archiver_url=args.archiver_url,
        publisher_url=args.publisher_url,
        vcs_manager=get_vcs_manager(args.vcs_store_url),
        differ_url=args.differ_url,
        listen_address=args.host,
        port=args.port)

    private_runner = web.AppRunner(private_app)
    public_runner = web.AppRunner(public_app)
    await private_runner.setup()
    await public_runner.setup()
    site = web.TCPSite(private_runner, args.host, port=args.port)
    await site.start()
    logging.info("Listening on %s:%s", args.host, args.port)
    site = web.TCPSite(public_runner, args.host, port=args.public_port)
    await site.start()
    logging.info("Listening on %s:%s", args.host, args.public_port)
    while True:
        await asyncio.sleep(3600)


if __name__ == "__main__":
    import sys

    sys.exit(asyncio.run(main(sys.argv)))
