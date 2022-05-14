#!/usr/bin/python3
# Copyright (C) 2019-2021 Jelmer Vernooij <jelmer@jelmer.uk>
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

import aiohttp
from aiohttp import (
    web,
    ClientResponseError,
    ClientSession,
    ClientTimeout,
    ContentTypeError,
    ClientConnectorError,
    ClientOSError,
    ServerDisconnectedError,
)
import aiozipkin
import asyncio
from datetime import datetime, timedelta
import logging
from typing import Optional
import urllib.parse

from aiohttp.web_middlewares import normalize_path_middleware
import asyncpg
from aiohttp_apispec import (
    docs,
    response_schema,
    setup_aiohttp_apispec,
    )

from marshmallow import Schema, fields
from yarl import URL

from janitor import SUITE_REGEX
from janitor.config import Config
from janitor.queue import get_queue_position
from . import (
    check_admin,
    check_qa_reviewer,
    check_worker_creds,
    env,
    highlight_diff,
    get_archive_diff,
    get_vcs_diff,
    iter_accept,
    BuildDiffUnavailable,
    DebdiffRetrievalError,
)
from .common import (
    render_template_for_request,
    )
from janitor.logs import get_log_manager
from .webhook import process_webhook
from ..schedule import (
    do_schedule,
    do_schedule_control,
    PolicyUnavailable,
)
from ..vcs import VcsManager

routes = web.RouteTableDef()


class PublishPolicySchema(Schema):

    mode = fields.Str(description="publish mode")


class PolicySchema(Schema):

    publish_policy = fields.Dict(keys=fields.Str(), values=fields.Nested(PublishPolicySchema))
    command = fields.Str(description='command to run')


@docs(
    responses={
        404: {"description": "Package does not exist or does not have a policy"},
        200: {"description": "Success response"}
    }
)
@response_schema(PolicySchema())
@routes.get("/pkg/{package}/policy", name="package-policy")
async def handle_policy(request):
    package = request.match_info["package"]
    suite_policies = {}
    async with request.app['db'].acquire() as conn:
        rows = await conn.fetch(
            "SELECT suite, publish, command "
            "FROM policy WHERE package = $1", package)
    if not rows:
        return web.json_response({"reason": "Package not found"}, status=404)
    for row in rows:
        suite_policies[row['suite']] = {
            "publish_policy": {p['role']: {'mode': p['mode']} for p in row['publish']},
            "command": row['command'],
        }
    return web.json_response({"by_suite": suite_policies})


@docs()
@routes.post("/{suite}/pkg/{package}/publish", name="package-publish")
async def handle_publish(request):
    publisher_url = request.app['publisher_url']
    package = request.match_info["package"]
    suite = request.match_info["suite"]
    post = await request.post()
    mode = post.get("mode")
    if mode not in (None, "push-derived", "push", "propose", "attempt-push"):
        return web.json_response({"error": "Invalid mode", "mode": mode}, status=400)
    url = urllib.parse.urljoin(publisher_url, "%s/%s/publish" % (suite, package))
    if request['user']:
        try:
            requestor = request['user']["email"]
        except KeyError:
            requestor = request['user']["name"]
    else:
        requestor = "user from web UI"
    data = {"requestor": requestor}
    if mode:
        data["mode"] = mode
    try:
        async with request.app['http_client_session'].post(url, data=data) as resp:
            if resp.status in (200, 202):
                return web.json_response(await resp.json(), status=resp.status)
            else:
                return web.json_response(await resp.json(), status=400)
    except ContentTypeError as e:
        return web.json_response(
            {"reason": "publisher returned error %s" % e}, status=400
        )
    except ClientConnectorError:
        return web.json_response({"reason": "unable to contact publisher"}, status=400)


@routes.post("/webhook", name="webhook")
@routes.get("/webhook", name="webhook-help")
async def handle_webhook(request):
    if request.headers.get("Content-Type") != "application/json":
        text = await render_template_for_request(env, "webhook.html", request, {})
        return web.Response(
            content_type="text/html",
            text=text,
            headers={"Cache-Control": "max-age=600"},
        )
    return await process_webhook(request, request.app['db'])


class ScheduleResultSchema(Schema):

    package = fields.Str(description="package name")
    suite = fields.Str(description="suite")
    offset = fields.Int(description="offset from top of queue")
    estimated_duration_seconds = fields.Int(description="estimated duration in seconds")
    queue_position = fields.Int(description="new position in the queue")
    queue_wait_time = fields.Int(description="new delay until run, in seconds")


@response_schema(ScheduleResultSchema())
@routes.post(
    "/{suite:" + SUITE_REGEX + "}/pkg/{package}/schedule", name="package-schedule")
async def handle_schedule(request):
    package = request.match_info["package"]
    suite = request.match_info["suite"]
    post = await request.post()
    offset = post.get("offset")
    try:
        refresh = bool(int(post.get("refresh", "0")))
    except ValueError:
        return web.json_response({"error": "invalid boolean for refresh"}, status=400)
    async with request.app['db'].acquire() as conn:
        package = await conn.fetchrow(
            'SELECT name, branch_url FROM package WHERE name = $1', package)
        if package is None:
            return web.json_response({"reason": "Package not found"}, status=404)
        if request['user']:
            try:
                requestor = request['user']["email"]
            except KeyError:
                requestor = request['user']["name"]
        else:
            requestor = "user from web UI"
        if package['branch_url'] is None:
            return web.json_response({"reason": "No branch URL defined."}, status=400)
        try:
            offset, estimated_duration = await do_schedule(
                conn,
                package['name'],
                suite,
                offset,
                refresh=refresh,
                requestor=requestor,
                bucket="manual",
            )
        except PolicyUnavailable:
            return web.json_response(
                {"reason": "Publish policy not yet available."}, status=503
            )
        (queue_position, queue_wait_time) = await get_queue_position(
            conn, suite, package['name']
        )
    response_obj = {
        "package": package['name'],
        "suite": suite,
        "offset": offset,
        "estimated_duration_seconds": estimated_duration.total_seconds(),
        "queue_position": queue_position,
        "queue_wait_time": queue_wait_time.total_seconds(),
    }
    return web.json_response(response_obj)


