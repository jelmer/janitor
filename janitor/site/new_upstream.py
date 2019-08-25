#!/usr/bin/python3

import argparse
import asyncio
import sys

from janitor import state
from janitor.build import (
    changes_filename,
)

from janitor.site import (
    changes_get_binaries,
    env,
    get_build_architecture,
    open_changes_file,
)


async def generate_pkg_file(package, suite):
    try:
        package = await state.get_package(package)
    except IndexError:
        raise KeyError(package)
    merge_proposals = [
        (url, status)
        for (unused_package, url, status) in
        await state.iter_proposals(package.name, suite=suite)]
    run = await state.get_last_unmerged_success(package.name, suite)
    candidate = await state.get_candidate(package.name, suite)
    if candidate is not None:
        candidate_command, candidate_context, candidate_value = candidate
    else:
        candidate_context = None
        candidate_value = None
    if not run:
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
    previous_runs = state.iter_previous_runs(package.name, suite)
    (queue_position, queue_wait_time) = await state.get_queue_position(
        package.name, suite)
    kwargs = {
        'package': package.name,
        'merge_proposals': merge_proposals,
        'maintainer_email': package.maintainer_email,
        'uploader_emails': package.uploader_emails,
        'vcs_url': package.branch_url,
        "vcs_browse": package.vcs_browse,
        'command': command,
        'build_version': build_version,
        'result_code': result_code,
        'context': context,
        'start_time': start_time,
        'finish_time': finish_time,
        'run_id': run_id,
        'result': result,
        'suite': suite,
        'candidate_version': candidate_context,
        'candidate_value': candidate_value,
        'previous_runs': previous_runs,
        'branch_name': branch_name,
        'branch_url': branch_url,
        'run': run,
        'queue_position': queue_position,
        'queue_wait_time': queue_wait_time,
        }
    if run and run.build_version:
        kwargs['changes_name'] = changes_filename(
            run.package, run.build_version,
            get_build_architecture())
        try:
            changes_file = open_changes_file(run, kwargs['changes_name'])
        except FileNotFoundError:
            pass
        else:
            kwargs['binary_packages'] = []
            for binary in changes_get_binaries(changes_file):
                kwargs['binary_packages'].append(binary)
    else:
        kwargs['changes_name'] = None

    template = env.get_template('new-upstream-package.html')
    return await template.render_async(**kwargs)


async def generate_candidates(suite):
    template = env.get_template('new-upstream-candidates.html')
    candidates = [(package.name, context, value) for
                  (package, suite, command, context, value) in
                  await state.iter_candidates(suite=suite)]
    candidates.sort()
    return await template.render_async(candidates=candidates, suite=suite)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(prog='report-new-upstream-pkg')
    parser.add_argument("package")
    args = parser.parse_args()

    loop = asyncio.get_event_loop()
    sys.stdout.write(loop.run_until_complete(generate_pkg_file(args.package)))
