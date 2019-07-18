#!/usr/bin/python3

import argparse
import asyncio
import sys

from janitor import state
from janitor.site import env, get_run_diff, highlight_diff

from silver_platter.debian.lintian import (
    available_lintian_fixers,
    )


SUITE = 'lintian-fixes'


async def generate_pkg_file(package):
    try:
        (package, maintainer_email, uploader_emails, vcs_url) = list(
            await state.iter_packages(package=package))[0]
    except IndexError:
        raise KeyError(package)
    # TODO(jelmer): Filter out proposals not for this suite.
    merge_proposals = [
        (url, status)
        for (package, url, status, revision) in
        await state.iter_proposals(package)]
    run = await state.get_last_success(package, SUITE)
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
    candidate = await state.get_candidate(package, SUITE)
    if candidate is not None:
        candidate_command, candidate_context, candidate_value = candidate
    else:
        candidate_context = None
        candidate_value = None
    previous_runs = [x async for x in state.iter_previous_runs(package, SUITE)]
    def show_diff():
        diff = get_run_diff(run)
        if diff is None:
            return None
        return diff.decode('utf-8', 'replace')
    kwargs = {
        'package': package,
        'merge_proposals': merge_proposals,
        'maintainer_email': maintainer_email,
        'uploader_emails': uploader_emails,
        'vcs_url': vcs_url,
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
        'candidate_tags': candidate_context.split(' ') if candidate_context else None,
        'candidate_value': candidate_value,
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
    candidates = [(package, context.split(' '), value) for
                  (package, suite, command, context, value) in
                  await state.iter_candidates('lintian-fixes')]
    return await template.render_async(
        supported_tags=supported_tags, candidates=candidates)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(prog='report-lintian-fixes-pkg')
    parser.add_argument("package")
    args = parser.parse_args()

    loop = asyncio.get_event_loop()
    sys.stdout.write(loop.run_until_complete(generate_pkg_file(args.package)))
