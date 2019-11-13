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
import functools
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

from silver_platter.debian import (
    select_preferred_probers,
    pick_additional_colocated_branches,
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
    )
from .config import read_config
from .logs import get_log_manager, ServiceUnavailable
from .prometheus import setup_metrics
from .pubsub import Topic, pubsub_handler
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
lintian_brush_fixer_failed_count = Gauge(
    'lintian_brush_fixer_failed_count',
    'Number of failures per lintian-brush fixer.',
    labelnames=('fixer', ))
review_status_count = Gauge(
    'reivew_status_count', 'Last runs by review status.',
    labelnames=('review_status',))


class NoChangesFile(Exception):
    """No changes file found."""


class JanitorResult(object):

    def __init__(self, pkg, log_id, description=None,
                 code=None, build_distribution=None, build_version=None,
                 changes_filename=None, worker_result=None,
                 logfilenames=None):
        self.package = pkg
        self.log_id = log_id
        self.description = description
        self.code = code
        self.build_distribution = build_distribution
        self.build_version = build_version
        self.changes_filename = changes_filename
        self.branch_name = None
        self.logfilenames = logfilenames
        if worker_result:
            self.context = worker_result.context
            if self.code is None:
                self.code = worker_result.code
            if self.description is None:
                self.description = worker_result.description
            self.main_branch_revision = worker_result.main_branch_revision
            self.subworker_result = worker_result.subworker
            self.revision = worker_result.revision
        else:
            self.context = None
            self.main_branch_revision = None
            self.revision = None
            self.subworker_result = None

    def json(self):
        return {
            'package': self.package,
            'log_id': self.log_id,
            'description': self.description,
            'code': self.code,
            'build_distribution': self.build_distribution,
            'build_version': self.build_version,
            'changes_filename': self.changes_filename,
            'branch_name': self.branch_name,
            'logfilenames': self.logfilenames,
            'subworker': self.subworker_result,
        }


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
                 main_branch_revision=None, revision=None):
        self.code = code
        self.description = description
        self.context = context
        self.subworker = subworker
        self.main_branch_revision = main_branch_revision
        self.revision = revision

    @classmethod
    def from_file(cls, path):
        """create a WorkerResult object from a JSON file."""
        with open(path, 'r') as f:
            worker_result = json.load(f)
        return cls(
                worker_result.get('code'), worker_result.get('description'),
                worker_result.get('context'), worker_result.get('subworker'),
                worker_result.get('main_branch_revision'),
                worker_result.get('revision'))


async def run_subprocess(args, env, log_path=None):
    if log_path:
        read, write = os.pipe()
        p = await asyncio.create_subprocess_exec(
            *args, env=env, stdout=write, stderr=write,
            stdin=asyncio.subprocess.PIPE)
        p.stdin.close()
        os.close(write)
        tee = await asyncio.create_subprocess_exec('tee', log_path, stdin=read)
        os.close(read)
        await tee.wait()
        return await p.wait()
    else:
        p = await asyncio.create_subprocess_exec(
            *args, env=env, stdin=asyncio.subprocess.PIPE)
        p.stdin.close()
        return await p.wait()


async def invoke_subprocess_worker(
        worker_kind, main_branch, env, command, output_directory,
        resume_branch=None, cached_branch=None,
        pre_check=None, post_check=None,
        build_command=None, log_path=None,
        resume_branch_result=None,
        last_build_version=None, subpath=None):
    subprocess_env = dict(os.environ.items())
    for k, v in env.items():
        if v is not None:
            subprocess_env[k] = v
    worker_module = {
        'local': 'janitor.worker',
        'gcb': 'janitor.gcb_worker',
        'ssh': 'janitor.ssh_worker',
        }[worker_kind.split(':')[0]]
    args = [sys.executable, '-m', worker_module,
            '--branch-url=%s' % main_branch.user_url,
            '--output-directory=%s' % output_directory]
    if ':' in worker_kind:
        args.append('--host=%s' % worker_kind.split(':')[1])
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
    if subpath:
        args.append('--subpath=%s' % subpath)
    if resume_branch_result:
        resume_result_path = os.path.join(
            output_directory, 'previous_result.json')
        with open(resume_result_path, 'w') as f:
            json.dump(resume_branch_result, f)
        args.append('--resume-result-path=%s' % resume_result_path)
    if last_build_version:
        args.append('--last-build-version=%s' % last_build_version)

    args.extend(command)
    return await asyncio.run_subprocess(
        args, env=subprocess_env, log_path=log_path)


