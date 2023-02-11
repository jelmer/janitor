#!/usr/bin/python3

from aiohttp import web


async def iter_publish_history(conn, limit=None):
    query = """
SELECT
    publish.id, publish.timestamp, publish.branch_name,
    publish.mode, publish.merge_proposal_url, publish.result_code,
    publish.description, codebase.web_url, codebase.name AS codebase
FROM
    publish
JOIN codebase ON codebase.branch_url = publish.target_branch_url AND codebase.subpath = publish.subpath
ORDER BY timestamp DESC
"""
    if limit:
        query += " LIMIT %d" % limit
    return await conn.fetch(query)


async def write_history(conn, limit=None):
    return {
        "count": limit,
        "history": await iter_publish_history(conn, limit=limit),
    }


async def get_publish(conn, id):
    query = """
SELECT
    publish.id AS id,
    publish.timestamp AS timestamp,
    publish.branch_name AS branch_name,
    publish.mode AS mode,
    publish.merge_proposal_url AS merge_proposal_url,
    publish.result_code AS result_code,
    publish.description AS description,
    codebase.web_url AS vcs_browse,
    codebase.name AS codebase
FROM
    publish
JOIN codebase ON codebase.branch_url = publish.target_branch_url AND codebase.subpath = publish.subpath
WHERE id = $1
"""
    return await conn.fetchrow(query, id)


async def write_publish(conn, id):
    publish = await get_publish(conn, id)
    if publish is None:
        raise web.HTTPNotFound(text="no such publish: %s" % id)
    return {
        "publish": publish
    }
