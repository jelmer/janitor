#!/usr/bin/python3

import asyncpg
from typing import List, Dict
from .common import generate_pkg_context
from silver_platter.debian.lintian import (
    available_lintian_fixers,
    )
from lintian_brush import load_renamed_tags

from . import env
from .. import state


SUITE = 'lintian-fixes'

renamed_tags = load_renamed_tags()

async def generate_pkg_file(db, config, policy, client, archiver_url, publisher_url,
                            package, run_id=None):
    kwargs = await generate_pkg_context(
        db, config, SUITE, policy, client, archiver_url,
        publisher_url, package, run_id=run_id)
    run = kwargs['run']
    if run and run.result:
        applied = run.result.get('applied', [])
    else:
        applied = []
    fixed_tags = set()
    if isinstance(applied, dict):
        applied = [applied]
    for applied in applied:
        for tag in applied.get('fixed_lintian_tags', []):
            fixed_tags.add(tag)
    kwargs['fixed_tags'] = fixed_tags
    kwargs['candidate_tags'] = (
        set(kwargs['candidate_context'].split(' '))
        if kwargs['candidate_context'] else set())
    template = env.get_template('lintian-fixes-package.html')
    return await template.render_async(**kwargs)


async def iter_lintian_tags(conn):
    return await conn.fetch("""
select tag, count(tag) from (
    select
      json_array_elements(
        json_array_elements(
          result->'applied')->'fixed_lintian_tags') #>> '{}' as tag
    from
      last_runs
    where
      build_distribution = 'lintian-fixes'
   ) as bypackage group by 1 order by 2
 desc
""")


async def generate_tag_list(conn: asyncpg.Connection):
    tags = []
    oldnames = {}  # type: Dict[str, List[str]]
    for tag in await iter_lintian_tags(conn):
        try:
            newname = renamed_tags[tag]
        except KeyError:
            tags.append(tag)
        else:
            oldnames.setdefault(newname, []).append(tag)
    template = env.get_template('lintian-fixes-tag-list.html')
    tags.sort()
    return await template.render_async(tags=tags, oldnames=oldnames)


async def iter_last_successes_by_lintian_tag(
        conn: asyncpg.Connection, tags: List[str]):
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
  (json_array_elements(
     json_array_elements(
       result->'applied')->'fixed_lintian_tags') #>> '{}') as tag
from
  run
where
  build_distribution  = 'lintian-fixes' and
  result_code = 'success'
) as package where tag = ANY($1::text[]) order by package, start_time desc
""", tags)


async def generate_tag_page(db, tag):
    template = env.get_template('lintian-fixes-tag.html')
    oldnames = []
    for oldname, newname in renamed_tags.items():
        if newname == tag:
            oldnames.append(oldname)
    async with db.acquire() as conn:
        packages = list(await iter_last_successes_by_lintian_tag(
            conn, [tag] + oldnames))
    return await template.render_async(
        tag=tag, oldnames=oldnames, packages=packages)


async def generate_candidates(db):
    template = env.get_template('lintian-fixes-candidates.html')
    supported_tags = set()
    for fixer in available_lintian_fixers():
        supported_tags.update(fixer.lintian_tags)
    async with db.acquire() as conn:
        candidates = [(package.name, context.split(' '), value) for
                      (package, suite, context, value, success_chance) in
                      await state.iter_candidates(conn, suite=SUITE)]
        candidates.sort()
    return await template.render_async(
        supported_tags=supported_tags, candidates=candidates)


async def generate_developer_table_page(db, developer):
    template = env.get_template('lintian-fixes-developer-table.html')
    async with db.acquire() as conn:
        packages = [p for p, removed in
                    await state.iter_packages_by_maintainer(
                        conn, developer)
                    if not removed]
        open_proposals = {}
        for package, url, status in await state.iter_proposals(
                conn, packages, SUITE):
            if status == 'open':
                open_proposals[package] = url
        candidates = {}
        for row in await state.iter_candidates(
                conn, packages=packages, suite=SUITE):
            candidates[row[0].name] = row[2].split(' ')
        runs = {}
        async for run in state.iter_last_unabsorbed_runs(
                conn, suite=SUITE, packages=packages):
            runs[run.package] = run
        queue_data = {
            package: (position, duration)
            for (package, position, duration) in
            await state.get_queue_positions(conn, SUITE, packages)}

    by_package = {}
    for package in packages:
        run = runs.get(package)
        fixed = set()
        unfixed = set()
        if run and run.result:
            applied = run.result.get('applied')
            if isinstance(applied, dict):
                applied = [applied]
            for applied in applied:
                for tag in applied.get('fixed_lintian_tags', []):
                    fixed.add(tag)
        if run and run.instigated_context:
            for tag in run.instigated_context.split(' '):
                unfixed.add(tag)
        unfixed -= fixed
        open_proposal = open_proposals.get(package)
        package_candidates = set(candidates.get(package, []))
        if open_proposal:
            status = 'proposal'
        elif run and run.result and run.result_code in (
                'success', 'nothing-new-to-do'):
            status = 'unabsorbed'
        elif run and run.result_code != 'nothing-to-do':
            status = 'error'
        elif package_candidates:
            status = 'candidates'
        else:
            status = 'nothing-to-do'

        by_package[package] = (
            run,
            package_candidates,
            fixed,
            unfixed,
            open_proposal,
            status,
            queue_data.get(package, (None, None)))

    return await template.render_async(
        packages=packages, by_package=by_package, suite=SUITE,
        developer=developer)


async def iter_lintian_brush_fixer_failures(conn: asyncpg.Connection, fixer):
    query = """
