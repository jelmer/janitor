#!/usr/bin/python3


async def iter_publish_history(conn, limit=None):
    query = """
SELECT
    publish.id, publish.timestamp, publish.package, publish.branch_name,
    publish.mode, publish.merge_proposal_url, publish.result_code,
    publish.description, package.vcs_browse
FROM
    publish
JOIN package ON publish.package = package.name
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
    publish.package AS package,
    publish.branch_name AS branch_name,
    publish.mode AS mode,
    publish.merge_proposal_url AS merge_proposal_url,
    publish.result_code AS result_code,
    publish.description AS description,
    package.vcs_browse AS vcs_browse
FROM
    publish
JOIN package ON publish.package = package.name
WHERE id = $1
"""
    return await conn.fetchrow(query, id)


async def write_publish(conn, id):
    return {
        "publish": await get_publish(conn, id),
    }
