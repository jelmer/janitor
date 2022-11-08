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

from datetime import datetime
import re
from typing import Optional

import aiozipkin

from aiohttp import web

from .. import is_admin, env, check_logged_in, is_qa_reviewer
from ..common import html_template
from ..pkg import MergeProposalUserUrlResolver


@html_template(env, "cupboard/rejected.html")
async def handle_rejected(request):
    from .review import generate_rejected

    campaign = request.query.get("campaign")
    async with request.app.database.acquire() as conn:
        return await generate_rejected(conn, request.app['config'], campaign=campaign)


@html_template(env, "cupboard/history.html", headers={"Vary": "Cookie"})
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


@html_template(env, "cupboard/workers.html", headers={"Vary": "Cookie"})
async def handle_workers(request):
    async with request.app.database.acquire() as conn:
        return {"workers": await conn.fetch(
            'select name, link, count(run.id) as run_count from worker '
            'left join run on run.worker = worker.name '
            'group by worker.name, worker.link')}


@html_template(env, "cupboard/queue.html", headers={"Vary": "Cookie"})
async def handle_queue(request):
    limit = int(request.query.get("limit", "100"))
    from .queue import write_queue

    return await write_queue(
        request.app.database,
        queue_status=request.app['runner_status'],
        limit=limit,
    )


@html_template(env, "cupboard/never-processed.html", headers={"Vary": "Cookie"})
async def handle_never_processed(request):
    campaign = request.query.get("campaign")
    if campaign is not None and campaign.lower() == "_all":
        campaign = None
    campaigns = [campaign] if campaign else None
    async with request.app.database.acquire() as conn:
        query = """\
        select c.package, c.suite from candidate c
        where not exists (
            SELECT FROM run WHERE run.package = c.package AND c.suite = suite)
        """
        args = []
        if campaigns:
            query += " AND suite = ANY($1::text[])"
            args.append(campaigns)
        return {
            "never_processed": await conn.fetch(query, *args),
            "campaign": campaign,
        }


@html_template(env, "cupboard/result-code-index.html", headers={"Vary": "Cookie"})
async def handle_result_codes(request):
    campaign = request.query.get("campaign")
    exclude_never_processed = "exclude_never_processed" in request.query
    exclude_transient = "exclude_transient" in request.query
    if campaign is not None and campaign.lower() == "_all":
        campaign = None
    all_campaigns = [c.name for c in request.app['config'].campaign]
    args = [[campaign] if campaign else all_campaigns]
    async with request.app.database.acquire() as conn:
        query = """\
    select (
            case when result_code = 'nothing-new-to-do' then 'success'
            else result_code end), count(result_code) from last_runs
        where suite = ANY($1::text[])
    """
        if exclude_transient:
            query += " AND NOT failure_transient"
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
            "campaign": campaign, "all_campaigns": all_campaigns}


@html_template(env, "cupboard/result-code.html", headers={"Vary": "Cookie"})
async def handle_result_code(request):
    campaign = request.query.get("campaign")
    if campaign is not None and campaign.lower() == "_all":
        campaign = None
    code = request.match_info.get("code")
    query = ('SELECT * FROM last_runs '
             'WHERE result_code = ANY($1::text[]) AND suite = ANY($2::text[])')
    codes = [code]
    all_campaigns = [c.name for c in request.app['config'].campaign]
    async with request.app.database.acquire() as conn:
        return {
            "code": code,
            "runs": await conn.fetch(query, codes, [campaign] if campaign else all_campaigns),
            "campaign": campaign,
            "all_campaigns": all_campaigns}


@html_template(env, "cupboard/publish.html")
async def handle_publish(request):
    id = request.match_info["id"]
    from .publish import write_publish
    async with request.app.database.acquire() as conn:
        return await write_publish(conn, id)


@html_template(env, "cupboard/publish-history.html", headers={"Vary": "Cookie"})
async def handle_publish_history(request):
    limit = int(request.query.get("limit", "100"))
    from .publish import write_history

    async with request.app.database.acquire() as conn:
        return await write_history(conn, limit=limit)


@html_template(env, "cupboard/review-stats.html", headers={"Vary": "Cookie"})
async def handle_review_stats(request):
    from .review import generate_review_stats
    async with request.app.database.acquire() as conn:
        return await generate_review_stats(conn)


