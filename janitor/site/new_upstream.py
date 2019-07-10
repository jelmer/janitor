#!/usr/bin/python3

import argparse
import asyncio
import sys

from janitor import state
from janitor.build import (
    changes_filename,
    get_build_architecture,
)

from janitor.site import (
    changes_get_binaries,
    env,
    format_duration,
    get_changes_path,
)


async def generate_pkg_file(package, suite):
    try:
        (package, maintainer_email, vcs_url) = list(
            await state.iter_packages(package=package))[0]
    except IndexError:
        raise KeyError(package)
    # TODO(jelmer): Filter out proposals not for this suite.
    merge_proposals = [
        (url, status)
        for (package, url, status) in await state.iter_proposals(package)]
    run = await state.get_last_success(package, suite)
    if not run:
        command = None
        build_version = None
        result_code = None
        context = None
        start_time = None
        finish_time = None
        run_id = None
        result = None
    else:
        command = run.command
        build_version = run.build_version
        result_code = run.result_code
        context = run.context
        start_time = run.times[0]
        finish_time = run.times[1]
        run_id = run.id
        result = run.result
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
        'previous_runs': previous_runs,
        }
    if run and run.build_version:
        kwargs['changes_name'] = changes_filename(
            run.package, run.build_version,
            get_build_architecture())
        changes_path = get_changes_path(run, kwargs['changes_name'])
        kwargs['binary_packages'] = []
        if changes_path:
            for binary in changes_get_binaries(changes_path):
                kwargs['binary_packages'].append(binary)
    else:
        kwargs['changes_name'] = None

    template = env.get_template('new-upstream-package.html')
    return await template.render_async(**kwargs)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(prog='report-new-upstream-pkg')
    parser.add_argument("package")
    args = parser.parse_args()

    loop = asyncio.get_event_loop()
    sys.stdout.write(loop.run_until_complete(generate_pkg_file(args.package)))
