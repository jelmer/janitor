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


async def generate_candidates(db):
    template = env.get_template('orphan-candidates.html')
    candidates = []
    async with db.acquire() as conn:
        for (package, suite, context, value,
             success_chance) in await state.iter_candidates(conn, suite=SUITE):
            candidates.append((package.name, context, value))
        candidates.sort(key=lambda x: x[2], reverse=True)
    return await template.render_async(candidates=candidates)


async def generate_pkg_file(db, policy, client, archiver_url, publisher_url,
                            package, run_id=None):
    kwargs = await generate_pkg_context(
        db, SUITE, policy, client, archiver_url, publisher_url, package,
        run_id=run_id)
    template = env.get_template('orphan-package.html')
    return await template.render_async(**kwargs)