async def process_one(
        db, output_directory, worker_kind, vcs_url, pkg, env, command,
        build_command, suite, pre_check=None, post_check=None,
        dry_run=False, incoming=None, logfile_manager=None,
        debsign_keyid=None, vcs_manager=None,
        possible_transports=None, possible_hosters=None,
        use_cached_only=False, refresh=False, vcs_type=None,
        subpath=None, overall_timeout=None):
    note('Running %r on %s', command, pkg)
    packages_processed_count.inc()
    log_id = str(uuid.uuid4())

    env = dict(env.items())
    env['PACKAGE'] = pkg

    # TODO(jelmer): Ideally, there shouldn't be any command-specific code here.
    if suite == "fresh-releases":
        branch_name = 'new-upstream'
    elif suite == "fresh-snapshots":
        branch_name = 'new-upstream-snapshot'
    elif suite == "lintian-fixes":
        branch_name = "lintian-fixes"
    elif suite == "unchanged":
        branch_name = "master"
    else:
        raise AssertionError('Unknown command %s' % command[0])

    if not use_cached_only:
        probers = select_preferred_probers(vcs_type)
        try:
            main_branch = open_branch_ext(
                vcs_url, possible_transports=possible_transports,
                probers=probers)
        except BranchOpenFailure as e:
            return JanitorResult(
                pkg, log_id=log_id, description=e.description, code=e.code,
                logfilenames=[])

        if subpath:
            # TODO(jelmer): cluster all packages for a single repository
            return JanitorResult(
                pkg, log_id=log_id, code='package-in-subpath',
                description=(
                    'The package is stored in a subpath rather than the '
                    'repository root.'),
                logfilenames=[])

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
                description='Missing cache branch for %s' % pkg,
                logfilenames=[])
        note('Using cached branch %s', main_branch.user_url)
        resume_branch = vcs_manager.get_branch(pkg, branch_name)
        cached_branch = None

    if refresh and resume_branch:
        note('Since refresh was requested, ignoring resume branch.')
        resume_branch = None

    async with db.acquire() as conn:
        if resume_branch is not None:
            resume_branch_result = await state.get_run_result_by_revision(
                conn, revision=resume_branch.last_revision())
        else:
            resume_branch_result = None

        last_build_version = await state.get_last_build_version(
            conn, pkg, suite)

    log_path = os.path.join(output_directory, 'worker.log')
    try:
        retcode = await asyncio.wait_for(
            invoke_subprocess_worker(
                worker_kind, main_branch, env, command, output_directory,
                resume_branch=resume_branch, cached_branch=cached_branch,
                pre_check=pre_check, post_check=post_check,
                build_command=build_command, log_path=log_path,
                resume_branch_result=resume_branch_result,
                last_build_version=last_build_version,
                subpath=subpath, overall_timeout=overall_timeout),
            timeout=overall_timeout)
    except asyncio.TimeoutError:
        return JanitorResult(
            pkg, log_id=log_id,
            code='timeout',
            description='Run timed out after %d seconds' % overall_timeout,
            logfilenames=[])

    logfilenames = []
    for name in os.listdir(output_directory):
        parts = name.split('.')
        if parts[-1] == 'log' or (
                len(parts) == 3 and
                parts[-2] == 'log' and
                parts[-1].isdigit()):
            src_build_log_path = os.path.join(output_directory, name)
            try:
                await logfile_manager.import_log(
                    pkg, log_id, src_build_log_path)
            except ServiceUnavailable as e:
                warning('Unable to upload logfile %s: %s',
                        name, e)
            else:
                logfilenames.append(name)

    if retcode != 0:
        try:
            with open(log_path, 'r') as f:
                description = list(f.readlines())[-1]
        except FileNotFoundError:
            description = 'Worker exited with return code %d' % retcode

        return JanitorResult(
            pkg, log_id=log_id,
            code='worker-failure',
            description=description,
            logfilenames=logfilenames)

    json_result_path = os.path.join(output_directory, 'result.json')
    if os.path.exists(json_result_path):
        worker_result = WorkerResult.from_file(json_result_path)
    else:
        worker_result = WorkerResult(
            'worker-missing-result',
            'Worker failed and did not write a result file.')

    if worker_result.code is not None:
        return JanitorResult(
            pkg, log_id=log_id, worker_result=worker_result,
            logfilenames=logfilenames)

    result = JanitorResult(
        pkg, log_id=log_id,
        code='success', worker_result=worker_result,
        logfilenames=logfilenames)

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
            worker_result=worker_result,
            logfilenames=logfilenames)

    enable_tag_pushing(local_branch)

    if vcs_manager:
        vcs_manager.import_branches(
            main_branch, local_branch,
            pkg, branch_name,
            additional_colocated_branches=(
                pick_additional_colocated_branches(main_branch)))
        result.branch_name = branch_name

    if result.changes_filename:
        changes_path = os.path.join(
            output_directory, result.changes_filename)
        debsign(changes_path, debsign_keyid)
        if incoming is not None:
            dget_changes(changes_path, incoming)

    return result


