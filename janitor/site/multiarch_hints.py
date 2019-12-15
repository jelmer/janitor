#!/usr/bin/python3

from janitor.site import (
    env,
    )


SUITE = 'multiarch-fixes'


async def render_start():
    template = env.get_template('multiarch-fixes-start.html')
    return await template.render_async()
