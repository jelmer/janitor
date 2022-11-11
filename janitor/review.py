#!/usr/bin/python3
# Copyright (C) 2018 Jelmer Vernooij <jelmer@jelmer.uk>
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

import asyncpg

from typing import Optional, List, Any

from .schedule import do_schedule


async def store_review(conn, run_id, status, comment, reviewer, is_qa_reviewer):
    async with conn.transaction():
        if status == "reschedule":
            status = "rejected"

            run = await conn.fetchrow(
                'SELECT package, suite, codebase FROM run WHERE id = $1', run_id)
            await do_schedule(
                conn,
                package=run['package'],
                campaign=run['suite'],
                refresh=True,
                requestor="reviewer (%s)" % reviewer,
                bucket="default",
                codebase=run['codebase']
            )

        if status != 'abstained' and is_qa_reviewer:
            await conn.execute(
                "UPDATE run SET review_status = $1 WHERE id = $2",
                status, run_id)
        await conn.execute(
            "INSERT INTO review (run_id, comment, reviewer, review_status) VALUES "
            " ($1, $2, $3, $4) ON CONFLICT (run_id, reviewer) "
            "DO UPDATE SET review_status = EXCLUDED.review_status, comment = EXCLUDED.comment, "
            "reviewed_at = NOW()", run_id, comment, reviewer, status)


async def iter_needs_review(
        conn: asyncpg.Connection,
        campaigns: Optional[List[str]] = None,
        limit: Optional[int] = None,
        publishable_only: bool = False,
        required_only: Optional[bool] = None,
        reviewer: Optional[str] = None):
    args: List[Any] = []
    query = """
SELECT id, command, package, suite, vcs_type, result_branches, main_branch_revision, value, finish_time FROM publish_ready
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