async def export_queue_length(db):
    while True:
        async with db.acquire() as conn:
            queue_length.set(await state.queue_length(conn))
            queue_duration.set(
                (await state.queue_duration(conn)).total_seconds())
            current_tick.set(await state.current_tick(conn))
        await asyncio.sleep(60)


async def export_stats(db):
    while True:
        async with db.acquire() as conn:
            for suite, count in await state.get_published_by_suite(conn):
                apt_package_count.labels(suite=suite).set(count)

            by_suite = {}
            by_suite_result = {}
            async for package_name, suite, run_duration, result_code in (
                    state.iter_by_suite_result_code(conn)):
                by_suite.setdefault(suite, 0)
                by_suite[suite] += 1
                by_suite_result.setdefault((suite, result_code), 0)
                by_suite_result[(suite, result_code)] += 1
            for suite, count in by_suite.items():
                run_count.labels(suite=suite).set(count)
            for (suite, result_code), count in by_suite_result.items():
                run_result_count.labels(
                    suite=suite, result_code=result_code).set(count)
            for suite, count in await state.get_never_processed(conn):
                never_processed_count.labels(suite).set(count)
            for fixer, count in await state.iter_failed_lintian_fixers(conn):
                lintian_brush_fixer_failed_count.labels(fixer).set(count)
            for review_status, count in await state.iter_review_status(conn):
                review_status_count.labels(review_status).set(count)

        # Every 30 minutes
        await asyncio.sleep(60 * 30)


class QueueProcessor(object):

    def __init__(
            self, database, worker_kind, build_command, pre_check=None,
            post_check=None, dry_run=False, incoming=None,
            logfile_manager=None, debsign_keyid=None, vcs_manager=None,
            concurrency=1, use_cached_only=False, overall_timeout=None):
        """Create a queue processor.

        Args:
          worker_kind: The kind of worker to run ('local', 'gcb')
          build_command: The command used to build packages
          pre_check: Function to run prior to modifying a package
          post_check: Function to run after modifying a package
          incoming: directory to copy debian packages to
        """
        self.database = database
        self.worker_kind = worker_kind
        self.build_command = build_command
        self.pre_check = pre_check
        self.post_check = post_check
        self.dry_run = dry_run
        self.incoming = incoming
        self.logfile_manager = logfile_manager
        self.debsign_keyid = debsign_keyid
        self.vcs_manager = vcs_manager
        self.concurrency = concurrency
        self.use_cached_only = use_cached_only
        self.started = {}
        self.per_run_directory = {}
        self.topic_queue = Topic(repeat_last=True)
        self.topic_result = Topic()
        self.overall_timeout = overall_timeout

    def status_json(self):
        return {'processing': [{
                'id': item.id,
                'package': item.package,
                'suite': item.suite,
                'estimated_duration':
                    item.estimated_duration.total_seconds()
                    if item.estimated_duration else None,
                'current_duration':
                    (datetime.now() - start_time).total_seconds(),
                'start_time': start_time.isoformat(),
                } for item, start_time in self.started.items()],
                'concurrency': self.concurrency}

    async def process_queue_item(self, item):
        start_time = datetime.now()
        self.started[item] = start_time

        self.topic_queue.publish(self.status_json())

        with tempfile.TemporaryDirectory() as output_directory:
            self.per_run_directory[item.id] = output_directory
            result = await process_one(
                self.database, output_directory, self.worker_kind,
                item.branch_url, item.package, item.env, item.command,
                suite=item.suite, pre_check=self.pre_check,
                build_command=self.build_command, post_check=self.post_check,
                dry_run=self.dry_run, incoming=self.incoming,
                debsign_keyid=self.debsign_keyid, vcs_manager=self.vcs_manager,
                logfile_manager=self.logfile_manager,
                use_cached_only=self.use_cached_only, refresh=item.refresh,
                vcs_type=item.vcs_type, subpath=item.subpath,
                overall_timeout=self.overall_timeout)
        finish_time = datetime.now()
        build_duration.labels(package=item.package, suite=item.suite).observe(
            finish_time.timestamp() - start_time.timestamp())
        if not self.dry_run:
            async with self.database.acquire() as conn:
                await state.store_run(
                    conn, result.log_id, item.package, item.branch_url,
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
                    suite=item.suite,
                    logfilenames=result.logfilenames)
                await state.drop_queue_item(conn, item.id)
        self.topic_result.publish(result.json())
        del self.started[item]
        self.topic_queue.publish(self.status_json())
        last_success_gauge.set_to_current_time()

    async def process(self):
        todo = set()
        async with self.database.acquire() as conn:
            async for item in state.iter_queue(conn, limit=self.concurrency):
                todo.add(self.process_queue_item(item))

        def handle_sigterm():
            self.concurrency = None
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
                if self.concurrency:
                    async with self.database.acquire() as conn:
                        for i in enumerate(done):
                            async for item in state.iter_queue(
                                    conn, limit=self.concurrency):
                                if item in self.started:
                                    continue
                                todo.add(self.process_queue_item(item))
        finally:
            loop.remove_signal_handler(signal.SIGTERM)


