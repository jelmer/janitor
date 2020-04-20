#!/usr/bin/python3

from janitor.site import env


async def iter_publish_history(conn, limit=None):
    query = """
SELECT
    publish.timestamp, publish.package, publish.branch_name,
    publish.main_branch_revision, publish.revision, publish.mode,
    publish.merge_proposal_url, publish.result_code, publish.description,
    package.vcs_browse
FROM
    publish
JOIN package ON publish.package = package.name
ORDER BY timestamp DESC
"""
    if limit:
        query += " LIMIT %d" % limit
    for row in await conn.fetch(query):
        yield row


async def write_history(conn, limit=None):
    template = env.get_template('publish-history.html')
    return await template.render_async(
        count=limit,
        history=iter_publish_history(conn, limit=limit))
