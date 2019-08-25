#!/usr/bin/python3
# Copyright (C) 2018 Jelmer Vernooij <jelmer@jelmer.uk>
#
# This program is free software; you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation; either version 2 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program; if not, write to the Free Software
# Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA

from aiohttp import web
import asyncio
from datetime import datetime
import json
import os
import signal
import sys
import tempfile
import uuid

from debian.deb822 import Changes

from breezy import debug
from breezy.plugins.debian.util import (
    debsign,
    dget_changes,
    )

from prometheus_client import (
    Counter,
    Gauge,
    Histogram,
)

from silver_platter.proposal import (
    find_existing_proposed,
    enable_tag_pushing,
    UnsupportedHoster,
    NoSuchProject,
    get_hoster,
    )
from silver_platter.utils import (
    open_branch,
    BranchMissing,
    BranchUnavailable,
    )

from . import (
    state,
    ADDITIONAL_COLOCATED_BRANCHES,
    )
from .logs import get_log_manager
from .prometheus import setup_metrics
from .trace import note, warning
from .vcs import (
    get_vcs_abbreviation,
    open_branch_ext,
    BranchOpenFailure,
    LocalVcsManager,
    )

apt_package_count = Gauge(
    'apt_package_count', 'Number of packages with a version published',
    ['suite'])
packages_processed_count = Counter(
    'package_count', 'Number of packages processed.')
queue_length = Gauge(
    'queue_length', 'Number of items in the queue.')
queue_duration = Gauge(
    'queue_duration', 'Time to process all items in the queue sequentially')
last_success_gauge = Gauge(
    'job_last_success_unixtime',
    'Last time a batch job successfully finished')
build_duration = Histogram(
    'build_duration', 'Build duration',
    ['package', 'suite'])
current_tick = Gauge(
    'current_tick',
    'The current tick in the queue that\'s being processed')
run_count = Gauge(
    'run_count', 'Number of total runs.',
    labelnames=('suite', ))
run_result_count = Gauge(
    'run_result_count', 'Number of runs by code.',
    labelnames=('suite', 'result_code'))
never_processed_count = Gauge(
    'never_processed_count', 'Number of items never processed.',
    labelnames=('suite', ))


class NoChangesFile(Exception):
    """No changes file found."""


class JanitorResult(object):

    def __init__(self, pkg, log_id, description=None,
                 code=None, is_new=None,
                 build_distribution=None, build_version=None,
                 changes_filename=None, worker_result=None):
        self.package = pkg
        self.log_id = log_id
        self.description = description
        self.code = code
        self.is_new = is_new
        self.build_distribution = build_distribution
        self.build_version = build_version
        self.changes_filename = changes_filename
        self.branch_name = None
        self.revision = None
        if worker_result:
            self.context = worker_result.context
            if self.code is None:
                self.code = worker_result.code
            if self.description is None:
                self.description = worker_result.description
            self.main_branch_revision = worker_result.main_branch_revision
            self.subworker_result = worker_result.subworker
        else:
            self.context = None
            self.main_branch_revision = None
            self.subworker_result = None


def find_changes(path, package):
    for name in os.listdir(path):
        if name.startswith('%s_' % package) and name.endswith('.changes'):
            break
    else:
        raise NoChangesFile(path, package)

    with open(os.path.join(path, name), 'r') as f:
        changes = Changes(f)
        return (name, changes["Version"], changes["Distribution"])


class WorkerResult(object):
    """The result from a worker."""

    def __init__(self, code, description, context=None, subworker=None,
                 main_branch_revision=None):
        self.code = code
        self.description = description
        self.context = context
        self.subworker = subworker
        self.main_branch_revision = main_branch_revision

    @classmethod
    def from_file(cls, path):
        """create a WorkerResult object from a JSON file."""
        with open(path, 'r') as f:
            worker_result = json.load(f)
        return cls(
                worker_result.get('code'), worker_result.get('description'),
                worker_result.get('context'), worker_result.get('subworker'),
                worker_result.get('main_branch_revision'))


