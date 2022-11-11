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


routes = web.RouteTableDef()


@routes.get("/cupboard/rejected", name="cupboard-rejected")
@html_template("cupboard/rejected.html")
async def handle_rejected(request):
    from .review import generate_rejected

    campaign = request.query.get("campaign")
    async with request.app.database.acquire() as conn:
        return await generate_rejected(conn, request.app['config'], campaign=campaign)


@routes.get("/cupboard/history", name="history")
@html_template("cupboard/history.html", headers={"Vary": "Cookie"})
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



@routes.get("/cupboard/reprocess-logs", name="reprocess-logs")
@html_template("cupboard/reprocess-logs.html")
async def handle_reprocess_logs(request):
    return {}


@routes.get("/cupboard/workers", name="workers")
@html_template("cupboard/workers.html", headers={"Vary": "Cookie"})
async def handle_workers(request):
    async with request.app.database.acquire() as conn:
        return {"workers": await conn.fetch(
            'select name, link, count(run.id) as run_count from worker '
            'left join run on run.worker = worker.name '
            'group by worker.name, worker.link')}


@routes.get("/cupboard/queue", name="queue")
@html_template("cupboard/queue.html", headers={"Vary": "Cookie"})
async def handle_queue(request):
    limit = int(request.query.get("limit", "100"))
    from .queue import write_queue

    return await write_queue(
        request.app.database,
        queue_status=request.app['runner_status'],
        limit=limit,
    )


@routes.get("/cupboard/never-processed", name="never-processed")
@html_template("cupboard/never-processed.html", headers={"Vary": "Cookie"})
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


@routes.get("/cupboard/result-codes/", name="result-code-list")
@html_template("cupboard/result-code-index.html", headers={"Vary": "Cookie"})
async def handle_result_codes(request):
    campaign = request.query.get("campaign")
    exclude_never_processed = "exclude_never_processed" in request.query
    include_transient = "include_transient" in request.query
    if campaign is not None and campaign.lower() == "_all":
        campaign = None
    all_campaigns = [c.name for c in request.app['config'].campaign]
    args = [[campaign] if campaign else all_campaigns]
    async with request.app.database.acquire() as conn:
        if include_transient:
            query = """\
    select (
            case when result_code = 'nothing-new-to-do' then 'success'
            else result_code end), count(result_code) from last_runs
        where suite = ANY($1::text[]) group by 1
    """
        else:
            query = """\
    select result_code, count(result_code) from last_effective_runs
    where suite = ANY($1::text[]) group by 1
    """
        if not exclude_never_processed:
            query = """(%s) union
    select 'never-processed', count(*) from candidate c
        where not exists (
            SELECT FROM run WHERE run.package = c.package AND c.suite = suite)
        and suite = ANY($1::text[]) order by 2 desc
    """ % query
        return {
            "exclude_never_processed": exclude_never_processed,
            "include_transient": include_transient,
            "result_codes": await conn.fetch(query, *args),
            "campaign": campaign, "all_campaigns": all_campaigns}


@routes.get("/cupboard/result-codes/{code}", name="result-code")
@html_template("cupboard/result-code.html", headers={"Vary": "Cookie"})
async def handle_result_code(request):
    campaign = request.query.get("campaign")
    include_transient = "include_transient" in request.query
    if campaign is not None and campaign.lower() == "_all":
        campaign = None
    code = request.match_info["code"]
    if code == "success":
        table = "last_runs"
        codes = ["success", "nothing-new-to-do"]
    else:
        codes = [code]
        if include_transient:
            table = "last_runs"
        else:
            table = "last_effective_runs"
    query = ('SELECT * FROM %s '
             'WHERE result_code = ANY($1::text[]) AND suite = ANY($2::text[])' % table)
    all_campaigns = [c.name for c in request.app['config'].campaign]
    async with request.app.database.acquire() as conn:
        return {
            "code": code,
            "runs": await conn.fetch(query, codes, [campaign] if campaign else all_campaigns),
            "campaign": campaign,
            "all_campaigns": all_campaigns}


