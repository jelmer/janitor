#!/usr/bin/python3

from aiohttp import ClientConnectorError
import urllib.parse

from janitor import state
from janitor.site import (
    env,
    get_debdiff,
    get_vcs_type,
    DebdiffRetrievalError,
    )

from silver_platter.debian.lintian import (
    available_lintian_fixers,
    )


SUITE = 'lintian-fixes'


async def generate_pkg_file(db, policy, client, archiver_url, publisher_url,
                            package, run_id=None):
    async with db.acquire() as conn:
        package = await state.get_package(conn, name=package)
        if package is None:
            raise KeyError(package)
        if run_id is not None:
            run = await state.get_run(conn, run_id)
            if not run:
                raise KeyError(run_id)
            merge_proposals = []
        else:
            run = await state.get_last_unabsorbed_run(
                conn, package.name, SUITE)
            merge_proposals = [
                (url, status) for (unused_package, url, status) in
                await state.iter_proposals(conn, package.name, suite=SUITE)]
        (publish_policy, changelog_policy,
         unused_command) = await state.get_publish_policy(
             conn, package.name, SUITE)
        if run is None:
            # No runs recorded
            command = None
            build_version = None
            result_code = None
            context = None
            start_time = None
            finish_time = None
            run_id = None
            result = None
            branch_name = None
            branch_url = None
            fixed_tags = set()
            unchanged_run = None
        else:
            command = run.command
            build_version = run.build_version
            result_code = run.result_code
            context = run.context
            start_time = run.times[0]
            finish_time = run.times[1]
            run_id = run.id
            result = run.result
            branch_name = run.branch_name
            branch_url = run.branch_url
            if run.result:
                applied = run.result.get('applied', [])
            else:
                applied = []
            fixed_tags = set()
            if isinstance(applied, dict):
                applied = [applied]
            for applied in applied:
                for tag in applied.get('fixed_lintian_tags', []):
                    fixed_tags.add(tag)
            if run.main_branch_revision:
                unchanged_run = await state.get_unchanged_run(
                    conn, run.main_branch_revision)
            else:
                unchanged_run = None

        candidate = await state.get_candidate(conn, package.name, SUITE)
        if candidate is not None:
            (candidate_context, candidate_value,
             candidate_success_chance) = candidate
        else:
            candidate_context = None
            candidate_value = None
        previous_runs = [
            x async for x in
            state.iter_previous_runs(conn, package.name, SUITE)]
        (queue_position, queue_wait_time) = await state.get_queue_position(
            conn, SUITE, package.name)

    async def show_diff():
        if not run.revision or run.revision == run.main_branch_revision:
            return ''
        url = urllib.parse.urljoin(publisher_url, 'diff/%s' % run.id)
        try:
            async with client.get(url) as resp:
                if resp.status == 200:
                    return (await resp.read()).decode('utf-8', 'replace')
                else:
                    return (
                        'Unable to retrieve diff; error %d' % resp.status)
        except ClientConnectorError as e:
            return 'Unable to retrieve diff; error %s' % e

    async def show_debdiff():
        if not run.build_version or not run.main_branch_revision:
            return ''
        if not unchanged_run or not unchanged_run.build_version:
            return ''
        try:
            debdiff = await get_debdiff(
                client, archiver_url, run, unchanged_run,
                filter_boring=True)
            return debdiff.decode('utf-8', 'replace')
        except FileNotFoundError:
            return ''
        except DebdiffRetrievalError as e:
            return 'Error retrieving debdiff: %s' % e

    async def vcs_type():
        return await get_vcs_type(client, publisher_url, run.package)

    kwargs = {
        'package': package.name,
        'unchanged_run': unchanged_run,
        'merge_proposals': merge_proposals,
        'maintainer_email': package.maintainer_email,
        'uploader_emails': package.uploader_emails,
        'removed': package.removed,
        'vcs_url': package.vcs_url,
        'vcs_type': vcs_type,
        'vcs_browse': package.vcs_browse,
        'vcswatch_version': package.vcswatch_version,
        'command': command,
        'build_version': build_version,
        'result_code': result_code,
        'context': context,
        'start_time': start_time,
        'finish_time': finish_time,
        'run_id': run_id,
        'result': result,
        'suite': SUITE,
        'show_diff': show_diff,
        'show_debdiff': show_debdiff,
        'branch_name': branch_name,
        'previous_runs': previous_runs,
        'run': run,
        'candidate_context': candidate_context,
        'candidate_success_chance': candidate_success_chance,
        'candidate_tags':
            set(candidate_context.split(' ')) if candidate_context else set(),
        'candidate_value': candidate_value,
        'branch_url': branch_url,
        'queue_position': queue_position,
        'queue_wait_time': queue_wait_time,
        'publish_policy': publish_policy,
        'fixed_tags': fixed_tags,
        'changelog_policy': changelog_policy,
        }
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
        by_tag = await state.iter_absorbed_lintian_fixes(conn)
    return await template.render_async(by_tag=by_tag)


async def render_start():
    template = env.get_template('lintian-fixes-start.html')
    import lintian_brush
    from silver_platter.debian.lintian import DEFAULT_ADDON_FIXERS
    return await template.render_async(
        {'lintian_brush': lintian_brush,
         'ADDON_FIXERS': DEFAULT_ADDON_FIXERS})