@response_schema(ScheduleResultSchema())
@routes.post("/run/{run_id}/reschedule", name="run-reschedule")
async def handle_run_reschedule(request):
    run_id = request.match_info["run_id"]
    post = await request.post()
    offset = post.get("offset")
    try:
        refresh = bool(int(post.get("refresh", "0")))
    except ValueError:
        return web.json_response({"error": "invalid boolean for refresh"}, status=400)
    if request['user']:
        try:
            requestor = request['user']["email"]
        except KeyError:
            requestor = request['user']["name"]
    else:
        requestor = "user from web UI"
    async with request.app['db'].acquire() as conn:
        run = await conn.fetchrow(
            "SELECT suite, package FROM run WHERE id = $1",
            run_id)
        if run is None:
            return web.json_response({"reason": "Run not found"}, status=404)
        try:
            offset, estimated_duration = await do_schedule(
                conn,
                run['package'],
                run['suite'],
                offset,
                refresh=refresh,
                requestor=requestor,
                bucket="manual",
            )
        except PolicyUnavailable:
            return web.json_response(
                {"reason": "Publish policy not yet available."}, status=503
            )
        (queue_position, queue_wait_time) = await get_queue_position(
            conn, run['suite'], run['package']
        )
    response_obj = {
        "package": run['package'],
        "suite": run['suite'],
        "offset": offset,
        "estimated_duration_seconds": estimated_duration.total_seconds(),
        "queue_position": queue_position,
        "queue_wait_time": queue_wait_time.total_seconds(),
    }
    return web.json_response(response_obj)


@response_schema(ScheduleResultSchema())
@routes.post("/run/{run_id}/schedule-control", name="run-schedule-control")
async def handle_schedule_control(request):
    run_id = request.match_info["run_id"]
    post = await request.post()
    offset = post.get("offset")
    try:
        refresh = bool(int(post.get("refresh", "0")))
    except ValueError:
        return web.json_response({"error": "invalid boolean for refresh"}, status=400)
    async with request.app['db'].acquire() as conn:
        run = await conn.fetchrow(
            "SELECT main_branch_revision, package, branch_url FROM run "
            "WHERE id = $1",
            run_id)
        if run is None:
            return web.json_response({"reason": "Run not found"}, status=404)
        if request['user']:
            requestor = request['user']["email"]
        else:
            requestor = "user from web UI"
        if run['branch_url'] is None:
            return web.json_response({"reason": "No branch URL defined."}, status=400)
        offset, estimated_duration = await do_schedule_control(
            conn, run['package'],
            offset=offset,
            refresh=refresh,
            requestor=requestor,
            main_branch_revision=run['main_branch_revision'],
        )
        (queue_position, queue_wait_time) = await get_queue_position(
            conn, "unchanged", run['package']
        )
    response_obj = {
        "package": run['package'],
        "suite": "unchanged",
        "offset": offset,
        "estimated_duration_seconds": estimated_duration.total_seconds(),
        "queue_position": queue_position,
        "queue_wait_time": queue_wait_time.total_seconds(),
    }
    return web.json_response(response_obj)


@docs()
@routes.get("/pkgnames", name="package-names")
async def handle_packagename_list(request):
    response_obj = []
    async with request.app['db'].acquire() as conn:
        for row in await conn.fetch('SELECT name FROM package WHERE NOT removed'):
            response_obj.append(row['name'])
    return web.json_response(response_obj, headers={"Cache-Control": "max-age=600"})


async def get_proposals(conn: asyncpg.Connection, package=None, suite=None):
    args = []
    query = """
SELECT
    DISTINCT ON (merge_proposal.url)
    merge_proposal.package AS package, merge_proposal.url AS url, merge_proposal.status AS status,
    run.suite
FROM
    merge_proposal
LEFT JOIN run
ON merge_proposal.revision = run.revision AND run.result_code = 'success'
"""
    if package is not None:
        args.append(package)
        query += " WHERE run.package = $1"
        if suite:
            query += " AND run.suite = $2"
            args.append(suite)
    elif suite:
        args.append(suite)
        query += " WHERE run.suite = $1"
    query += " ORDER BY merge_proposal.url, run.finish_time DESC"
    return await conn.fetch(query, *args)


class MergeProposalSchema(Schema):

    package = fields.Str(description='package name')
    url = fields.Url(description='merge proposal URL')
    status = fields.Str(description='status')


@docs()
@routes.get("/pkg/{package}/merge-proposals", name="package-merge-proposals")
@routes.get("/merge-proposals", name="merge-proposals")
async def handle_merge_proposal_list(request):
    response_obj = []
    async with request.app['db'].acquire() as conn:
        for row in await get_proposals(conn, request.match_info.get("package"), request.match_info.get("suite")):
            response_obj.append({"package": row['package'], "url": row['url'], "status": row['status']})
    return web.json_response(response_obj)


@docs()
@routes.post("/merge-proposal", name="merge-proposal")
async def handle_merge_proposal_change(request):
    check_admin(request)
    post = await request.post()

    url = urllib.parse.urljoin(request.app['publisher_url'], "merge-proposal")
    async with request.app['http_client_session'].post(url, data={
            'url': post['url'], 'status': post['status']}, raise_for_status=True):
        return web.json_response({})