async def invoke_subprocess_worker(
        worker_kind, main_branch, env, command, output_directory,
        resume_branch=None, cached_branch=None,
        pre_check=None, post_check=None,
        build_command=None, log_path=None,
        resume_branch_result=None,
        last_build_version=None):
    subprocess_env = dict(os.environ.items())
    for k, v in env.items():
        if v is not None:
            subprocess_env[k] = v
    worker_module = {
        'local': 'janitor.worker',
        'gcb': 'janitor.gcb_worker',
        }[worker_kind]
    args = [sys.executable, '-m', worker_module,
            '--branch-url=%s' % main_branch.user_url,
            '--output-directory=%s' % output_directory]
    if resume_branch:
        args.append('--resume-branch-url=%s' % resume_branch.user_url)
    if cached_branch:
        args.append('--cached-branch-url=%s' % cached_branch.user_url)
    if pre_check:
        args.append('--pre-check=%s' % pre_check)
    if post_check:
        args.append('--post-check=%s' % post_check)
    if build_command:
        args.append('--build-command=%s' % build_command)
    if resume_branch_result:
        resume_result_path = os.path.join(
            output_directory, 'previous_result.json')
        with open(resume_result_path, 'w') as f:
            json.dump(resume_branch_result, f)
        args.append('--resume-result-path=%s' % resume_result_path)
    if last_build_version:
        args.append('--last-build-version=%s' % last_build_version)

    args.extend(command)

    if log_path:
        read, write = os.pipe()
        p = await asyncio.create_subprocess_exec(
            *args, env=subprocess_env, stdout=write, stderr=write)
        os.close(write)
        tee = await asyncio.create_subprocess_exec('tee', log_path, stdin=read)
        os.close(read)
        await tee.wait()
        return await p.wait()
    else:
        p = await asyncio.create_subprocess_exec(
            *args, env=subprocess_env)
        return await p.wait()


async def process_one(
        worker_kind, vcs_url, pkg, env, command, build_command,
        suite, pre_check=None, post_check=None,
        dry_run=False, incoming=None, logfile_manager=None,
        debsign_keyid=None, vcs_manager=None,
        possible_transports=None, possible_hosters=None,
        use_cached_only=False, refresh=False):
    note('Running %r on %s', command, pkg)
    packages_processed_count.inc()
    log_id = str(uuid.uuid4())

    # TODO(jelmer): Ideally, there shouldn't be any command-specific code here.
    if command == ["new-upstream"]:
        branch_name = 'new-upstream'
    elif command == ["new-upstream", "--snapshot"]:
        branch_name = 'new-upstream-snapshot'
    elif command == ["lintian-brush"]:
        branch_name = "lintian-fixes"
    elif command == ["just-build"]:
        branch_name = "master"
    else:
        raise AssertionError('Unknown command %s' % command[0])

    if not use_cached_only:
        try:
            main_branch = open_branch_ext(
                vcs_url, possible_transports=possible_transports)
        except BranchOpenFailure as e:
            return JanitorResult(
                pkg, log_id=log_id, description=e.description, code=e.code)

        try:
            hoster = get_hoster(main_branch, possible_hosters=possible_hosters)
        except UnsupportedHoster as e:
            # We can't figure out what branch to resume from when there's no
            # hoster that can tell us.
            resume_branch = None
            warning('Unsupported hoster (%s)', e)
        else:
            try:
                (resume_branch, unused_overwrite, unused_existing_proposal) = (
                    find_existing_proposed(
                        main_branch, hoster, branch_name))
            except NoSuchProject as e:
                warning('Project %s not found', e.project)
                resume_branch = None

        if resume_branch is None and vcs_manager:
            resume_branch = vcs_manager.get_branch(
                pkg, branch_name, get_vcs_abbreviation(main_branch))

        if resume_branch is not None:
            note('Resuming from %s', resume_branch.user_url)

        if vcs_manager:
            cached_branch = vcs_manager.get_branch(
                pkg, 'master', get_vcs_abbreviation(main_branch))
        else:
            cached_branch = None

        if cached_branch is not None:
            note('Using cached branch %s', cached_branch.user_url)
    else:
        if vcs_manager:
            main_branch = vcs_manager.get_branch(pkg, 'master')
        else:
            main_branch = None
        if main_branch is None:
            return JanitorResult(
                pkg, log_id=log_id,
                code='cached-branch-missing',
                description='Missing cache branch for %s' % pkg)
        note('Using cached branch %s', main_branch.user_url)
        resume_branch = vcs_manager.get_branch(pkg, branch_name)
        cached_branch = None

    if refresh and resume_branch:
        note('Since refresh was requested, ignoring resume branch.')
        resume_branch = None

    if resume_branch is not None:
        resume_branch_result = await state.get_run_result_by_revision(
            revision=resume_branch.last_revision())
    else:
        resume_branch_result = None

    last_build_version = await state.get_last_build_version(pkg, suite)

    with tempfile.TemporaryDirectory() as output_directory:
        log_path = os.path.join(output_directory, 'worker.log')
        retcode = await invoke_subprocess_worker(
                worker_kind, main_branch, env, command, output_directory,
                resume_branch=resume_branch, cached_branch=cached_branch,
                pre_check=pre_check, post_check=post_check,
                build_command=build_command, log_path=log_path,
                resume_branch_result=resume_branch_result,
                last_build_version=last_build_version)

        for name in os.listdir(output_directory):
            parts = name.split('.')
            if parts[-1] == 'log' or (
                    len(parts) == 3 and
                    parts[-2] == 'log' and
                    parts[-1].isdigit()):
                src_build_log_path = os.path.join(output_directory, name)
                await logfile_manager.import_log(
                    pkg, log_id, src_build_log_path)

        if retcode != 0:
            try:
                with open(log_path, 'r') as f:
                    description = list(f.readlines())[-1]
            except FileNotFoundError:
                description = 'Worker exited with return code %d' % retcode

            return JanitorResult(
                pkg, log_id=log_id,
                code='worker-failure',
                description=description)

        json_result_path = os.path.join(output_directory, 'result.json')
        if os.path.exists(json_result_path):
            worker_result = WorkerResult.from_file(json_result_path)
        else:
            worker_result = WorkerResult(
                'worker-missing-result',
                'Worker failed and did not write a result file.')

        if worker_result.code is not None:
            return JanitorResult(
                pkg, log_id=log_id, worker_result=worker_result)

        result = JanitorResult(
            pkg, log_id=log_id,
            code='success', worker_result=worker_result)

        try:
            (result.changes_filename, result.build_version,
             result.build_distribution) = find_changes(
                 output_directory, result.package)
        except NoChangesFile as e:
            # Oh, well.
            note('No changes file found: %s', e)

        try:
            local_branch = open_branch(os.path.join(output_directory, pkg))
        except (BranchMissing, BranchUnavailable) as e:
            return JanitorResult(
                pkg, log_id,
                description='result branch unavailable: %s' % e,
                code='result-branch-unavailable',
                worker_result=worker_result)

        result.revision = local_branch.last_revision().decode('utf-8')
        enable_tag_pushing(local_branch)

        if vcs_manager:
            vcs_manager.import_branches(
                main_branch, local_branch,
                pkg, branch_name,
                additional_colocated_branches=(
                    ADDITIONAL_COLOCATED_BRANCHES))
            result.branch_name = branch_name

        if result.changes_filename:
            changes_path = os.path.join(
                output_directory, result.changes_filename)
            debsign(changes_path, debsign_keyid)
            if incoming is not None:
                dget_changes(changes_path, incoming)

    return result