async def handle_review_post(request):
    from .review import generate_review
    from ...review import store_review
    check_logged_in(request)

    post = await request.post()
    publishable_only = post.get("publishable_only", "true") == "true"
    async with request.app.database.acquire() as conn:
        if "review_status" in post:
            review_status = {
                'approve': 'approved',
                'reject': 'rejected',
                'reschedule': 'rescheduled',
                'abstain': 'abstained'}[post["review_status"].lower()]
            review_comment = post.get("review_comment")
            await store_review(
                conn, post["run_id"], status=review_status,
                comment=review_comment,
                reviewer=request['user'],
                is_qa_reviewer=is_qa_reviewer(request))
        text = await generate_review(
            conn,
            request,
            request.app['http_client_session'],
            request.app['differ_url'],
            request.app['vcs_managers'],
            campaigns=post.getall("suite", None),
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

    campaigns = request.query.getall("suite", None)
    async with request.app.database.acquire() as conn:
        text = await generate_review(
            conn,
            request,
            request.app['http_client_session'],
            request.app['differ_url'],
            request.app['vcs_managers'],
            campaigns=campaigns,
            publishable_only=publishable_only,
        )
    return web.Response(
        content_type="text/html", text=text, headers={"Cache-Control": "no-cache"}
    )


@html_template(env, "cupboard/run.html", headers={"Vary": "Cookie"})
async def handle_run(request):
    from ..common import get_run
    from ..pkg import generate_run_file

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
        request.app['http_client_session'],
        request.app['config'],
        request.app['differ_url'],
        request.app['publisher_url'],
        request.app.logfile_manager,
        run,
        request.app['vcs_managers'],
        is_admin=is_admin(request),
        span=span,
    )


@html_template(
    env, "cupboard/broken-merge-proposals.html", headers={"Vary": "Cookie"}
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


@html_template(env, "cupboard/changeset.html", headers={"Vary": "Cookie"})
async def handle_changeset(request):
    span = aiozipkin.request_span(request)
    async with request.app.database.acquire() as conn:
        with span.new_child('sql:changeset'):
            cs = await conn.fetchrow('SELECT * FROM change_set WHERE id = $1', request.match_info['id'])
        with span.new_child('sql:runs'):
            runs = await conn.fetch(
                'SELECT * FROM run WHERE change_set = $1 ORDER BY finish_time DESC',
                request.match_info['id'])
        with span.new_child('sql:todo'):
            todo = await conn.fetch('SELECT * FROM change_set_todo WHERE change_set = $1',
                                    request.match_info['id'])
    return {'changeset': cs, 'runs': runs, 'todo': todo}


@html_template(env, "cupboard/changeset-list.html", headers={"Vary": "Cookie"})
async def handle_changeset_list(request):
    span = aiozipkin.request_span(request)
    async with request.app.database.acquire() as conn:
        with span.new_child('sql:changesets'):
            cs = await conn.fetch("""\
select * from change_set where exists (
    select from candidate where change_set = change_set.id)
""")
    return {'changesets': cs}


async def handle_run_redirect(request):

    run_id = request.match_info["run_id"]

    async with request.app.database.acquire() as conn:
        package = await conn.fetchone("SELECT package FROM run WHERE id = $1", run_id)
        if package is None:
            raise web.HTTPNotFound(text="No such run: %s" % run_id)
        raise web.HTTPPermanentRedirect(
            location=request.app.router["cupboard-run"].url_for(
                pkg=package, run_id=run_id))


@html_template(env, "cupboard/merge-proposals.html", headers={"Vary": "Cookie"})
async def handle_merge_proposals(request):
    from .merge_proposals import write_merge_proposals

    suite = request.match_info.get("suite")
    return await write_merge_proposals(request.app.database, suite)


@html_template(env, "cupboard/merge-proposal.html", headers={"Vary": "Cookie"})
async def handle_merge_proposal(request):
    from .merge_proposals import write_merge_proposal

    try:
        url = request.query["url"]
    except KeyError:
        raise web.HTTPBadRequest(text="no url specified")
    return await write_merge_proposal(request.app.database, url)


async def generate_ready_list(
    db, review_status: Optional[str] = None
):
    async with db.acquire() as conn:
        query = 'SELECT package, suite, id, command, result FROM publish_ready'

        conditions = [
            "EXISTS (SELECT * FROM unnest(unpublished_branches) "
            "WHERE mode in "
            "('propose', 'attempt-push', 'push-derived', 'push'))"]
        args = []
        if review_status:
            args.append(review_status)
            conditions.append('review_status = %d' % len(args))

        query += " WHERE " + " AND ".join(conditions)

        query += " ORDER BY package ASC"

        runs = await conn.fetch(query, *args)
    return {"runs": runs}


async def generate_done_list(db, since: Optional[datetime] = None):
    async with db.acquire() as conn:
        oldest = await conn.fetchval(
            "SELECT MIN(absorbed_at) FROM absorbed_runs")

        if since:
            orig_runs = await conn.fetch(
                "SELECT * FROM absorbed_runs WHERE absorbed_at >= $1 "
                "ORDER BY absorbed_at DESC NULLS LAST", since)
        else:
            orig_runs = await conn.fetch(
                "SELECT * FROM absorbed_runs "
                "ORDER BY absorbed_at DESC NULLS LAST")

    mp_user_url_resolver = MergeProposalUserUrlResolver()

    runs = []
    for orig_run in orig_runs:
        run = dict(orig_run)
        if not run['merged_by']:
            run['merged_by_url'] = None
        else:
            run['merged_by_url'] = mp_user_url_resolver.resolve(
                run['merge_proposal_url'], run['merged_by'])
        runs.append(run)

    return {"oldest": oldest, "runs": runs, "since": since}


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
        )
    else:
        try:
            f = await request.app['artifact_manager'].get_artifact(
                run_id, filename
            )
        except FileNotFoundError:
            raise web.HTTPNotFound(text="No artifact %s for run %s" % (filename, run_id))
        return web.Response(body=f.read())