@docs()
@routes.post("/refresh-proposal-status", name="refresh-proposal-status")
async def handle_refresh_proposal_status(request):
    post = await request.post()
    try:
        mp_url = post["url"]
    except KeyError:
        raise web.HTTPBadRequest(text="No URL specified")

    data = {"url": mp_url}
    url = urllib.parse.urljoin(request.app['publisher_url'], "refresh-status")
    async with request.app['http_client_session'].post(url, data=data) as resp:
        if resp.status in (200, 202):
            return web.Response(text="Success", status=resp.status)
        return web.Response(text=(await resp.text()), status=resp.status)


class QueueItemSchema(Schema):

    queue_id = fields.Int(description="Queue identifier")
    branch_url = fields.Str(description="Branch URL")
    package = fields.Str(description="Package name")
    context = fields.Str(description="Run context")  # type: ignore
    command = fields.Str(description="Command")


@docs()
@routes.get("/queue", name="queue")
async def handle_queue(request):
    limit = request.query.get("limit")
    params = {}
    if limit is not None:
        params['limit'] = str(int(limit))
    url = URL(request.app['runner_url']) / "queue"
    span = aiozipkin.request_span(request)
    with span.new_child('runner:queue'):
        async with request.app['http_client_session'].get(url, param=params) as resp:
            return web.json_response(
                await resp.json(), status=resp.status, headers={"Cache-Control": "max-age=60"})


@docs()
@routes.get("/{suite}/pkg/{package}/revision-info", name="package-revision-info")
@routes.get("/pkg/{package}/run/{run_id}/revision-info", name="package-run-revision-info")
@routes.get("/run/{run_id}/revision-info", name="run-revision-info")
async def handle_revision_info(request):
    role = request.query.get("role", "main")
    run_id = request.match_info.get('run_id')
    package = request.match_info.get("package")
    suite = request.match_info.get("suite")
    run = await find_vcs_info(request.app['db'], role, run_id, package, suite)
    if run is None:
        if run_id is None:
            return web.json_response(
                {"error": "no unabsorbed run for %s/%s" % (package, suite)},
                status=404)
        else:
            return web.json_response(
                {"error": "no run %s" % (run_id, )}, status=404)

    try:
        revision_info = await request.app['vcs_manager'].get_revision_info(
            run['package'],
            run['base_revision'].encode('utf-8') if run['base_revision'] else None,
            run['revision'].encode('utf-8') if run['revision'] else None,
            run['vcs_type'])
        return web.json_response(revision_info)
    except ContentTypeError as e:
        return web.json_response(
            {"error": "publisher returned error %d" % e.code}, status=400)
    except ClientConnectorError:
        return web.json_response(
            {"error": "unable to contact publisher"}, status=502)
    except ClientOSError:
        return web.json_response(
            {"error": "unable to contact publisher - oserror"}, status=502)


async def find_vcs_info(db, role, run_id=None, package=None, suite=None):
    async with db.acquire() as conn:
        if run_id is None:
            return await conn.fetchrow(
                'SELECT id, package, vcs_type, new_result_branch.base_revision, '
                'new_result_branch.revision FROM last_unabsorbed_runs '
                'LEFT JOIN new_result_branch ON '
                'new_result_branch.run_id = last_unabsorbed_runs.id '
                'WHERE package = $1 AND suite = $2 AND role = $3',
                package, suite, role)
        else:
            return await conn.fetchrow(
                'SELECT id, package, vcs_type, '
                'new_result_branch.base_revision AS base_revision, '
                'new_result_branch.revision AS revision FROM run '
                'LEFT JOIN new_result_branch ON new_result_branch.run_id = run.id '
                'WHERE id = $1 AND role = $2', run_id, role)



@docs()
@routes.get("/{suite}/pkg/{package}/diff", name="package-diff")
@routes.get("/pkg/{package}/run/{run_id}/diff", name="package-run-diff")
@routes.get("/run/{run_id}/diff", name="run-diff")
async def handle_diff(request):
    role = request.query.get("role", "main")
    run_id = request.match_info.get('run_id')
    package = request.match_info.get("package")
    suite = request.match_info.get("suite")
    run = await find_vcs_info(request.app['db'], role, run_id, package, suite)
    if run is None:
        if run_id:
            raise web.HTTPNotFound(text="no run %s" % (run_id, ))
        else:
            raise web.HTTPNotFound(
                text="no unabsorbed run for %s/%s" % (package, suite))

    try:
        max_diff_size = int(request.query["max_diff_size"])
    except KeyError:
        max_diff_size = None
    try:
        try:
            diff = await get_vcs_diff(
                request.app['http_client_session'], request.app['vcs_manager'],
                run['vcs_type'], run['package'],
                run['base_revision'].encode('utf-8') if run['base_revision'] else None,
                run['revision'].encode('utf-8') if run['revision'] else None)
        except ClientResponseError as e:
            return web.Response(status=e.status, text="Unable to retrieve diff")
        except NotImplementedError:
            raise web.HTTPBadRequest(text="unsupported vcs %s" % run['vcs_type'])
        if max_diff_size is not None and len(diff) > max_diff_size:
            return web.Response(
                status=413,
                text="Diff too large (%d bytes). See it at %s"
                % (
                    len(diff),
                    request.app.router["run-diff"].url_for(run_id=run_id),
                ),
            )

        for accept in iter_accept(request):
            if accept in ("text/x-diff", "text/plain", "*/*"):
                return web.Response(
                    body=diff,
                    content_type="text/x-diff",
                    headers={
                        "Cache-Control": "max-age=3600",
                        "Vary": "Accept",
                    },
                )
            if accept == "text/html":
                return web.Response(
                    text=highlight_diff(diff.decode("utf-8", "replace")),
                    content_type="text/html",
                    headers={
                        "Cache-Control": "max-age=3600",
                        "Vary": "Accept",
                    },
                )
        raise web.HTTPNotAcceptable(
            text="Acceptable content types: " "text/html, text/x-diff"
        )
    except ContentTypeError as e:
        return web.Response(text="publisher returned error %d" % e.code, status=400)
    except ClientConnectorError:
        return web.Response(text="unable to contact publisher", status=502)
    except ClientOSError:
        return web.Response(text="unable to contact publisher - oserror", status=502)


