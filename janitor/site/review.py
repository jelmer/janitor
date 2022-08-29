#!/usr/bin/python3

import logging

import asyncpg
import aiozipkin
from asyncio import TimeoutError
from aiohttp import ClientConnectorError, ClientResponseError
from typing import List, Optional, Any

from janitor import state
from . import (
    env,
    get_archive_diff,
    get_vcs_diff,
    BuildDiffUnavailable,
    DebdiffRetrievalError,
)
from .common import (
    get_unchanged_run,
    render_template_for_request,
)

MAX_DIFF_SIZE = 200 * 1024


async def iter_needs_review(
        conn: asyncpg.Connection,
        campaigns: Optional[List[str]] = None,
        limit: Optional[int] = None,
        publishable_only: bool = False,
        required_only: Optional[bool] = None,
        reviewer: Optional[str] = None):
    args: List[Any] = []
    query = """
SELECT id, command, package, suite, vcs_type, result_branches, main_branch_revision, value FROM publish_ready
"""
    conditions = []
    if campaigns is not None:
        args.append(campaigns)
        conditions.append("suite = ANY($%d::text[])" % len(args))

    publishable_condition = (
        "exists (select from unnest(unpublished_branches) where "
        "mode in ('propose', 'attempt-push', 'push-derived', 'push'))"
    )

    order_by = []

    order_by.append("(SELECT COUNT(*) FROM review WHERE run_id = id) ASC")

    if publishable_only:
        conditions.append(publishable_condition)
    else:
        order_by.append(publishable_condition + " DESC")

    if required_only is not None:
        args.append(required_only)
        conditions.append('needs_review = $%d' % (len(args)))

    if reviewer is not None:
        args.append(reviewer)
        conditions.append('not exists (select from review where reviewer = $%d and run_id = id)' % (len(args)))

    if conditions:
        query += " WHERE " + " AND ".join(conditions)

    order_by.extend(["value DESC NULLS LAST", "finish_time DESC"])

    if order_by:
        query += " ORDER BY " + ", ".join(order_by) + " "

    if limit is not None:
        query += " LIMIT %d" % limit
    return await conn.fetch(query, *args)


async def generate_rejected(conn, config, campaign=None):
    if campaign is None:
        campaigns = [c.name for c in config.campaign]
    else:
        campaigns = [campaign]

    runs = await conn.fetch(
        "SELECT id, suite, package, review_comment FROM run "
        "WHERE review_status = 'rejected' AND suite = ANY($1::text[]) "
        "ORDER BY finish_time DESC",
        campaigns)

    return {"runs": runs, "suite": campaign}


async def generate_review(
    conn, request, client, differ_url, vcs_managers, suites=None,
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
            campaigns=suites,
            publishable_only=publishable_only,
            required_only=required_only,
            reviewer=reviewer
        )
    if not entries:
        return await render_template_for_request(
            env, "cupboard/review-done.html", request, {
                'publishable_only': publishable_only})

    (
        run_id,
        command,
        package,
        suite,
        vcs_type,
        result_branches,
        main_branch_revision,
        value,
    ) = entries.pop(0)

    async def show_diff(role):
        try:
            (remote_name, base_revid, revid) = state.get_result_branch(result_branches, role)
        except KeyError:
            return ""
        external_url = "/api/run/%s/diff?role=%s" % (run_id, role)
        try:
            with span.new_child('vcs-diff'):
                diff = (await get_vcs_diff(
                    client, vcs_managers[vcs_type], package,
                    base_revid.encode('utf-8') if base_revid else None,
                    revid.encode('utf-8'))).decode("utf-8", "replace")
                if len(diff) > MAX_DIFF_SIZE:
                    return "Diff too large (%d). See it at %s" % (
                        len(diff),
                        external_url,
                    )
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
            (remote_name, base_revid, revid) = state.get_result_branch(result_branches, role)
        except KeyError:
            return []

        old_revid = base_revid.encode('utf-8') if base_revid else None
        new_revid = revid.encode('utf-8') if revid else None
        if old_revid == new_revid:
            return []
        if vcs_type is None:
            logging.warning("No vcs known for run %s", run_id)
            return []
        try:
            return await vcs_managers[vcs_type].get_revision_info(package, old_revid, new_revid)
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
                conn, package, main_branch_revision.encode('utf-8')
            )
        if unchanged_run is None:
            return "<p>No control run</p>"
        try:
            with span.new_child('archive-diff'):
                text, unused_content_type = await get_archive_diff(
                    client,
                    differ_url,
                    run_id,
                    unchanged_run.id,
                    kind="debdiff",
                    filter_boring=True,
                    accept="text/html",
                )
                return text.decode("utf-8", "replace")
        except DebdiffRetrievalError as e:
            return "Unable to retrieve debdiff: %r" % e
        except BuildDiffUnavailable:
            return "<p>No build diff generated</p>"

    kwargs = {
        "show_diff": show_diff,
        "show_debdiff": show_debdiff,
        "get_revision_info": get_revision_info,
        "package_name": package,
        "run_id": run_id,
        "command": command,
        "branches": result_branches,
        "suite": suite,
        "suites": suites,
        "value": value,
        "publishable_only": publishable_only,
        "MAX_DIFF_SIZE": MAX_DIFF_SIZE,
        "todo": [
            {
                'package': entry['package'],
                'command': entry['command'],
                'id': entry['id'],
                'branches': [rb[0] for rb in entry['result_branches']],
                'value': entry['value']
            } for entry in entries
        ],
    }
    return await render_template_for_request(env, "cupboard/review.html", request, kwargs)


async def store_review(conn, run_id, status, comment, reviewer, is_qa_reviewer):
    async with conn.transaction():
        if status != 'abstained' and is_qa_reviewer:
            await conn.execute(
                "UPDATE run SET review_status = $1, review_comment = $2 WHERE id = $3",
                status,
                comment,
                run_id,
            )
        await conn.execute(
            "INSERT INTO review (run_id, comment, reviewer, review_status) VALUES "
            " ($1, $2, $3, $4) ON CONFLICT (run_id, reviewer) "
            "DO UPDATE SET review_status = EXCLUDED.review_status, comment = EXCLUDED.comment, "
            "reviewed_at = NOW()", run_id, comment, reviewer, status)


async def generate_review_stats(conn):
    return {
        'by_reviewer': await conn.fetch(
            "select distinct(reviewer), count(*) from review group by reviewer"),
        'by_review_status': await conn.fetch(
            "with total as (select count(*) as cnt from review) "
            "select review_status, count(*) as cnt, "
            "1.0 * count(*) / (select cnt from total) * 100.0 as pct "
            "from review group by 1"
        )}
