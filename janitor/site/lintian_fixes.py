#!/usr/bin/python3

from .common import generate_pkg_context
from silver_platter.debian.lintian import (
    available_lintian_fixers,
    )

from . import env
from .. import state


SUITE = 'lintian-fixes'


async def generate_pkg_file(db, policy, client, archiver_url, publisher_url,
                            package, run_id=None):
    kwargs = await generate_pkg_context(
        db, SUITE, policy, client, archiver_url,
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


async def generate_tag_list(conn):
    tags = sorted(await state.iter_lintian_tags(conn))
    template = env.get_template('lintian-fixes-tag-list.html')
    return await template.render_async(tags=tags)


async def generate_tag_page(db, tag):
    template = env.get_template('lintian-fixes-tag.html')
    async with db.acquire() as conn:
        packages = list(await state.iter_last_successes_by_lintian_tag(
            conn, tag))
    return await template.render_async(tag=tag, packages=packages)


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


async def generate_failing_fixer(db, fixer):
    template = env.get_template('lintian-fixes-failed.html')
    async with db.acquire() as conn:
        failures = await state.iter_lintian_brush_fixer_failures(
            conn, fixer)
    return await template.render_async(failures=failures, fixer=fixer)


async def generate_failing_fixers_list(db):
    template = env.get_template('lintian-fixes-failed-list.html')
    async with db.acquire() as conn:
        fixers = await state.iter_failed_lintian_fixers(conn)
    return await template.render_async(fixers=fixers)


async def generate_regressions_list(db):
    template = env.get_template('lintian-fixes-regressions.html')
    async with db.acquire() as conn:
        packages = await state.iter_lintian_fixes_regressions(conn)
    return await template.render_async(packages=packages)


async def generate_stats(db):
    template = env.get_template('lintian-fixes-stats.html')
    async with db.acquire() as conn:
        by_tag = await state.iter_lintian_fixes_counts(conn)
    return await template.render_async(by_tag=by_tag)


async def render_start():
    template = env.get_template('lintian-fixes-start.html')
    import lintian_brush
    from silver_platter.debian.lintian import DEFAULT_ADDON_FIXERS
    return await template.render_async(
        {'lintian_brush': lintian_brush,
         'ADDON_FIXERS': DEFAULT_ADDON_FIXERS})
