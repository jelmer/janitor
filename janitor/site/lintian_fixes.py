#!/usr/bin/python3

import argparse
import asyncio
import sys

from janitor import state
from janitor.site import env, format_duration, get_run_diff, highlight_diff


async def generate_pkg_file(package):
    suite = 'lintian-fixes'
    try:
        (package, maintainer_email, vcs_url) = list(
            await state.iter_packages(package=package))[0]
    except IndexError:
        raise KeyError(package)
    # TODO(jelmer): Filter out proposals not for this suite.
    merge_proposals = [
        (url, status)
        for (package, url, status, revision) in
        await state.iter_proposals(package)]
    run = await state.get_last_success(package, suite)
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
    previous_runs = [x async for x in state.iter_previous_runs(package, suite)]
    kwargs = {
        'package': package,
        'merge_proposals': merge_proposals,
        'maintainer_email': maintainer_email,
        'vcs_url': vcs_url,
        'command': command,
        'build_version': build_version,
        'result_code': result_code,
        'context': context,
        'start_time': start_time,
        'finish_time': finish_time,
        'run_id': run_id,
        'result': result,
        'suite': suite,
        'format_duration': format_duration,
        'show_diff': lambda: get_run_diff(run).decode('utf-8'),
        'highlight_diff': highlight_diff,
        'branch_name': branch_name,
        'previous_runs': previous_runs,
        'run': run,
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


if __name__ == '__main__':
    parser = argparse.ArgumentParser(prog='report-lintian-fixes-pkg')
    parser.add_argument("package")
    args = parser.parse_args()

    loop = asyncio.get_event_loop()
    sys.stdout.write(loop.run_until_complete(generate_pkg_file(args.package)))
