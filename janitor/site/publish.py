#!/usr/bin/python3

from janitor import state
from janitor.site import env


async def get_history(limit):
    return [run async for run in state.iter_publish_history(limit=limit)]


async def write_history(limit=None):
    template = env.get_template('publish-history.html')
    return await template.render_async(
        count=limit,
        history=await get_history(limit))
