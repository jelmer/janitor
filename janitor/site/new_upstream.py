#!/usr/bin/python3

from functools import partial
from janitor import state
from . import tracker_url


async def generate_pkg_file(
        db, config, client, archiver_url, package, suite, run_id=None):
    async with db.acquire() as conn:
        package = await state.get_package(conn, package)
        if package is None:
            raise KeyError(package)
        if run_id is not None:
            run = await state.get_run(conn, run_id)
            merge_proposals = []
        else:
            run = await state.get_last_unabsorbed_run(
                conn, package.name, suite)
            merge_proposals = [
                (url, status)
                for (unused_package, url, status) in
                await state.iter_proposals(conn, package.name, suite=suite)]
        candidate = await state.get_candidate(conn, package.name, suite)
        if candidate is not None:
            (candidate_context, candidate_value,
             candidate_success_chance) = candidate
        else:
            candidate_context = None
            candidate_success_chance = None
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
        previous_runs = [
            r async for r in
            state.iter_previous_runs(conn, package.name, suite)]
        (queue_position, queue_wait_time) = await state.get_queue_position(
            conn, suite, package.name)
    return {
        'package': package.name,
        'merge_proposals': merge_proposals,
        'maintainer_email': package.maintainer_email,
        'uploader_emails': package.uploader_emails,
        'removed': package.removed,
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
        'candidate_success_chance': candidate_success_chance,
        'candidate_value': candidate_value,
        'previous_runs': previous_runs,
        'branch_name': branch_name,
        'branch_url': branch_url,
        'run': run,
        'queue_position': queue_position,
        'queue_wait_time': queue_wait_time,
        'tracker_url': partial(tracker_url, config),
        }


async def generate_candidates(db, suite):
    async with db.acquire() as conn:
        candidates = [(package.name, context, value, success_chance) for
                      (package, suite, context, value, success_chance) in
                      await state.iter_candidates(conn, suite=suite)]
    candidates.sort()
    return {'candidates': candidates, 'suite': suite}