@docs()
@routes.get("/run/{run_id}/{kind:debdiff|diffoscope}", name="run-archive-diff")
@routes.get("/pkg/{package}/run/{run_id}/{kind:debdiff|diffoscope}", name="package-run-archive-diff")
async def handle_archive_diff(request):
    run_id = request.match_info["run_id"]
    kind = request.match_info["kind"]
    async with request.app['db'].acquire() as conn:
        run = await conn.fetchrow(
            'select id, package, suite, main_branch_revision, result_code from run where id = $1',
            run_id)
        if run is None:
            raise web.HTTPNotFound(text="No such run: %s" % run_id)
        unchanged_run_id = await conn.fetchval(
            "SELECT id FROM run WHERE "
            "package = $1 AND revision = $2 AND result_code = 'success' "
            "ORDER BY finish_time DESC LIMIT 1",
            run['package'], run['main_branch_revision'])
        if unchanged_run_id is None:
            return web.json_response(
                {
                    "reason": "No matching unchanged build for %s" % run_id,
                    "run_id": [run['id']],
                    "unavailable_run_id": None,
                    "suite": run['suite'],
                },
                status=404,
            )

    if run['result_code'] != 'success':
        raise web.HTTPNotFound(text="Build %s has no artifacts" % run_id)

    filter_boring = "filter_boring" in request.query

    try:
        debdiff, content_type = await get_archive_diff(
            request.app['http_client_session'],
            request.app['differ_url'],
            run_id,
            unchanged_run_id,
            kind=kind,
            filter_boring=filter_boring,
            accept=request.headers.get("ACCEPT", "*/*"),
        )
    except BuildDiffUnavailable as e:
        return web.json_response(
            {
                "reason": "debdiff not calculated yet (run: %s, unchanged run: %s)"
                % (run['id'], unchanged_run_id),
                "run_id": [unchanged_run_id, run['id']],
                "unavailable_run_id": e.unavailable_run_id,
                "suite": run['suite'],
            },
            status=404,
        )
    except DebdiffRetrievalError as e:
        return web.json_response(
            {
                "reason": "unable to contact differ for binary diff: %r" % e,
                "inner_reason": e.args[0],
            },
            status=503,
        )

    return web.Response(
        body=debdiff,
        content_type=content_type,
        headers={"Cache-Control": "max-age=3600", "Vary": "Accept"},
    )


async def consider_publishing(session, publisher_url, run_id):
    url = urllib.parse.urljoin(publisher_url, "/consider/%s" % run_id)
    try:
        async with session.post(url) as resp:
            if resp.status != 200:
                logging.warning(
                    'Failed to submit run %s for publish consideration: %s',
                    run_id, await resp.read())
    except ClientConnectorError:
        logging.warning(
                'Failed to submit %s for publish consideration', run_id)


@docs()
@routes.post("/run/{run_id}", name="run-update")
@routes.post("/pkg/{package}/run/{run_id}", name="package-run-update")
@routes.post("/{suite}/pkg/{package}", name="suite-package-run-update")
async def handle_run_post(request):
    from .review import store_review
    async with request.app['db'].acquire() as conn:
        try:
            run_id = request.match_info["run_id"]
        except KeyError:
            package = request.match_info["package"]
            suite = request.match_info["suite"]
            run_id = await conn.fetchval(
                'SELECT id FROM run WHERE package = $1 AND suite = $2',
                package, suite)

        check_qa_reviewer(request)
        span = aiozipkin.request_span(request)
        post = await request.post()
        review_status = post.get("review-status")
        review_comment = post.get("review-comment")
        if review_status:
            review_status = review_status.lower()
            if review_status == "reschedule":
                with span.new_child('sql:run'):
                    run = await conn.fetchrow(
                        'SELECT package, suite FROM run WHERE id = $1',
                        run_id)
                with span.new_child('schedule'):
                    await do_schedule(
                        conn,
                        run['package'],
                        run['suite'],
                        refresh=True,
                        requestor="reviewer",
                        bucket="default",
                    )
                review_status = "rejected"
            with span.new_child('sql:update-run'):
                try:
                    user = request['user']['email']
                except KeyError:
                    user = request['user']['name']
                await store_review(
                    conn, run_id, status=review_status, comment=review_comment,
                    reviewer=user)
            if review_status == 'approved':
                await consider_publishing(
                    request.app['http_client_session'], request.app['publisher_url'],
                    run_id)
        return web.json_response(
            {"review-status": review_status, "review-comment": review_comment}
        )


class BuildInfoSchema(Schema):

    version = fields.Str(description="build version")
    distribution = fields.Str(description="build distribution name")


class RunSchema(Schema):

    run_id = fields.Str(description="Run identifier")
    start_time = fields.DateTime(description="Run start time")
    finish_time = fields.DateTime(description="Run finish time")
    command = fields.Str(description="Command to run")
    description = fields.Str(description="Build result description")
    package = fields.Str(description="Package name")
    build_info = BuildInfoSchema()
    result_code = fields.Str(description="Result code")


