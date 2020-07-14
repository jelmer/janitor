#!/usr/bin/python3

from . import state, html_template
from .common import generate_pkg_context


SUITE = 'orphan'


@html_template('orphan-start.html')
async def render_start():
    return {}


@html_template(
    'orphan-candidates.html', headers={'Cache-Control': 'max-age=3600'})
async def generate_candidates(db):
    candidates = []
    async with db.acquire() as conn:
        for (package, suite, context, value,
             success_chance) in await state.iter_candidates(conn, suite=SUITE):
            candidates.append((package.name, context, value))
        candidates.sort(key=lambda x: x[2], reverse=True)
    return {'candidates': candidates}


@html_template('orphan-package.html', headers={'Cache-Control': 'max-age=600'})
async def generate_pkg_file(db, config, policy, client, archiver_url,
                            publisher_url, package, run_id=None):
    return await generate_pkg_context(
        db, config, SUITE, policy, client, archiver_url, publisher_url,
        package, run_id=run_id)
