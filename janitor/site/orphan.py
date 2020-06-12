#!/usr/bin/python3

import asyncpg
from . import state
from .common import generate_pkg_context
from janitor.site import (
    env,
    )


SUITE = 'orphan'


async def render_start():
    template = env.get_template('orphan-start.html')
    return await template.render_async()