@docs()
@routes.get("/run/+index", name="run-list")
@routes.get("/pkg/{package}/run", name="package-run-list")
async def handle_run_list(request):
    query = 'SELECT id FROM run'
    limit = request.query.get("limit")
    package = request.match_info.get('package')
    args = []
    if package is not None:
        query += ' WHERE package = $1'
        args.append(package)
    if limit is not None:
        query += ' LIMIT %d' % int(limit)
    response_obj = []
    async with request.app['db'].acquire() as conn:
        for row in await conn.fetch(query, *args):
            response_obj.append({'run_id': row['id']})
    return web.json_response(response_obj, headers={"Cache-Control": "max-age=600"})


@docs()
@routes.get("/run/+success", name="run-success-list")
async def handle_run_success_list(request):
    query = (
        'select id, '
        'array(select role from new_result_branch where run_id = id) as branches, '
        'result, review_status from run where result_code = \'success\'')
    limit = request.query.get("limit")
    if limit is not None:
        query += ' limit %d' % int(limit)
    response_obj = []
    async with request.app['db'].acquire() as conn:
        for row in await conn.fetch(query):
            response_obj.append({
                'run_id': row['id'],
                'result': row['result'],
                'branches': row['branches'],
                'review_status': row['review_status']})
    return web.json_response(response_obj, headers={"Cache-Control": "max-age=600"})


@docs()
@routes.post("/publish/scan", name="publish-scan")
async def handle_publish_scan(request):
    check_admin(request)
    publisher_url = request.app['publisher_url']
    url = urllib.parse.urljoin(publisher_url, "/scan")
    try:
        async with request.app['http_client_session'].post(url) as resp:
            return web.Response(body=await resp.read(), status=resp.status)
    except ClientConnectorError:
        return web.Response(text="unable to contact publisher", status=400)


@docs()
@routes.post("/publish/autopublish", name="publish-autopublish")
async def handle_publish_autopublish(request):
    check_admin(request)
    publisher_url = request.app['publisher_url']
    url = urllib.parse.urljoin(publisher_url, "/autopublish")
    try:
        async with request.app['http_client_session'].post(url) as resp:
            return web.Response(body=await resp.read(), status=resp.status)
    except ClientConnectorError:
        return web.Response(text="unable to contact publisher", status=400)


@docs()
@routes.get("/runner/status", name="runner-status")
async def handle_runner_status(request):
    url = URL(request.app['runner_url']) / "status"
    span = aiozipkin.request_span(request)
    with span.new_child('runner:status'):
        try:
            async with request.app['http_client_session'].get(url) as resp:
                return web.json_response(await resp.json(), status=resp.status)
        except ContentTypeError as e:
            return web.json_response({"reason": "runner returned error %s" % e}, status=400)
        except ClientConnectorError as e:
            return web.json_response({"reason": "unable to contact runner", "details": repr(e)}, status=502)


@docs()
@routes.get("/active-runs/{run_id}/log/", name="run-log-list")
async def handle_runner_log_index(request):
    run_id = request.match_info["run_id"]
    url = URL(request.app['runner_url']) / "log" / run_id
    span = aiozipkin.request_span(request)
    with span.new_child('runner:log-list'):
        try:
            async with request.app['http_client_session'].get(url) as resp:
                ret = await resp.json()
        except ContentTypeError as e:
            return web.json_response({"reason": "runner returned error %s" % e}, status=400)
        except ClientConnectorError as e:
            return web.json_response({"reason": "unable to contact runner", "details": repr(e)}, status=502)
        except asyncio.TimeoutError:
            return web.json_response({"reason": "timeout contacting runner"}, status=502)

    for accept in iter_accept(request):
        if accept in ('application/json', ):
            return web.json_response(ret)
        elif accept in ('text/plain', ):
            return web.Response(
                text=''.join([line + '\n' for line in ret]),
                content_type='text/plain')
        elif accept in ('text/html', ):
            text = await render_template_for_request(
                env, "log-index.html", request, {'contents': ret})
            return web.Response(text=text, content_type="text/html")

    return web.json_response(ret)


@docs(
    responses={
        200: {"description": "success response"},
    })
@routes.post("/active-runs/{run_id}/kill", name="run-kill")
async def handle_runner_kill(request):
    span = aiozipkin.request_span(request)
    with span.new_child('check-admin'):
        check_admin(request)
    run_id = request.match_info["run_id"]
    with span.new_child('runner:kill'):
        url = urllib.parse.urljoin(request.app['runner_url'], "kill/%s" % run_id)
        try:
            async with request.app['http_client_session'].post(url) as resp:
                return web.json_response(await resp.json(), status=resp.status)
        except ContentTypeError as e:
            return web.json_response({"reason": "runner returned error %s" % e}, status=400)
        except ClientConnectorError:
            return web.json_response({"reason": "unable to contact runner"}, status=502)
        except asyncio.TimeoutError:
            return web.Response(text="timeout contacting runner", status=502)


@docs()
@routes.get("/active-runs/{run_id}/log/{filename}", name="run-log")
async def handle_runner_log(request):
    run_id = request.match_info["run_id"]
    filename = request.match_info["filename"]
    span = aiozipkin.request_span(request)
    with span.new_child('runner:log'):
        url = urllib.parse.urljoin(request.app['runner_url'], "log/%s/%s" % (run_id, filename))
        try:
            async with request.app['http_client_session'].get(url) as resp:
                body = await resp.read()
                return web.Response(
                    body=body, status=resp.status, content_type="text/plain"
                )
        except ContentTypeError as e:
            return web.Response(text="runner returned error %s" % e, status=400)
        except ClientConnectorError:
            return web.Response(text="unable to contact runner", status=502)
        except asyncio.TimeoutError:
            return web.Response(text="timeout contacting runner", status=502)


