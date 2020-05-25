#!/usr/bin/python3

import asyncpg
from . import state
from .common import generate_pkg_context
from janitor.site import (
    env,
    )


SUITE = 'multiarch-fixes'


async def generate_pkg_file(db, policy, client, archiver_url, publisher_url,
                            package, run_id=None):
    kwargs = await generate_pkg_context(
        db, SUITE, policy, client, archiver_url, publisher_url, package,
        run_id=run_id)
    template = env.get_template('multiarch-fixes-package.html')
    return await template.render_async(**kwargs)


async def render_start():
    template = env.get_template('multiarch-fixes-start.html')
    return await template.render_async()


async def iter_hint_links(conn):
    return await conn.fetch("""
select hint, count(hint) from (
    select
        json_array_elements(
          result->'applied-hints')->'link' #>> '{}' as hint
    from
      last_runs
    where
      suite = 'multiarch-fixes'
   ) as bypackage group by 1 order by 2
 desc
""")


async def generate_hint_list(conn: asyncpg.Connection):
    hint_links = await iter_hint_links(conn)
    hints = [(link.split('#')[-1], count) for link, count in hint_links]
    template = env.get_template('multiarch-fixes-hint-list.html')
    return await template.render_async(hints=hints)


async def iter_last_successes_by_hint(conn: asyncpg.Connection, hint: str):
    return await conn.fetch("""
select distinct on (package) * from (
select
  package,
  command,
  build_version,
  result_code,
  context,
  start_time,
  id,
  json_array_elements(
     result->'applied-hints')->'link' #>> '{}' as hint
from
  run
where
  build_distribution  = 'multiarch-fixes' and
  result_code = 'success'
) as package where hint like $1 order by package, start_time desc
""", '%#' + hint)


async def generate_hint_page(db, hint):
    template = env.get_template('multiarch-fixes-hint.html')
    async with db.acquire() as conn:
        packages = list(await iter_last_successes_by_hint(conn, hint))
    return await template.render_async(hint=hint, packages=packages)


async def generate_candidates(db):
    template = env.get_template('multiarch-fixes-candidates.html')
    candidates = []
    async with db.acquire() as conn:
        for (package, suite, context, value,
             success_chance) in await state.iter_candidates(conn, suite=SUITE):
            hints = {}
            for h in context.split(' '):
                hints.setdefault(h, 0)
                hints[h] += 1
            candidates.append((package.name, list(hints.items()), value))
        candidates.sort(key=lambda x: x[2], reverse=True)
    return await template.render_async(candidates=candidates)
