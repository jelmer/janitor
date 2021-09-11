#!/usr/bin/python3

import asyncpg
import aiozipkin
from asyncio import TimeoutError
from aiohttp import ClientConnectorError
from typing import List, Optional, Tuple, Any, AsyncIterable

from janitor import state
from . import (
    get_archive_diff,
    get_vcs_diff,
    BuildDiffUnavailable,
    DebdiffRetrievalError,
    render_template_for_request,
)
from .common import get_unchanged_run

MAX_DIFF_SIZE = 200 * 1024


async def iter_needs_review(
        conn: asyncpg.Connection,
        suites: Optional[List[str]] = None,
        limit: Optional[int] = None,
        publishable_only: bool = False,
        required_only: Optional[bool] = None,
        reviewer: Optional[str] = None):
    args: List[Any] = []
    query = """
SELECT id, package, suite, vcs_type, result_branches, main_branch_revision, value FROM publish_ready
"""
    conditions = []
    if suites is not None:
        args.append(suites)
        conditions.append("suite = ANY($%d::text[])" % len(args))

    publishable_condition = (
        "exists (select from unnest(unpublished_branches) where "
        "mode in ('propose', 'attempt-push', 'push-derived', 'push'))"
    )

    order_by = []

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


async def generate_rejected(conn, suite=None):
    if suite is None:
        suites = None
    else:
        suites = [suite]
    entries = [
        entry
        async for entry in state.iter_publish_ready(
            conn, review_status=["rejected"], needs_review=False, suites=suites, publishable_only=False
        )
    ]

    def entry_key(entry):
        return entry[0].finish_time

    entries.sort(key=entry_key, reverse=True)
    return {"entries": entries, "suite": suite}


async def generate_review(
    conn, request, client, differ_url, vcs_store_url, suites=None,
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
            suites=suites,
            publishable_only=publishable_only,
            required_only=required_only,
            reviewer=reviewer
        )
    if not entries:
        return await render_template_for_request("review-done.html", request, {})

    (
        run_id,
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
            diff = (await get_vcs_diff(
                client, vcs_store_url, vcs_type, package, base_revid.encode('utf-8') if base_revid else None,
                revid.encode('utf-8'))).decode("utf-8", "replace")
            if len(diff) > MAX_DIFF_SIZE:
                return "Diff too large (%d). See it at %s" % (
                    len(diff),
                    external_url,
                )
            else:
                return diff
        except NotImplementedError as e:
            return str(e)
        except ClientConnectorError as e:
            return "Unable to retrieve diff; error %s" % e
        except TimeoutError:
            return "Timeout while retrieving diff; see it at %s" % external_url

    async def show_debdiff():
        with span.new_child("sql:unchanged-run"):
            unchanged_run = await get_unchanged_run(
                conn, package, main_branch_revision.encode('utf-8')
            )
        if unchanged_run is None:
            return "<p>No control run</p>"
        try:
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
        "package_name": package,
        "run_id": run_id,
        "branches": result_branches,
        "suite": suite,
        "suites": suites,
        "value": value,
        "MAX_DIFF_SIZE": MAX_DIFF_SIZE,
        "todo": [
            {
                'package': entry['package'],
                'id': entry['id'],
                'branches': [rb[0] for rb in entry['result_branches']],
                'value': entry['value']
            } for entry in entries
        ],
    }
    return await render_template_for_request("review.html", request, kwargs)


async def store_review(conn, run_id, status, comment, reviewer):
    async with conn.transaction():
        if status != 'abstained':
            await conn.execute(
                "UPDATE run SET review_status = $1, review_comment = $2 WHERE id = $3",
                status,
                comment,
                run_id,
            )
        await conn.execute(
            "INSERT INTO review (run_id, comment, reviewer, review_status) VALUES "
            " ($1, $2, $3, $4)", run_id, comment, reviewer, status)