@docs()
@routes.get("/publish/{publish_id}", name="publish-details")
async def handle_publish_id(request):
    publish_id = request.match_info["publish_id"]
    async with request.app['db'].acquire() as conn:
        row = await conn.fetchrow("""
SELECT
  package,
  branch_name,
  main_branch_revision,
  revision,
  mode,
  merge_proposal_url,
  result_code,
  description
FROM publish WHERE id = $1
""", publish_id)
        if row:
            raise web.HTTPNotFound(text="no such publish: %s" % publish_id)
    return web.json_response(
        {
            "package": row['package'],
            "branch": row['branch_name'],
            "main_branch_revision": row['main_branch_revision'],
            "revision": row['revision'],
            "mode": row['mode'],
            "merge_proposal_url": row['merge_proposal_url'],
            "result_code": row['result_code'],
            "description": row['description'],
        }
    )


@docs()
@routes.get("/{suite:" + SUITE_REGEX + "}/report", name="report")
async def handle_report(request):
    suite = request.match_info["suite"]
    report = {}
    merge_proposal = {}
    async with request.app['db'].acquire() as conn:
        for package, url in await conn.fetch("""
SELECT
    DISTINCT ON (merge_proposal.url)
    merge_proposal.package, merge_proposal.url
FROM
    merge_proposal
LEFT JOIN run
ON merge_proposal.revision = run.revision AND run.result_code = 'success'
AND status = 'open'
WHERE run.suite = $1
""", suite):
            merge_proposal[package] = url
        query = """
SELECT DISTINCT ON (package)
  result_code,
  start_time,
  package,
  result
FROM
  last_unabsorbed_runs
WHERE suite = $1
ORDER BY package, suite, start_time DESC
"""
        for record in await conn.fetch(query, suite):
            if record['result_code'] not in ("success", "nothing-to-do"):
                continue
            data = {
                "timestamp": record['start_time'].isoformat(),
                "result": record['result'],
            }
            if record['package'] in merge_proposal:
                data["merge-proposal"] = merge_proposal[record['package']]
            report[record['package']] = data
    return web.json_response(
        report, headers={"Cache-Control": "max-age=600"}, status=200
    )


@docs()
@routes.get("/needs-review", name="needs-review")
@routes.get("/{suite:" + SUITE_REGEX + "}/needs-review", name="needs-review-suite")
async def handle_needs_review(request):
    from .review import iter_needs_review
    suite = request.match_info.get("suite")
    reviewer = request.query.get("reviewer")
    if reviewer is None and request.get('user'):
        reviewer = request['user'].get('email')
    span = aiozipkin.request_span(request)
    publishable_only = request.query.get("publishable_only", "true") == "true"
    if 'required_only' in request.query:
        required_only = (request.query['required_only'] == 'true')
    else:
        required_only = None
    limit = request.query.get("limit", '200')
    if limit:
        limit = int(limit)
    else:
        limit = None
    ret = []
    async with request.app['db'].acquire() as conn:
        with span.new_child('sql:needs-review'):
            for (
                run_id,
                command,
                package,
                suite,
                vcs_type,
                result_branches,
                main_branch_revision,
                value,
            ) in await iter_needs_review(
                conn,
                suites=([suite] if suite else None),
                required_only=required_only,
                publishable_only=publishable_only,
                reviewer=reviewer,
                limit=limit
            ):
                ret.append({
                    'package': package,
                    'command': command,
                    'id': run_id,
                    'branches': [rb[0] for rb in result_branches],
                    'value': value})
    return web.json_response(ret, status=200)


@docs()
@routes.get("/active-runs/+peek")
async def handle_run_peek(request):
    span = aiozipkin.request_span(request)
    url = URL(request.app['runner_url']) / "active-runs/+peek"
    with span.new_child('forward-runner'):
        try:
            async with request.app['http_client_session'].get(url) as resp:
                if resp.status != 201:
                    try:
                        internal_error = await resp.json()
                    except ContentTypeError:
                        internal_error = await resp.text()
                    ret = {"internal-status": resp.status, "internal-result": internal_error}
                    if 'reason' in internal_error:
                        ret['reason'] = internal_error['reason']
                    return web.json_response(ret, status=400)
                assignment = await resp.json()
                return web.json_response(assignment, status=201)
        except (ClientConnectorError, ServerDisconnectedError) as e:
            return web.json_response({"reason": "unable to contact runner: %s" % e}, status=502)
        except asyncio.TimeoutError as e:
            return web.json_response({"reason": "timeout contacting runner: %s" % e}, status=502)


@docs()
@routes.post("/active-runs", name="run-assign")
async def handle_run_assign(request):
    json = await request.json()
    span = aiozipkin.request_span(request)
    with span.new_child('check-worker-creds'):
        worker_name = await check_worker_creds(request.app['db'], request)
    url = URL(request.app['runner_url']) / "active-runs"
    with span.new_child('forward-runner'):
        try:
            json["worker"] = worker_name
            async with request.app['http_client_session'].post(
                    url, json=json, timeout=ClientTimeout(60)
            ) as resp:
                if resp.status != 201:
                    try:
                        internal_error = await resp.json()
                    except ContentTypeError:
                        internal_error = await resp.text()
                    ret = {"internal-status": resp.status, "internal-result": internal_error}
                    if 'reason' in internal_error:
                        ret['reason'] = internal_error['reason']
                    return web.json_response(ret, status=400)
                assignment = await resp.json()
                return web.json_response(assignment, status=201)
        except (ClientConnectorError, ServerDisconnectedError) as e:
            return web.json_response({"reason": "unable to contact runner: %s" % e}, status=502)
        except asyncio.TimeoutError as e:
            return web.json_response({"reason": "timeout contacting runner: %s" % e}, status=502)


