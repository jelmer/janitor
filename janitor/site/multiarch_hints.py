#!/usr/bin/python3

import asyncpg
from . import state
from .common import generate_pkg_context
from janitor.site import (
    env,
    )


SUITE = 'multiarch-fixes'


async def generate_pkg_file(db, config, policy, client, differ_url,
                            publisher_url, package, run_id=None):
    return await generate_pkg_context(
        db, config, SUITE, policy, client, differ_url, publisher_url,
        package, run_id=run_id)


async def render_start():
    return {'SUITE': SUITE}


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
    return {'hints': hints}


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
    async with db.acquire() as conn:
        packages = list(await iter_last_successes_by_hint(conn, hint))
    return {'hint': hint, 'packages': packages}


async def generate_candidates(db):
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
    return {'candidates': candidates}


async def generate_stats(db):
    async with db.acquire() as conn:
        hints_per_run = {(c or 0): nr for (c, nr) in await conn.fetch("""\
select json_array_length(result->'applied-hints'), count(*) from run
where result_code = 'success' and suite = 'multiarch-fixes' group by 1
""")}
        per_kind = {h: nr for (h, nr) in await conn.fetch("""\
select split_part(link::text, '#', 2), count(*) from
multiarch_hints group by 1
""")}

        absorbed_per_kind = {h: nr for (h, nr) in await conn.fetch("""\
select split_part(link::text, '#', 2), count(*) from
absorbed_multiarch_hints group by 1
""")}
    return {
        'hints_per_run': hints_per_run,
        'per_kind': per_kind,
        'absorbed_per_kind': absorbed_per_kind}
