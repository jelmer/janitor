#!/usr/bin/python3

from aiohttp import web, ClientSession, ContentTypeError, ClientConnectorError
import argparse
import asyncio
import urllib.parse
import sys

from janitor import state
from janitor.site import env, highlight_diff

from silver_platter.debian.lintian import (
    available_lintian_fixers,
    )


SUITE = 'lintian-fixes'


async def generate_pkg_file(publisher_url, vcs_manager, package, run_id=None):
    try:
        package = await state.get_package(name=package)
    except IndexError:
        raise KeyError(package)
    if run_id is not None:
        run = state.get_run(run_id)
        if not run:
            raise KeyError(run_id)
        merge_proposals = []
    else:
        run = await state.get_last_unmerged_success(package.name, SUITE)
        merge_proposals = [
            (url, status) for (unused_package, url, status) in
            await state.iter_proposals(package.name, suite=SUITE)]
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
    candidate = await state.get_candidate(package.name, SUITE)
    if candidate is not None:
        candidate_command, candidate_context, candidate_value = candidate
    else:
        candidate_context = None
        candidate_value = None
    previous_runs = [
        x async for x in state.iter_previous_runs(package.name, SUITE)]

    async def show_diff():
        url = urllib.parse.urljoin(publisher_url, 'diff/%s' % run.id)
        async with ClientSession() as client:
            try:
                async with client.get(url) as resp:
                    if resp.status == 200:
                        return (await resp.read()).decode('utf-8', 'replace')
                    else:
                        return 'Unable to retrieve diff; error %d' % resp.status
            except ClientConnectorError as e:
                return 'Unable to retrieve diff; error %s' % e

    (queue_position, queue_wait_time) = await state.get_queue_position(
        package.name, SUITE)
    kwargs = {
        'package': package.name,
        'merge_proposals': merge_proposals,
        'maintainer_email': package.maintainer_email,
        'uploader_emails': package.uploader_emails,
        'vcs_url': package.vcs_url,
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
        }
    template = env.get_template('lintian-fixes-package.html')
    return await template.render_async(**kwargs)


async def generate_tag_list():
    tags = sorted(await state.iter_lintian_tags())
    template = env.get_template('lintian-fixes-tag-list.html')
    return await template.render_async(tags=tags)


async def generate_tag_page(tag):
    template = env.get_template('lintian-fixes-tag.html')
    packages = list(await state.iter_last_successes_by_lintian_tag(tag))
    return await template.render_async(tag=tag, packages=packages)


async def generate_candidates():
    template = env.get_template('lintian-fixes-candidates.html')
    supported_tags = set()
    for fixer in available_lintian_fixers():
        supported_tags.update(fixer.lintian_tags)
    candidates = [(package.name, context.split(' '), value) for
                  (package, suite, command, context, value) in
                  await state.iter_candidates(suite=SUITE)]
    candidates.sort()
    return await template.render_async(
        supported_tags=supported_tags, candidates=candidates)


async def generate_developer_page(developer):
    template = env.get_template('lintian-fixes-developer.html')
    packages = [p for p, removed in
                await state.iter_packages_by_maintainer(developer)
                if not removed]
    proposals = {}
    for package, url, status in await state.iter_proposals(packages, SUITE):
        if status == 'open':
            proposals[package] = url
    candidates = {}
    for row in await state.iter_candidates(packages=packages, suite=SUITE):
        candidates[row[0].name] = row[3].split(' ')
    nothing_to_do = []
    errors = []
    ready_changes = []
    runs = {}
    merge_proposals = []
    async for run in state.iter_last_unmerged_successes(
            suite=SUITE, packages=packages):
        runs[run.package] = run
        if run.package in candidates:
            del candidates[run.package]
        if run.result_code not in ('success', 'nothing-to-do'):
            errors.append(run)
        else:
            if run.result and run.result.get('applied'):
                if proposals.get(run.package):
                    merge_proposals.append((proposals[run.package], run))
                else:
                    ready_changes.append(run)
            else:
                nothing_to_do.append(run)

    return await template.render_async(
        developer=developer, packages=packages,
        candidates=list(candidates.items()),
        runs=runs, nothing_to_do=nothing_to_do, errors=errors,
        ready_changes=ready_changes, merge_proposals=merge_proposals)


async def generate_failing_fixer(fixer):
    template = env.get_template('lintian-fixes-failed.html')
    failures = await state.iter_lintian_brush_fixer_failures(
        fixer)
    return await template.render_async(failures=failures, fixer=fixer)


async def generate_failing_fixers_list():
    template = env.get_template('lintian-fixes-failed-list.html')
    fixers = await state.iter_failed_lintian_fixers()
    return await template.render_async(fixers=fixers)


if __name__ == '__main__':
    from janitor.vcs import LocalVcsManager
    import os
    parser = argparse.ArgumentParser(prog='report-lintian-fixes-pkg')
    parser.add_argument("package")
    args = parser.parse_args()

    vcs_manager = LocalVcsManager(
        os.path.join(os.path.dirname(__file__), '..', '..', 'vcs'))
    loop = asyncio.get_event_loop()
    sys.stdout.write(loop.run_until_complete(generate_pkg_file(
        vcs_manager, args.package)))