@docs()
@routes.post("/active-runs/{run_id}/finish", name="run-finish")
async def handle_run_finish(request: web.Request) -> web.Response:
    span = aiozipkin.request_span(request)
    with span.new_child('check-worker-creds'):
        worker_name = await check_worker_creds(request.app['db'], request)
    run_id = request.match_info["run_id"]
    with span.new_child('multipart-init'):
        reader: aiohttp.MultipartReader = await request.multipart()
        result = None
        with aiohttp.MultipartWriter("mixed") as runner_writer:
            while True:
                part = await reader.next()
                if part is None:
                    break
                if part.filename is None:  # type: ignore
                    logging.warning(
                        "No filename for part with headers %r", part.headers)
                    return web.json_response(
                        {
                            "reason": "missing filename for part",
                            "content_type": part.headers.get(aiohttp.hdrs.CONTENT_TYPE),
                        },
                        status=400,
                    )
                if part.filename == "result.json":  # type: ignore
                    result = await part.json()  # type: ignore
                else:
                    runner_writer.append(await part.read(), headers=part.headers)  # type: ignore

    if result is None:
        return web.json_response({"reason": "missing result.json"}, status=400)

    result["worker_name"] = worker_name

    part = runner_writer.append_json(  # type: ignore
        result,
        headers=[  # type: ignore
            ("Content-Disposition",
             'attachment; filename="result.json"; ' "filename*=utf-8''result.json")]
    )

    runner_url = urllib.parse.urljoin(request.app['runner_url'], "active-runs/%s/finish" % run_id)
    with span.new_child('runner:finish'):
        try:
            async with request.app['http_client_session'].post(
                runner_url, data=runner_writer
            ) as resp:
                if resp.status == 404:
                    json = await resp.json()
                    return web.json_response({"reason": json["reason"]}, status=404)
                if resp.status == 409:
                    result = await resp.json()
                    result["api_url"] = str(request.app.router["run"].url_for(run_id=result['id']))
                    return web.json_response(result, status=409)
                if resp.status not in (201, 200):
                    try:
                        internal_error = await resp.json()
                    except ContentTypeError:
                        internal_error = await resp.text()
                    return web.json_response(
                        {
                            "internal-status": resp.status,
                            "internal-reporter": "runner",
                            "internal-result": internal_error,
                        },
                        status=400,
                    )
                result = await resp.json()
        except ClientConnectorError:
            return web.Response(text="unable to contact runner", status=502)
        except ServerDisconnectedError:
            return web.Response(text="server disconnected", status=502)

    result["api_url"] = str(request.app.router["run"].url_for(run_id=run_id))
    return web.json_response(result, status=201)


def create_background_task(fn, title):
    loop = asyncio.get_event_loop()
    task = loop.create_task(fn)

    def log_result(future):
        try:
            future.result()
        except BaseException:
            logging.exception('%s failed', title)
        else:
            logging.debug('%s succeeded', title)
    task.add_done_callback(log_result)
    return task


@docs()
@routes.post('/run/{run_id}/reprocess-logs', name='admin-reprocess-logs-run')
async def handle_run_reprocess_logs(request):
    from ..reprocess_logs import reprocess_run_logs
    check_admin(request)
    post = await request.post()
    run_id = request.match_info['run_id']
    dry_run = 'dry_run' in post
    reschedule = 'reschedule' in post
    async with request.app['db'].acquire() as conn:
        run = await conn.fetchrow(
            'SELECT package, suite, command, finish_time - start_time AS duration, '
            'result_code, description, failure_details, change_set FROM run WHERE id = $1',
            run_id)

    result = await reprocess_run_logs(
        request.app['db'],
        request.app['logfile_manager'],
        run['package'], run['suite'], run_id,
        run['command'], run['change_set'], run['duration'], run['result_code'],
        run['description'], run['failure_details'],
        dry_run=dry_run, reschedule=reschedule)

    if result:
        (new_code, new_description, new_failure_details) = result
        return web.json_response(
            {'changed': True,
             'result_code': new_code,
             'description': new_description,
             'failure_details': new_failure_details})
    else:
        return web.json_response({
            'changed': False,
            'result_code': run['result_code'],
            'description': run['description'],
            'failure_details': run['failure_details']})


@docs()
@routes.post('/reprocess-logs', name='admin-reprocess-logs')
async def handle_reprocess_logs(request):
    from ..reprocess_logs import reprocess_run_logs
    check_admin(request)
    post = await request.post()
    dry_run = 'dry_run' in post
    reschedule = 'reschedule' in post
    try:
        run_ids = post.getall('run_id')
    except KeyError:
        run_ids = None

    if not run_ids:
        args = []
        query = """
SELECT
  package,
  suite,
  id,
  command,
  finish_time - start_time AS duration,
  result_code,
  description,
  failure_details
FROM run
WHERE
  (result_code = 'build-failed' OR
   result_code LIKE 'build-failed-stage-%' OR
   result_code LIKE 'autopkgtest-%' OR
   result_code LIKE 'build-%' OR
   result_code LIKE 'dist-%' OR
   result_code LIKE 'unpack-%s' OR
   result_code LIKE 'create-session-%' OR
   result_code LIKE 'missing-%')
"""
    else:
        args = [run_ids]
        query = """
SELECT
  package,
  suite,
  id,
  command,
  finish_time - start_time AS duration,
  result_code,
  description,
  failure_details,
  change_set
FROM run
WHERE
  id = ANY($1::text[])
"""
    async with request.app['db'].acquire() as conn:
        rows = await conn.fetch(query, *args)

    async def do_reprocess():
        todo = [
            reprocess_run_logs(
                request.app['db'],
                request.app['logfile_manager'],
                row['package'], row['suite'], row['id'],
                row['command'], row['change_set'], row['duration'], row['result_code'],
                row['description'], row['failure_details'],
                dry_run=dry_run, reschedule=reschedule)
            for row in rows]
        for i in range(0, len(todo), 100):
            await asyncio.wait(set(todo[i : i + 100]))

    create_background_task(do_reprocess(), 'reprocess logs')

    return web.json_response([
        {'package': row['package'],
         'suite': row['suite'],
         'log_id': row['id']}
        for row in rows])


