#!/usr/bin/python3

from aiohttp import web
import asyncpg
from functools import partial
from janitor import state
from . import tracker_url


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
        query = """
SELECT
  candidate.package AS package,
  candidate.suite AS suite,
  candidate.context AS context,
  candidate.value AS value,
  candidate.success_chance AS success_chance,
  package.archive_version AS archive_version
FROM candidate
INNER JOIN package on package.name = candidate.package
WHERE NOT package.removed AND suite = $1
"""
        candidates = await conn.fetch(query, suite)
    candidates.sort(key=lambda row: row['package'])
    return {"candidates": candidates, "suite": suite}
