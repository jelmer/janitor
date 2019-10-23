#!/usr/bin/python3

from janitor import state
from janitor.site import env


async def write_history(conn, limit=None):
    template = env.get_template('publish-history.html')
    return await template.render_async(
        count=limit,
        history=state.iter_publish_history(conn, limit=limit))
