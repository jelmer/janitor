#!/usr/bin/python3

from typing import Optional, Dict, List

import asyncpg


async def get_proposals_with_run(
    conn: asyncpg.Connection, suite: Optional[str]
):
    query = """
SELECT
    DISTINCT ON (merge_proposal.url)
    run.package AS package,
    run.suite AS suite,
    merge_proposal.url AS url,
    merge_proposal.status AS status
FROM
    merge_proposal
LEFT JOIN new_result_branch ON new_result_branch.revision = merge_proposal.revision
LEFT JOIN run ON run.id = new_result_branch.run_id
"""
    args = []
    if suite:
        args.append(suite)
        query += """
WHERE suite = $1
"""
    query += """
ORDER BY merge_proposal.url, run.finish_time DESC
"""
    return await conn.fetch(query, *args)


async def write_merge_proposals(db, suite):
    async with db.acquire() as conn:
        proposals_by_status: Dict[str, List[asyncpg.Record]] = {}
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
        "rejected_proposals": proposals_by_status.get("rejected", []),
        "abandoned_proposals": proposals_by_status.get("abandoned", []),
    }


async def get_proposal_with_run(conn: asyncpg.Connection, url: str):
    query = """
SELECT
    run.package AS package,
    run.suite AS suite,
    merge_proposal.url AS url,
    merge_proposal.status AS status,
    merge_proposal.merged_at AS merged_at,
    merge_proposal.merged_by AS merged_by,
    merge_proposal.last_scanned AS last_scanned,
    merge_proposal.can_be_merged AS can_be_merged
FROM
    merge_proposal
LEFT JOIN new_result_branch ON new_result_branch.revision = merge_proposal.revision
LEFT JOIN run ON run.id = new_result_branch.run_id
WHERE url = $1
"""
    return await conn.fetchrow(query, url)


async def get_publishes(conn, url):
    return await conn.fetch("""
SELECT * FROM publish WHERE merge_proposal_url = $1
ORDER BY timestamp ASC
""", url)


async def write_merge_proposal(db, url):
    async with db.acquire() as conn:
        proposal = await get_proposal_with_run(conn, url)

        publishes = await get_publishes(conn, url)

    return {
        "proposal": proposal,
        "publishes": publishes,
    }