async def handle_status(request):
    queue_processor = request.app.queue_processor
    return web.json_response(queue_processor.status_json())


async def handle_log_index(request):
    queue_processor = request.app.queue_processor
    run_id = int(request.match_info['run_id'])
    try:
        directory = queue_processor.per_run_directory[run_id]
    except KeyError:
        return web.Response(
            text='No such current run: %s' % run_id, status=404)
    return web.json_response([
        n for n in os.listdir(directory)
        if os.path.isfile(os.path.join(directory, n))])


async def handle_log(request):
    queue_processor = request.app.queue_processor
    run_id = int(request.match_info['run_id'])
    filename = request.match_info['filename']
    if '/' in filename:
        return web.Response(
            text='Invalid filename %s' % request.match_info['filename'],
            status=400)
    try:
        directory = queue_processor.per_run_directory[run_id]
    except KeyError:
        return web.Response(
            text='No such current run: %s' % run_id, status=404)
    full_path = os.path.join(directory, filename)
    if os.path.exists(full_path):
        return web.FileResponse(full_path)
    else:
        return web.Response(text='No such logfile: %s' % filename, status=404)


async def run_web_server(listen_addr, port, queue_processor):
    app = web.Application()
    app.queue_processor = queue_processor
    setup_metrics(app)
    app.router.add_get('/status', handle_status)
    app.router.add_get('/log/{run_id}', handle_log_index)
    app.router.add_get('/log/{run_id}/{filename}', handle_log)
    app.router.add_get('/ws/queue', functools.partial(
        pubsub_handler, queue_processor.topic_queue))
    app.router.add_get('/ws/result', functools.partial(
        pubsub_handler, queue_processor.topic_result))
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
        '--incoming', type=str,
        help='Path to copy built Debian packages into.')
    parser.add_argument(
        '--debsign-keyid', type=str,
        help='GPG key to sign Debian package with.')
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
    parser.add_argument(
        '--config', type=str, default='janitor.conf',
        help='Path to configuration.')
    parser.add_argument(
        '--overall-timeout', type=int, default=None,
        help='Overall timeout per run (in seconds).')

    args = parser.parse_args()

    debug.set_debug_flags_from_config()

    with open(args.config, 'r') as f:
        config = read_config(f)

    state.DEFAULT_URL = config.database_location
    vcs_manager = LocalVcsManager(config.vcs_location)
    logfile_manager = get_log_manager(config.logs_location)
    db = state.Database(config.database_location)
    queue_processor = QueueProcessor(
        db,
        args.worker,
        args.build_command,
        args.pre_check, args.post_check,
        args.dry_run, args.incoming, logfile_manager,
        args.debsign_keyid,
        vcs_manager,
        args.concurrency,
        args.use_cached_only,
        overall_timeout=args.overall_timeout)
    loop = asyncio.get_event_loop()
    loop.run_until_complete(asyncio.gather(
        loop.create_task(queue_processor.process()),
        loop.create_task(export_queue_length(db)),
        loop.create_task(export_stats(db)),
        loop.create_task(run_web_server(
            args.listen_address, args.port, queue_processor)),
        ))


if __name__ == '__main__':
    sys.exit(main(sys.argv))
