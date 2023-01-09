#!/usr/bin/python3

import logging

from typing import Dict, List

import aiozipkin
from asyncio import TimeoutError
from aiohttp import ClientConnectorError, ClientResponseError
import asyncpg

from breezy.revision import NULL_REVISION

from janitor import state
from .. import (
    get_archive_diff,
    BuildDiffUnavailable,
    DebdiffRetrievalError,
)
from ..common import (
    get_unchanged_run,
    render_template_for_request,
)
from . import iter_needs_review


async def generate_rejected(conn, config, campaign=None):
    if campaign is None:
        campaigns = [c.name for c in config.campaign]
    else:
        campaigns = [campaign]

    runs = await conn.fetch(
        "SELECT id, suite, package FROM last_unabsorbed_runs "
        "WHERE review_status = 'rejected' AND suite = ANY($1::text[]) "
        "ORDER BY finish_time DESC",
        campaigns)

    reviews: Dict[str, List[asyncpg.Record]] = {}

    for row in await conn.fetch(
            'SELECT * FROM review WHERE run_id = ANY($1::text[])',
            [run['id'] for run in runs]):
        reviews.setdefault(row['run_id'], []).append(row)

    return {"runs": runs, "suite": campaign, "reviews": reviews}


async def generate_review(
    conn, request, client, differ_url, vcs_managers, campaigns=None,
    publishable_only=True
):
    if 'required_only' in request.query:
        required_only = (request.query['required_only'] == 'true')
    else:
        required_only = None

    limit = int(request.query.get('limit', '100'))

    span = aiozipkin.request_span(request)

    if request.get('user'):
        reviewer = request['user'].get('email')
    else:
        reviewer = None

    with span.new_child('sql:needs-review'):
        entries = await iter_needs_review(
            conn,
            limit=limit,
            campaigns=campaigns,
            publishable_only=publishable_only,
            required_only=required_only,
            reviewer=reviewer
        )
    if not entries:
        return await render_template_for_request(
            "cupboard/review-done.html", request, {
                'publishable_only': publishable_only})

    (
        run_id,
        package,
        campaign,
    ) = entries.pop(0)

    evaluate_url = str(request.url.join(request.app["evaluate_url"]))

    try:
        async with request.app['http_client_session'].get(evaluate_url.replace('RUN_ID', run_id), raise_for_status=True) as resp:
            evaluate = await resp.text()
    except (ClientConnectorError, ClientResponseError) as e:
        evaluate = "Unable to retrieve evaluation: %s" % e

    kwargs = {
        "review_instructions_url": request.app.get("review_instructions_url"),
        "package_name": package,
        "run_id": run_id,
        "suite": campaign,
        "suites": campaigns,
        "campaign": campaign,
        "campaigns": campaigns,
        "evaluate": evaluate,
        "evaluate_url": evaluate_url,
        "publishable_only": publishable_only,
        "todo": [
            {
                'package': entry['package'],
                'id': entry['id'],
            } for entry in entries
        ],
    }
    return await render_template_for_request("cupboard/review.html", request, kwargs)


async def generate_evaluate(db, vcs_managers, http_client_session, differ_url, run_id, span):
    MAX_DIFF_SIZE = 200 * 1024

    async with db.acquire() as conn:
        run = await conn.fetchrow(
            'SELECT package, array(SELECT row(role, remote_name, base_revision, revision) '
            'FROM new_result_branch WHERE run_id = id) AS result_branches, vcs_type, main_branch_revision, '
            'finish_time, value, command FROM run WHERE id = $1', run_id)

    async def show_diff(role):
        try:
            (remote_name, base_revid, revid) = state.get_result_branch(run['result_branches'], role)
        except KeyError:
            return ""
        external_url = f"/api/run/{run_id}/diff?role={role}"
        if run['vcs_type'] is None:
            return "no vcs known"
        if revid is None:
            return "Branch deleted"
        try:
            with span.new_child('vcs-diff'):
                diff = (await vcs_managers[run['vcs_type']].get_diff(
                    run['package'],
                    base_revid.encode('utf-8') if base_revid else NULL_REVISION,
                    revid.encode('utf-8'))
                ).decode("utf-8", "replace")
                if len(diff) > MAX_DIFF_SIZE:
                    return f"Diff too large ({len(diff)}). See it at {external_url}"
                else:
                    return diff
        except ClientResponseError as e:
            return "Unable to retrieve diff; error code %d" % e.status
        except NotImplementedError as e:
            return str(e)
        except ClientConnectorError as e:
            return "Unable to retrieve diff; error %s" % e
        except TimeoutError:
            return "Timeout while retrieving diff; see it at %s" % external_url

    async def get_revision_info(role):
        try:
            (remote_name, base_revid, revid) = state.get_result_branch(run['result_branches'], role)
        except KeyError:
            return []

        if base_revid == revid:
            return []
        if run['vcs_type'] is None:
            logging.warning("No vcs known for run %s", run_id)
            return []
        if revid is None:
            return []
        old_revid = base_revid.encode('utf-8') if base_revid else NULL_REVISION
        new_revid = revid.encode('utf-8')
        try:
            return await vcs_managers[run['vcs_type']].get_revision_info(run['package'], old_revid, new_revid)
        except ClientResponseError as e:
            logging.warning("Unable to retrieve commit info; error code %d", e.status)
            return []
        except ClientConnectorError as e:
            logging.warning("Unable to retrieve diff; error %s", e)
            return []
        except TimeoutError:
            logging.warning("Timeout while retrieving commit info")
            return []

    async def show_debdiff():
        with span.new_child("sql:unchanged-run"):
            unchanged_run = await get_unchanged_run(
                conn, run['package'], run['main_branch_revision'].encode('utf-8')
            )
        if unchanged_run is None:
            return "<p>No control run</p>"
        try:
            with span.new_child('archive-diff'):
                text, unused_content_type = await get_archive_diff(
                    http_client_session,
                    differ_url,
                    run_id,
                    unchanged_run['id'],
                    kind="debdiff",
                    filter_boring=True,
                    accept="text/html",
                )
                return text.decode("utf-8", "replace")
        except DebdiffRetrievalError as e:
            return "Unable to retrieve debdiff: %r" % e
        except BuildDiffUnavailable:
            return "<p>No build diff generated</p>"

    return {
        'run_id': run_id,
        'MAX_DIFF_SIZE': MAX_DIFF_SIZE,
        'finish_time': run['finish_time'],
        'campaign': run['campaign'],
        'branches': run['result_branches'],
        'value': run['value'],
        'command': run['command'],
        'show_diff': show_diff,
        'show_debdiff': show_debdiff,
        "get_revision_info": get_revision_info,
    }