@html_template(env, "cupboard/ready-list.html", headers={"Vary": "Cookie"})
async def handle_ready_proposals(request):
    review_status = request.query.get("review_status")
    return await generate_ready_list(request.app.database, review_status)


@html_template(env, "cupboard/done-list.html", headers={"Vary": "Cookie"})
async def handle_done_proposals(request):
    since_str = request.query.get("since")
    if since_str:
        try:
            since = datetime.fromisoformat(since_str)
        except ValueError as e:
            raise web.HTTPBadRequest(text="invalid since") from e
    else:
        since = None

    return await generate_done_list(request.app.database, since)


_extra_cupboard_links = []


def register_cupboard_link(title, shortlink):
    _extra_cupboard_links.append((title, shortlink))


def register_cupboard_endpoints(router):
    router.add_get("/cupboard/", handle_cupboard_start, name="cupboard-start")
    router.add_get("/cupboard/rejected", handle_rejected, name="cupboard-rejected")
    router.add_get("/cupboard/history", handle_history, name="history")
    router.add_get("/cupboard/reprocess-logs", handle_reprocess_logs, name="reprocess-logs")
    router.add_get("/cupboard/workers", handle_workers, name="workers")
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
    router.add_get(
        "/cupboard/review-stats", handle_review_stats, name="cupboard-review-stats"
    )
    router.add_get(
        "/cupboard/run/{run_id}/", handle_run_redirect, name="cupboard-run-redirect")
    router.add_get(
        "/cupboard/cs/", handle_changeset_list, name="cupboard-changeset-list")
    router.add_get(
        "/cupboard/cs/{id}/", handle_changeset, name="cupboard-changeset")
    router.add_get("/cupboard/pkg/{pkg}/{run_id}/", handle_run, name="cupboard-run")
    router.add_get(
        "/cupboard/broken-merge-proposals", handle_broken_mps, name="broken-mps"
    )
    router.add_get(
        "/cupboard/merge-proposals",
        handle_merge_proposals,
        name="cupboard-merge-proposals",
    )
    router.add_get(
        "/cupboard/merge-proposal",
        handle_merge_proposal,
        name="cupboard-merge-proposal",
    )
    router.add_get("/cupboard/ready", handle_ready_proposals, name="cupboard-ready")
    router.add_get("/cupboard/done", handle_done_proposals, name="cupboard-done")
    router.add_get(
        "/cupboard/pkg/{pkg}/{run_id}/{filename:.+}",
        handle_result_file,
        name="cupboard-result-file",
    )
