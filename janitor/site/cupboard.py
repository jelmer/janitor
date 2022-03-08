#!/usr/bin/python
# Copyright (C) 2019-2022 Jelmer Vernooij <jelmer@jelmer.uk>
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

"""Serve the janitor cupboard site."""


import aiozipkin

from aiohttp import web
from aiohttp.web_middlewares import normalize_path_middleware

from .. import state

from . import is_admin, env
from .common import html_template


@html_template(env, "cupboard/rejected.html")
async def handle_rejected(request):
    from .review import generate_rejected

    campaign = request.query.get("campaign")
    async with request.app.database.acquire() as conn:
        return await generate_rejected(conn, request.app['config'], campaign=campaign)


@html_template(env, "cupboard/history.html", headers={"Cache-Control": "max-age=10", "Vary": "Cookie"})
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


@html_template(env, "cupboard/reprocess-logs.html")
async def handle_reprocess_logs(request):
    return {}


@html_template(env, "cupboard/queue.html", headers={"Cache-Control": "max-age=10", "Vary": "Cookie"})
async def handle_queue(request):
    limit = int(request.query.get("limit", "100"))
    from .queue import write_queue

    return await write_queue(
        request.app.http_client_session,
        request.app.database,
        queue_status=request.app['runner_status'],
        limit=limit,
    )


@html_template(env, "cupboard/never-processed.html", headers={"Cache-Control": "max-age=60", "Vary": "Cookie"})
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


@html_template(env, "cupboard/result-code-index.html", headers={"Cache-Control": "max-age=60", "Vary": "Cookie"})
async def handle_result_codes(request):
    from ..schedule import TRANSIENT_ERROR_RESULT_CODES
    suite = request.query.get("suite")
    exclude_never_processed = "exclude_never_processed" in request.query
    exclude_transient = "exclude_transient" in request.query
    if suite is not None and suite.lower() == "_all":
        suite = None
    all_suites = [c.name for c in request.app['config'].campaign]
    args = [[suite] if suite else all_suites]
    async with request.app.database.acquire() as conn:
        query = """\
    select (
            case when result_code = 'nothing-new-to-do' then 'success'
            else result_code end), count(result_code) from last_runs
        where suite = ANY($1::text[])
    """
        if exclude_transient:
            query += " AND result_code != ALL($2::text[])"
            args.append(TRANSIENT_ERROR_RESULT_CODES)
        query += " group by 1"
        if not exclude_never_processed:
            query = """(%s) union
    select 'never-processed', count(*) from candidate c
        where not exists (
            SELECT FROM run WHERE run.package = c.package AND c.suite = suite)
        and suite = ANY($1::text[]) order by 2 desc
    """ % query
        return {
            "exclude_never_processed": exclude_never_processed,
            "exclude_transient": exclude_transient,
            "result_codes": await conn.fetch(query, *args),
            "suite": suite, "all_suites": all_suites}


@html_template(env, "cupboard/result-code.html", headers={"Cache-Control": "max-age=60", "Vary": "Cookie"})
async def handle_result_code(request):
    suite = request.query.get("suite")
    if suite is not None and suite.lower() == "_all":
        suite = None
    code = request.match_info.get("code")
    query = ('SELECT * FROM last_runs '
             'WHERE result_code = ANY($1::text[]) AND suite = ANY($2::text[])')
    codes = [code]
    all_suites = [c.name for c in request.app['config'].campaign]
    async with request.app.database.acquire() as conn:
        return {
            "code": code,
            "runs": await conn.fetch(query, codes, [suite] if suite else all_suites),
            "suite": suite,
            "all_suites": all_suites}


@html_template(env, "cupboard/publish.html")
async def handle_publish(request):
    id = request.match_info["id"]
    from .publish import write_publish
    async with request.app.database.acquire() as conn:
        return await write_publish(conn, id)


@html_template(env, "cupboard/publish-history.html", headers={"Cache-Control": "max-age=10", "Vary": "Cookie"})
async def handle_publish_history(request):
    limit = int(request.query.get("limit", "100"))
    from .publish import write_history

    async with request.app.database.acquire() as conn:
        return await write_history(conn, limit=limit)


async def handle_review_post(request):
    from .review import generate_review, store_review
    check_qa_reviewer(request)

    post = await request.post()
    publishable_only = post.get("publishable_only", "true") == "true"
    async with request.app.database.acquire() as conn:
        if "review_status" in post:
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


@html_template(env, "cupboard/run.html", headers={"Cache-Control": "max-age=3600", "Vary": "Cookie"})
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


@html_template(
    env, "cupboard/broken-merge-proposals.html", headers={"Cache-Control": "max-age=600", "Vary": "Cookie"}
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


@html_template(env, "cupboard/start.html")
async def handle_cupboard_start(request):
    return {'extra_cupboard_links': _extra_cupboard_links}


@html_template(env, "cupboard/package-overview.html", headers={"Cache-Control": "max-age=600", "Vary": "Cookie"})
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


_extra_cupboard_links = []


def register_cupboard_link(title, shortlink):
    _extra_cupboard_links.append((title, shortlink))


def register_cupboard_endpoints(router):
    router.add_get("/cupboard/", handle_cupboard_start, name="cupboard-start")
    router.add_get("/cupboard/rejected", handle_rejected, name="cupboard-rejected")
    router.add_get("/cupboard/history", handle_history, name="history")
    router.add_get("/cupboard/reprocess-logs", handle_reprocess_logs, name="reprocess-logs")
    router.add_get("/cupboard/queue", handle_queue, name="queue")
    router.add_get(
        "/cupboard/never-processed", handle_never_processed, name="never-processed"
    )
    router.add_get(
        "/cupboard/result-codes/", handle_result_codes, name="result-code-list"
    )
    router.add_get(
        "/cupboard/result-codes/{code}", handle_result_code, name="result-code"
    )
    router.add_get(
        "/cupboard/publish/", handle_publish_history, name="publish-history"
    )
    router.add_get(
        "/cupboard/publish/{id}", handle_publish, name="publish"
    )
    router.add_get("/cupboard/review", handle_review, name="cupboard-review")
    router.add_post(
        "/cupboard/review", handle_review_post, name="cupboard-review-post"
    )
    router.add_get("/cupboard/pkg/{pkg}/", handle_pkg, name="cupboard-package")
    router.add_get("/cupboard/pkg/{pkg}/{run_id}/", handle_run, name="cupboard-run")
    router.add_get(
        "/cupboard/broken-merge-proposals", handle_broken_mps, name="broken-mps"
    )

