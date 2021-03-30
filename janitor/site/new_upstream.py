#!/usr/bin/python3

from aiohttp import web
import asyncpg
from functools import partial
from janitor import state
from . import tracker_url
from .common import iter_candidates


async def get_proposals(conn: asyncpg.Connection, package, suite):
    return await conn.fetch("""
SELECT
    DISTINCT ON (merge_proposal.url)
    merge_proposal.url, merge_proposal.status
FROM
    merge_proposal
LEFT JOIN run
ON merge_proposal.revision = run.revision AND run.result_code = 'success'
WHERE merge_proposal.package = $1 AND suite = $2
ORDER BY merge_proposal.url, run.finish_time DESC
""", package, suite)


async def generate_candidates(db, suite):
    async with db.acquire() as conn:
        candidates = [
            (row['package'], row['context'], row['value'], row['success_chance'])
            for row in await iter_candidates(conn, suite=suite)
        ]
    candidates.sort()
    return {"candidates": candidates, "suite": suite}