async def export_queue_length():
    while True:
        queue_length.set(await state.queue_length())
        queue_duration.set((await state.queue_duration()).total_seconds())
        current_tick.set(await state.current_tick())
        await asyncio.sleep(60)


async def export_stats():
    while True:
        for suite, count in await state.get_published_by_suite():
            apt_package_count.labels(suite=suite).set(count)

        by_suite = {}
        by_suite_result = {}
        async for package_name, suite, run_duration, result_code in (
                state.iter_by_suite_result_code()):
            by_suite.setdefault(suite, 0)
            by_suite[suite] += 1
            by_suite_result.setdefault((suite, result_code), 0)
            by_suite_result[(suite, result_code)] += 1
        for suite, count in by_suite.items():
            run_count.labels(suite=suite).set(count)
        for (suite, result_code), count in by_suite_result.items():
            run_result_count.labels(
                suite=suite, result_code=result_code).set(count)
        for suite, count in await state.get_never_processed():
            never_processed_count.labels(suite).set(count)

        # Every 30 minutes
        await asyncio.sleep(60 * 30)


async def process_queue(
        worker_kind, build_command,
        started, pre_check=None, post_check=None,
        dry_run=False, incoming=None, log_dir=None,
        debsign_keyid=None, vcs_manager=None,
        concurrency=1, use_cached_only=False):
    """Process the items added to the queue.

    Args:
      worker_kind: The kind of worker to run ('local', 'gcb')
      build_command: The command used to build packages
      pre_check: Function to run prior to modifying a package
      post_check: Function to run after modifying a package
      incoming: directory to copy debian packages to
      log_dir: Directory to cop
    """
    logfile_manager = get_log_manager(log_dir)

    async def process_queue_item(item):
        start_time = datetime.now()

        env = dict(item.env.items())
        env['PACKAGE'] = item.package

        result = await process_one(
            worker_kind, item.branch_url, item.package, env, item.command,
            suite=item.suite, pre_check=pre_check,
            build_command=build_command, post_check=post_check,
            dry_run=dry_run, incoming=incoming,
            debsign_keyid=debsign_keyid, vcs_manager=vcs_manager,
            logfile_manager=logfile_manager, use_cached_only=use_cached_only,
            refresh=item.refresh)
        finish_time = datetime.now()
        build_duration.labels(package=item.package, suite=item.suite).observe(
            finish_time.timestamp() - start_time.timestamp())
        if not dry_run:
            await state.store_run(
                result.log_id, item.package, item.branch_url,
                start_time, finish_time, item.command,
                result.description,
                item.env.get('CONTEXT'),
                result.context,
                result.main_branch_revision,
                result.code,
                build_version=result.build_version,
                build_distribution=result.build_distribution,
                branch_name=result.branch_name,
                revision=result.revision,
                subworker_result=result.subworker_result,
                suite=item.suite)

            await state.drop_queue_item(item.id)
        last_success_gauge.set_to_current_time()

    started = set()
    todo = set()
    async for item in state.iter_queue(limit=concurrency):
        todo.add(process_queue_item(item))
        started.add(item)

    def handle_sigterm():
        global concurrency
        concurrency = None
        note('Received SIGTERM; not starting new jobs.')

    loop = asyncio.get_event_loop()
    loop.add_signal_handler(signal.SIGTERM, handle_sigterm)
    try:
        while True:
            if not todo:
                note('Nothing to do. Sleeping for 60s.')
                await asyncio.sleep(60)
                continue
            done, pending = await asyncio.wait(
                todo, return_when='FIRST_COMPLETED')
            for task in done:
                task.result()
            todo = pending
            if concurrency:
                for i in enumerate(done):
                    async for item in state.iter_queue(limit=concurrency):
                        if item in started:
                            continue
                        todo.add(process_queue_item(item))
                        started.add(item)
    finally:
        loop.remove_signal_handler(signal.SIGTERM)