@routes.get("/cupboard/publish/{id}", name="publish")
@html_template("cupboard/publish.html")
async def handle_publish(request):
    id = request.match_info["id"]
    from .publish import write_publish
    async with request.app.database.acquire() as conn:
        return await write_publish(conn, id)



@routes.get("/cupboard/publish/", name="publish-history")
@html_template("cupboard/publish-history.html", headers={"Vary": "Cookie"})
async def handle_publish_history(request):
    limit = int(request.query.get("limit", "100"))
    from .publish import write_history

    async with request.app.database.acquire() as conn:
        return await write_history(conn, limit=limit)



@routes.get("/cupboard/review-stats", name="cupboard-review-stats")
@html_template("cupboard/review-stats.html", headers={"Vary": "Cookie"})
async def handle_review_stats(request):
    from .review import generate_review_stats
    async with request.app.database.acquire() as conn:
        return await generate_review_stats(conn)


@routes.post("/cupboard/review", name="cupboard-review-post")
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


@routes.get("/cupboard/review", name="cupboard-review")
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


@routes.get("/cupboard/pkg/{pkg}/{run_id}/", name="cupboard-run")
@html_template("cupboard/run.html", headers={"Vary": "Cookie"})
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


@routes.get("/cupboard/broken-merge-proposals", name="broken-mps")
@html_template(
    "cupboard/broken-merge-proposals.html", headers={"Vary": "Cookie"}
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


@routes.get("/cupboard/", name="cupboard-start")
@html_template("cupboard/start.html")
async def handle_cupboard_start(request):
    return {'extra_cupboard_links': _extra_cupboard_links}


@routes.get("/cupboard/cs/{id}/", name="cupboard-changeset")
@html_template("cupboard/changeset.html", headers={"Vary": "Cookie"})
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



@routes.get("/cupboard/cs/", name="cupboard-changeset-list")
@html_template("cupboard/changeset-list.html", headers={"Vary": "Cookie"})
async def handle_changeset_list(request):
    span = aiozipkin.request_span(request)
    async with request.app.database.acquire() as conn:
        with span.new_child('sql:changesets'):
            cs = await conn.fetch("""\
select * from change_set where exists (
    select from candidate where change_set = change_set.id)
""")
    return {'changesets': cs}


@routes.get("/cupboard/run/{run_id}/", name="cupboard-run-redirect")
async def handle_run_redirect(request):

    run_id = request.match_info["run_id"]

    async with request.app.database.acquire() as conn:
        package = await conn.fetchone("SELECT package FROM run WHERE id = $1", run_id)
        if package is None:
            raise web.HTTPNotFound(text="No such run: %s" % run_id)
        raise web.HTTPPermanentRedirect(
            location=request.app.router["cupboard-run"].url_for(
                pkg=package, run_id=run_id))


@routes.get("/cupboard/merge-proposals", name="cupboard-merge-proposals")
@html_template("cupboard/merge-proposals.html", headers={"Vary": "Cookie"})
async def handle_merge_proposals(request):
    from .merge_proposals import write_merge_proposals

    suite = request.match_info.get("suite")
    return await write_merge_proposals(request.app.database, suite)


@routes.get("/cupboard/merge-proposal", name="cupboard-merge-proposal")
@html_template("cupboard/merge-proposal.html", headers={"Vary": "Cookie"})
async def handle_merge_proposal(request):
    from .merge_proposals import write_merge_proposal

    try:
        url = request.query["url"]
    except KeyError:
        raise web.HTTPBadRequest(text="no url specified")
    return await write_merge_proposal(request.app.database, url)


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


@routes.get(
    "/cupboard/pkg/{pkg}/{run_id}/{filename:.+}", name="cupboard-result-file")
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


@routes.get("/cupboard/ready", name="cupboard-ready")
@html_template("cupboard/ready-list.html", headers={"Vary": "Cookie"})
async def handle_ready_proposals(request):
    review_status = request.query.get("review_status")
    async with request.app.database.acquire() as conn:
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


@routes.get("/cupboard/done", name="cupboard-done")
@html_template("cupboard/done-list.html", headers={"Vary": "Cookie"})
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
    router.add_routes(routes)