select id, package, result->'failed'->$1 FROM last_runs
where
  suite = 'lintian-fixes' and (result->'failed')::jsonb?$1
"""
    return await conn.fetch(query, fixer)


async def generate_failing_fixer(db, fixer):
    async with db.acquire() as conn:
        failures = await iter_lintian_brush_fixer_failures(
            conn, fixer)
        return {'failures': failures, 'fixer': fixer}


async def iter_failed_lintian_fixers(conn):
    query = """
select json_object_keys((result->'failed')::json), count(*) from last_runs
where
  suite = 'lintian-fixes' and
  json_typeof((result->'failed')::json) = 'object' group by 1 order by 2 desc
"""
    return await conn.fetch(query)


async def generate_failing_fixers_list(db):
    template = env.get_template(
    async with db.acquire() as conn:
        fixers = await iter_failed_lintian_fixers(conn)
    return await template.render_async(fixers=fixers)


async def iter_lintian_fixes_regressions(conn):
    query = """
SELECT l.package, l.id, u.id, l.result_code FROM last_runs l
   INNER JOIN last_runs u ON l.main_branch_revision = u.main_branch_revision
   WHERE
    l.suite = 'lintian-fixes' AND
    u.suite = 'unchanged' AND
    l.result_code NOT IN ('success', 'nothing-to-do', 'nothing-new-to-do') AND
    u.result_code = 'success'
"""
    return await conn.fetch(query)


async def generate_regressions_list(db):
    template = env.get_template(
    async with db.acquire() as conn:
        packages = await iter_lintian_fixes_regressions(conn)
    return await template.render_async(packages=packages)


async def iter_lintian_fixes_counts(conn):
    per_tag = {}
    for (tag, absorbed, unabsorbed, total) in await conn.fetch("""
SELECT
   absorbed.tag,
   COALESCE(absorbed.cnt, 0),
   COALESCE(unabsorbed.cnt, 0),
   COALESCE(absorbed.cnt, 0)+COALESCE(unabsorbed.cnt, 0)
FROM (
    SELECT UNNEST(fixed_lintian_tags) AS tag, COUNT(*) AS cnt
    FROM absorbed_lintian_fixes group by 1 order by 2 desc
    ) AS absorbed
LEFT JOIN (
    SELECT UNNEST(fixed_lintian_tags) AS tag, COUNT(*) AS cnt
    FROM last_unabsorbed_lintian_fixes group by 1 order by 2 desc
    ) AS unabsorbed
ON absorbed.tag = unabsorbed.tag
"""):
        canonical_name = renamed_tags.get(tag, tag)
        per_tag.setdefault(canonical_name, (0, 0, 0))
        per_tag[canonical_name] = (
            per_tag[canonical_name][0] + absorbed,
            per_tag[canonical_name][1] + unabsorbed,
            per_tag[canonical_name][2] + total)
    entries = [
        (tag, absorbed, unabsorbed, total) for
        (tag, (absorbed, unabsorbed, total)) in per_tag.items()]
    entries.sort(key=lambda v: v[3], reverse=True)
    return entries


async def generate_stats(db):
    async with db.acquire() as conn:
        by_tag = await iter_lintian_fixes_counts(conn)
        tags_per_run = {c: nr for (c, nr) in await conn.fetch("""\
select coalesce(c, 0), count(*) from (
    select sum(array_length(fixed_lintian_tags, 1)) c
    from absorbed_lintian_fixes where suite = 'lintian-fixes' group by revision
) as p group by 1
""")}
        lintian_brush_versions = {
            (c or 'unknown'): nr for (c, nr) in await conn.fetch("""
select result#>>'{versions,lintian-brush}', count(*) from run
where result_code = 'success' and suite = 'lintian-fixes'
group by 1 order by 1 desc
""")}

    return {
        'by_tag': by_tag,
        'tags_per_run': tags_per_run,
        'lintian_brush_versions': lintian_brush_versions,
        }


async def render_start():
    import lintian_brush
    from silver_platter.debian.lintian import DEFAULT_ADDON_FIXERS
    return {
        'lintian_brush': lintian_brush,
        'ADDON_FIXERS': DEFAULT_ADDON_FIXERS}
