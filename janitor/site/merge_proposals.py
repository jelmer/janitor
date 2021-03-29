#!/usr/bin/python3

from typing import Optional

import asyncpg


async def get_proposals_with_run(
    conn: asyncpg.Connection, suite: str
):
    query = """
SELECT
    DISTINCT ON (merge_proposal.url)
    run.package AS package,
    run.suite AS suite
    merge_proposal.url AS url,
    merge_proposal.status AS status
FROM
    merge_proposal
LEFT JOIN new_result_branch ON new_result_branch.revision = merge_proposal.revision
LEFT JOIN run ON run.id = new_result_branch.run_id
WHERE suite = $1
ORDER BY merge_proposal.url, run.finish_time DESC
"""
    return await conn.fetch(query, suite)


async def write_merge_proposals(db, suite):
    async with db.acquire() as conn:
        proposals_by_status = {}
        for row in await get_proposals_with_run(conn, suite=suite):
            proposals_by_status.setdefault(row['status'], []).append(row)

    merged = proposals_by_status.get("merged", []) + proposals_by_status.get(
        "applied", []
    )
    return {
        "suite": suite,
        "open_proposals": proposals_by_status.get("open", []),
        "merged_proposals": merged,
        "closed_proposals": proposals_by_status.get("closed", []),
    }
