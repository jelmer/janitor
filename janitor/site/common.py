#!/usr/bin/python3

from aiohttp import ClientConnectorError
from functools import partial
import urllib.parse

from janitor import state
from janitor.site import (
    get_archive_diff,
    BuildDiffUnavailable,
    get_vcs_type,
    DebdiffRetrievalError,
    tracker_url,
    )


async def generate_pkg_context(db, config, suite, policy, client, differ_url,
                               publisher_url, package, run_id=None):
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
                conn, package.name, suite)
            merge_proposals = [
                (url, status) for (unused_package, url, status) in
                await state.iter_proposals(conn, package.name, suite=suite)]
        (publish_policy, changelog_policy,
         unused_command) = await state.get_publish_policy(
             conn, package.name, suite)
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
            branch_url = None
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
            branch_url = run.branch_url
            if run.main_branch_revision:
                unchanged_run = await state.get_unchanged_run(
                    conn, run.main_branch_revision)
            else:
                unchanged_run = None

        candidate = await state.get_candidate(conn, package.name, suite)
        if candidate is not None:
            (candidate_context, candidate_value,
             candidate_success_chance) = candidate
        else:
            candidate_context = None
            candidate_value = None
            candidate_success_chance = None
        previous_runs = [
            x async for x in
            state.iter_previous_runs(conn, package.name, suite)]
        (queue_position, queue_wait_time) = await state.get_queue_position(
            conn, suite, package.name)

    async def show_diff(role):
        try:
            (remote_name, base_revid, revid) = run.get_result_branch(role)
        except KeyError:
            return 'no result branch with role %s' % role
        if base_revid == revid:
            return ''
        url = urllib.parse.urljoin(
                publisher_url, 'diff/%s/%s' % (run.id, role))
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
        if not run.build_version:
            return ''
        if not unchanged_run or not unchanged_run.build_version:
            return ''
        try:
            debdiff, content_type = await get_archive_diff(
                client, differ_url, run, unchanged_run,
                kind='debdiff', filter_boring=True, accept='text/html')
            return debdiff.decode('utf-8', 'replace')
        except BuildDiffUnavailable:
            return ''
        except DebdiffRetrievalError as e:
            return 'Error retrieving debdiff: %s' % e

    async def vcs_type():
        return await get_vcs_type(client, publisher_url, run.package)

    return {
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
        'suite': suite,
        'show_diff': show_diff,
        'show_debdiff': show_debdiff,
        'previous_runs': previous_runs,
        'run': run,
        'candidate_context': candidate_context,
        'candidate_success_chance': candidate_success_chance,
        'candidate_value': candidate_value,
        'branch_url': branch_url,
        'queue_position': queue_position,
        'queue_wait_time': queue_wait_time,
        'publish_policy': publish_policy,
        'changelog_policy': changelog_policy,
        'tracker_url': partial(tracker_url, config),
        }


async def generate_candidates(db, suite):
    candidates = []
    async with db.acquire() as conn:
        for (package, suite, context, value,
             success_chance) in await state.iter_candidates(conn, suite=suite):
            candidates.append((package.name, value))
        candidates.sort(key=lambda x: x[1], reverse=True)
    return {'candidates': candidates, 'suite': suite}