@docs()
@routes.post('/mass-reschedule', name='admin-reschedule')
async def handle_mass_reschedule(request):
    check_admin(request)
    post = await request.post()
    try:
        result_code = post['result_code']
    except KeyError:
        raise web.HTTPBadRequest(text='result_code not specified')
    suite = post.get('suite')
    description_re = post.get('description_re')
    min_age = int(post.get('min_age', '0'))
    rejected = 'rejected' in post
    offset = int(post.get('offset', '0'))
    refresh = 'refresh' in post
    config = request.app['config']
    all_campaigns = [c.name for c in config.campaign]
    async with request.app['db'].acquire() as conn:
        query = """
SELECT
  package,
  suite,
  finish_time - start_time AS duration
FROM last_runs
WHERE
    branch_url IS NOT NULL AND
    package IN (SELECT name FROM package WHERE NOT removed) AND
"""
        where = []
        params = []
        if result_code is not None:
            params.append(result_code)
            where.append("result_code = $%d" % len(params))
        if suite:
            params.append(suite)
            where.append("suite = $%d" % len(params))
        else:
            params.append(all_campaigns)
            where.append("suite = ANY($%d::text[])" % len(params))
        if rejected:
            where.append("review_status = 'rejected'")
        if description_re:
            params.append(description_re)
            where.append("description ~ $%d" % len(params))
        if min_age:
            params.append(datetime.utcnow() - timedelta(days=min_age))
            where.append("finish_time < $%d" % len(params))
        query += " AND ".join(where)
        runs = await conn.fetch(query, *params)

    async def do_reschedule():
        async with request.app['db'].acquire() as conn:
            for run in runs:
                logging.info("Rescheduling %s, %s" % (run['package'], run['suite']))
                try:
                    await do_schedule(
                        conn,
                        run['package'],
                        run['suite'],
                        estimated_duration=run['duration'],
                        requestor="reschedule",
                        refresh=refresh,
                        offset=offset,
                        bucket="reschedule",
                    )
                except PolicyUnavailable:
                    logging.debug(
                        'Not rescheduling %s/%s: policy unavailable',
                        run['package'], run['suite'])

    create_background_task(do_reschedule(), 'mass-reschedule')
    return web.json_response([
            {'package': run['package'], 'suite': run['suite']}
            for run in runs])


@docs()
@routes.get("/active-runs", name="active-runs-list")
async def handle_list_active_runs(request):
    span = aiozipkin.request_span(request)
    with span.new_child('runner:active-runs-list'):
        url = urllib.parse.urljoin(request.app['runner_url'], "status")
        async with request.app['http_client_session'].get(url) as resp:
            if resp.status != 200:
                return web.json_response(await resp.json(), status=resp.status)
            status = await resp.json()
            return web.json_response(status["processing"], status=200)


@docs()
@routes.get("/result-codes/{result_code}", name="result-code")
async def handle_result_code(request):
    result_code = request.match_info["result_code"]
    ret = []
    async with request.app['db'].acquire() as conn:
        for row in await conn.fetch(
                'SELECT id, package, vcs_type, branch_url FROM last_runs '
                'WHERE result_code = $1', result_code):
            ret.append({
                'run_id': row['id'],
                'package': row['package'],
                'vcs_type': row['vcs_type'],
                'branch_url': row['branch_url'],
                })
    return web.json_response(ret)


@docs()
@routes.get("/active-runs/{run_id}", name="active-run-get")
async def handle_get_active_run(request):
    run_id = request.match_info["run_id"]
    span = aiozipkin.request_span(request)
    with span.new_child('runner:get-active-run'):
        url = urllib.parse.urljoin(request.app['runner_url'], "status")
        async with request.app['http_client_session'].get(url) as resp:
            if resp.status != 200:
                return web.json_response(await resp.json(), status=resp.status)
            processing = (await resp.json())["processing"]
            for entry in processing:
                if entry["id"] == run_id:
                    return web.json_response(entry, status=200)
            return web.json_response({}, status=404)


def create_app(
    db,
    publisher_url: str,
    runner_url: str,
    vcs_manager: VcsManager,
    differ_url: str,
    config: Config,
    external_url: Optional[URL] = None,
    trace_configs=None,
) -> web.Application:
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[trailing_slash_redirect])
    app.router.add_routes(routes)
    app['http_client_session'] = ClientSession(trace_configs=trace_configs)
    app['config'] = config
    app['logfile_manager'] = get_log_manager(config.logs_location)
    app['db'] = db
    app['external_url'] = external_url
    app['publisher_url'] = publisher_url
    app['vcs_manager'] = vcs_manager
    app['runner_url'] = runner_url
    app['differ_url'] = differ_url

    async def redirect_docs(req):
        raise web.HTTPFound(location='docs')

    app.router.add_get('/', redirect_docs)

    setup_aiohttp_apispec(
        app=app,
        title="Debian Janitor API Documentation",
        version=None,
        url="/swagger.json",
        swagger_path="/docs",
    )

    # app.middlewares.append(apispec_validation_middleware)
    return app