async def handle_status(started, request):
    return web.json_response({
        'processing': [item.package for item in started]
    })


async def run_web_server(listen_addr, port, started):
    app = web.Application()
    setup_metrics(app)
    app.router.add_get('/status', functools.partial(handle_status, started))
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()


def main(argv=None):
    import argparse
    parser = argparse.ArgumentParser(prog='janitor.runner')
    parser.add_argument(
        '--listen-address', type=str,
        help='Listen address', default='localhost')
    parser.add_argument(
        '--port', type=int,
        help='Listen port', default=9911)
    parser.add_argument(
        '--pre-check',
        help='Command to run to check whether to process package.',
        type=str)
    parser.add_argument(
        '--post-check',
        help='Command to run to check package before pushing.',
        type=str)
    parser.add_argument(
        '--build-command',
        help='Build package to verify it.', type=str,
        default=None)
    parser.add_argument(
        "--dry-run",
        help="Create branches but don't push or propose anything.",
        action="store_true", default=False)
    parser.add_argument(
        '--log-dir', help='Directory to store logs in.',
        type=str, default='https://s3.nl-ams.scw.cloud')
    parser.add_argument(
        '--incoming', type=str,
        help='Path to copy built Debian packages into.')
    parser.add_argument(
        '--debsign-keyid', type=str,
        help='GPG key to sign Debian package with.')
    parser.add_argument(
        '--vcs-result-dir', type=str,
        help='Directory to store VCS repositories in.')
    parser.add_argument(
        '--worker', type=str,
        default='local',
        choices=['local', 'gcb'],
        help='Worker to use.')
    parser.add_argument(
        '--concurrency', type=int, default=1,
        help='Number of workers to run in parallel.')
    parser.add_argument(
        '--use-cached-only', action='store_true',
        help='Use cached branches only.')

    args = parser.parse_args()

    debug.set_debug_flags_from_config()

    vcs_manager = LocalVcsManager(args.vcs_result_dir)
    loop = asyncio.get_event_loop()
    loop.run_until_complete(asyncio.gather(
        loop.create_task(process_queue(
            args.worker,
            args.build_command,
            started,
            args.pre_check, args.post_check,
            args.dry_run, args.incoming, args.log_dir,
            args.debsign_keyid,
            vcs_manager,
            args.concurrency,
            args.use_cached_only)),
        loop.create_task(export_queue_length()),
        loop.create_task(export_stats()),
        loop.create_task(run_web_server(
            args.listen_address, args.port, started)),
        ))


if __name__ == '__main__':
    sys.exit(main(sys.argv))
