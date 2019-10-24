#!/usr/bin/python3

from aiohttp import ClientConnectorError
import urllib.parse

from janitor import state
from janitor.policy import apply_policy
from janitor.site import (
    env,
    get_vcs_type,
    highlight_diff,
    )

from silver_platter.debian.lintian import (
    available_lintian_fixers,
    )


SUITE = 'lintian-fixes'


async def generate_pkg_file(db, policy, client, publisher_url, package, run_id=None):
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
        (publish_policy, unused_changelog_policy, commiter) = apply_policy(
            policy, SUITE, package.name, package.maintainer_email, package.uploader_emails)
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
        candidate = await state.get_candidate(conn, package.name, SUITE)
        if candidate is not None:
            candidate_command, candidate_context, candidate_value = candidate
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

    async def vcs_type():
        return await get_vcs_type(publisher_url, run.package)

    kwargs = {
        'package': package.name,
        'merge_proposals': merge_proposals,
        'maintainer_email': package.maintainer_email,
        'uploader_emails': package.uploader_emails,
        'removed': package.removed,
        'vcs_url': package.vcs_url,
        'vcs_type': vcs_type,
        'vcs_browse': package.vcs_browse,
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
        'highlight_diff': highlight_diff,
        'branch_name': branch_name,
        'previous_runs': previous_runs,
        'run': run,
        'candidate_context': candidate_context,
        'candidate_tags':
            candidate_context.split(' ') if candidate_context else None,
        'candidate_value': candidate_value,
        'branch_url': branch_url,
        'queue_position': queue_position,
        'queue_wait_time': queue_wait_time,
        'publish_policy': publish_policy,
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
                      (package, suite, command, context, value) in
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
            candidates[row[0].name] = row[3].split(' ')
        runs = {}
        async for run in state.iter_last_unabsorbed_runs(
                conn, suite=SUITE, packages=packages):
            runs[run.package] = run

    by_package = {}
    for package in packages:
        run = runs.get(package)
        fixed = set()
        if run and run.result:
            applied = run.result.get('applied')
            if isinstance(applied, dict):
                applied = [applied]
            for applied in applied:
                for tag in applied.get('fixed_lintian_tags', []):
                    fixed.add(tag)
        open_proposal = open_proposals.get(package)
        package_candidates = set(candidates.get(package, []))
        if open_proposal:
            status = 'proposal'
        elif run and run.result and run.result_code in (
                'success', 'nothing-to-do'):
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
            open_proposal, status)

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
