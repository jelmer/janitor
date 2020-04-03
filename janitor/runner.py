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

from aiohttp import web, MultipartWriter, ClientSession, ClientConnectionError
import asyncio
from contextlib import ExitStack
from datetime import datetime
import functools
import json
import os
import re
import signal
import sys
import tempfile
import uuid

from debian.deb822 import Changes

from breezy import debug, urlutils
from breezy.errors import PermissionDenied
from breezy.plugins.debian.util import (
    debsign,
    )

from prometheus_client import (
    Counter,
    Gauge,
    Histogram,
)

from lintian_brush.salsa import (
    guess_repository_url,
    salsa_url_from_alioth_url,
    )

from silver_platter.debian import (
    select_preferred_probers,
    select_probers,
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
    'review_status_count', 'Last runs by review status.',
    labelnames=('review_status',))


class NoChangesFile(Exception):
    """No changes file found."""


class JanitorResult(object):

    def __init__(self, pkg, log_id, branch_url, description=None,
                 code=None, build_distribution=None, build_version=None,
                 changes_filename=None, worker_result=None,
                 logfilenames=None, branch_name=None):
        self.package = pkg
        self.log_id = log_id
        self.description = description
        self.branch_url = branch_url
        self.code = code
        self.build_distribution = build_distribution
        self.build_version = build_version
        self.changes_filename = changes_filename
        self.branch_name = branch_name
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
            self.value = worker_result.value
        else:
            self.context = None
            self.main_branch_revision = None
            self.revision = None
            self.subworker_result = None
            self.value = None

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
            'value': self.value,
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
                 main_branch_revision=None, revision=None, value=None):
        self.code = code
        self.description = description
        self.context = context
        self.subworker = subworker
        self.main_branch_revision = main_branch_revision
        self.revision = revision
        self.value = value

    @classmethod
    def from_file(cls, path):
        """create a WorkerResult object from a JSON file."""
        with open(path, 'r') as f:
            worker_result = json.load(f)
        return cls(
                worker_result.get('code'), worker_result.get('description'),
                worker_result.get('context'), worker_result.get('subworker'),
                worker_result.get('main_branch_revision'),
                worker_result.get('revision'), worker_result.get('value'))


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
        last_build_version=None, subpath=None,
        build_distribution=None, build_suffix=None):
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
            '--branch-url=%s' % main_branch.user_url.rstrip('/'),
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
    if build_distribution:
        args.append('--build-distribution=%s' % build_distribution)
    if build_suffix:
        args.append('--build-suffix=%s' % build_suffix)

    args.extend(command)
    return await run_subprocess(args, env=subprocess_env, log_path=log_path)


async def open_guessed_salsa_branch(
        conn, pkg, vcs_type, vcs_url, possible_transports=None):
    package = await state.get_package(conn, pkg)
    probers = select_probers('git')
    vcs_url, params = urlutils.split_segment_parameters_raw(vcs_url)

    tried = set(vcs_url)

    # These are the same transformations applied by vcswatc. The goal is mostly
    # to get a URL that properly redirects.
    https_alioth_url = re.sub(
        r'(https?|git)://(anonscm|git).debian.org/(git/)?',
        r'https://anonscm.debian.org/git/',
        vcs_url)

    for salsa_url in [
            https_alioth_url,
            salsa_url_from_alioth_url(vcs_type, vcs_url),
            guess_repository_url(package.name, package.maintainer_email),
            'https://salsa.debian.org/debian/%s.git' % package.name,
            ]:
        if not salsa_url or salsa_url in tried:
            continue

        tried.add(salsa_url)

        salsa_url = urlutils.join_segment_parameters_raw(salsa_url, *params)

        note('Trying to access salsa URL %s instead.', salsa_url)
        try:
            branch = open_branch_ext(
                salsa_url, possible_transports=possible_transports,
                probers=probers)
        except BranchOpenFailure:
            pass
        else:
            note('Converting alioth URL: %s -> %s', vcs_url, salsa_url)
            return branch
    return None


async def open_branch_with_fallback(
        conn, pkg, vcs_type, vcs_url, possible_transports=None):
    probers = select_preferred_probers(vcs_type)
    try:
        return open_branch_ext(
            vcs_url, possible_transports=possible_transports,
            probers=probers)
    except BranchOpenFailure as e:
        if e.code == 'hosted-on-alioth':
            note('Branch %s is hosted on alioth. Trying some other options..',
                 vcs_url)
            try:
                branch = await open_guessed_salsa_branch(
                    conn, pkg, vcs_type, vcs_url,
                    possible_transports=possible_transports)
            except BranchOpenFailure:
                raise e
            else:
                if branch:
                    await state.update_branch_url(
                        conn, pkg, 'Git', branch.user_url.rstrip('/'))
                    return branch
        raise


class UploadFailedError(Exception):
    """Upload failed."""


async def upload_changes(changes_path, incoming_url):
    """Upload changes to the archiver.

    Args:
      changes_path: Changes path
      incoming_url: Incoming URL
    """
    async with ClientSession() as session:
        with MultipartWriter() as mpwriter, ExitStack() as es:
            f = open(changes_path, 'r')
            dsc = Changes(f)
            f.seek(0)
            es.enter_context(f)
            mpwriter.append(f)
            for file_details in dsc['files']:
                name = file_details['name']
                path = os.path.join(os.path.dirname(changes_path), name)
                f = open(path, 'rb')
                es.enter_context(f)
                mpwriter.append(f)
            try:
                async with session.post(incoming_url, data=mpwriter) as resp:
                    if resp.status != 200:
                        raise UploadFailedError(resp)
            except ClientConnectionError as e:
                raise UploadFailedError(e)


async def import_logs(output_directory, logfile_manager, pkg, log_id):
    logfilenames = []
    for entry in os.scandir(output_directory):
        if entry.is_dir():
            continue
        parts = entry.name.split('.')
        if parts[-1] == 'log' or (
                len(parts) == 3 and
                parts[-2] == 'log' and
                parts[-1].isdigit()):
            try:
                await logfile_manager.import_log(pkg, log_id, entry.path)
            except ServiceUnavailable as e:
                warning('Unable to upload logfile %s: %s',
                        entry.name, e)
            else:
                logfilenames.append(entry.name)
    return logfilenames


def get_suite_config(config, name):
    for s in config.suite:
        if s.name == name:
            return s
    raise KeyError(name)


class ActiveRun(object):

    def __init__(self, output_directory, pkg, suite, queue_id,
                 estimated_duration):
        self.start_time = datetime.now()
        self.output_directory = output_directory
        self.log_id = str(uuid.uuid4())
        self.pkg = pkg
        self.suite = suite
        self.queue_id = queue_id
        self.estimated_duration = estimated_duration

    def kill(self):
        self._task.cancel()

    def json(self):
        return {
            'queue_id': self.queue_id,
            'id': self.log_id,
            'package': self.pkg,
            'suite': self.suite,
            'estimated_duration':
                self.estimated_duration.total_seconds()
                if self.estimated_duration else None,
            'current_duration':
                (datetime.now() - self.start_time).total_seconds(),
            'start_time': self.start_time.isoformat(),
            }

    async def process(self, *args, **kwargs):
        try:
            return await self._process(*args, **kwargs)
        finally:
            self.finish_time = datetime.now()

    async def _process(
            self, db, config, worker_kind, vcs_url, command,
            build_command, pre_check=None, post_check=None,
            dry_run=False, incoming_url=None, logfile_manager=None,
            debsign_keyid=None, vcs_manager=None,
            possible_transports=None, possible_hosters=None,
            use_cached_only=False, refresh=False, vcs_type=None,
            subpath=None, overall_timeout=None, upstream_branch_url=None,
            committer=None):
        note('Running %r on %s', command, self.pkg)
        packages_processed_count.inc()

        env = {}
        env['PACKAGE'] = self.pkg
        if committer:
            env['COMMITTER'] = committer
        if upstream_branch_url:
            env['UPSTREAM_BRANCH_URL'] = upstream_branch_url

        try:
            suite_config = get_suite_config(config, self.suite)
        except KeyError:
            return JanitorResult(
                self.pkg, log_id=self.log_id,
                code='unknown-suite',
                description='Suite %s not in configuration' % self.suite,
                logfilenames=[],
                branch_url=vcs_url)

        if not use_cached_only:
            async with db.acquire() as conn:
                try:
                    main_branch = await open_branch_with_fallback(
                        conn, self.pkg, vcs_type, vcs_url,
                        possible_transports=possible_transports)
                except BranchOpenFailure as e:
                    await state.update_branch_status(
                        conn, vcs_url, None, status=e.code,
                        description=e.description, revision=None)
                    return JanitorResult(
                        self.pkg, log_id=self.log_id, branch_url=vcs_url,
                        description=e.description, code=e.code,
                        logfilenames=[])
                else:
                    branch_url = main_branch.user_url
                    await state.update_branch_status(
                        conn, vcs_url, branch_url, status='success',
                        revision=main_branch.last_revision())

            try:
                hoster = get_hoster(
                    main_branch, possible_hosters=possible_hosters)
            except UnsupportedHoster as e:
                # We can't figure out what branch to resume from when there's
                # no hoster that can tell us.
                resume_branch = None
                warning('Unsupported hoster (%s)', e)
            else:
                try:
                    (resume_branch, unused_overwrite,
                     unused_existing_proposal) = find_existing_proposed(
                            main_branch, hoster, suite_config.branch_name)
                except NoSuchProject as e:
                    warning('Project %s not found', e.project)
                    resume_branch = None
                except PermissionDenied as e:
                    warning('Unable to list existing proposals: %s', e)
                    resume_branch = None

            if resume_branch is None and vcs_manager:
                resume_branch = vcs_manager.get_branch(
                    self.pkg, suite_config.branch_name,
                    get_vcs_abbreviation(main_branch))

            if resume_branch is not None:
                note('Resuming from %s', resume_branch.user_url)

            if vcs_manager:
                cached_branch = vcs_manager.get_branch(
                    self.pkg, 'master', get_vcs_abbreviation(main_branch))
            else:
                cached_branch = None

            if cached_branch is not None:
                note('Using cached branch %s', cached_branch.user_url)
        else:
            if vcs_manager:
                main_branch = vcs_manager.get_branch(self.pkg, 'master')
            else:
                main_branch = None
            if main_branch is None:
                return JanitorResult(
                    self.pkg, log_id=self.log_id, branch_url=branch_url,
                    code='cached-branch-missing',
                    description='Missing cache branch for %s' % self.pkg,
                    logfilenames=[])
            note('Using cached branch %s', main_branch.user_url)
            resume_branch = vcs_manager.get_branch(
                self.pkg, suite_config.branch_name)
            cached_branch = None

        if refresh and resume_branch:
            note('Since refresh was requested, ignoring resume branch.')
            resume_branch = None

        async with db.acquire() as conn:
            if resume_branch is not None:
                (resume_branch_result, resume_branch_name, resume_review_status
                 ) = await state.get_run_result_by_revision(
                    conn, self.suite, revision=resume_branch.last_revision())
                if resume_review_status == 'rejected':
                    note('Unsetting resume branch, since last run was '
                         'rejected.')
                    resume_branch_result = None
                    resume_branch = None
                    resume_branch_name = None
            else:
                resume_branch_result = None
                resume_branch_name = None

            last_build_version = await state.get_last_build_version(
                conn, self.pkg, self.suite)

        log_path = os.path.join(self.output_directory, 'worker.log')
        try:
            self._task = asyncio.wait_for(
                invoke_subprocess_worker(
                    worker_kind, main_branch, env, command,
                    self.output_directory, resume_branch=resume_branch,
                    cached_branch=cached_branch, pre_check=pre_check,
                    post_check=post_check,
                    build_command=suite_config.build_command,
                    log_path=log_path,
                    resume_branch_result=resume_branch_result,
                    last_build_version=last_build_version, subpath=subpath,
                    build_distribution=suite_config.build_distribution,
                    build_suffix=suite_config.build_suffix),
                timeout=overall_timeout)
            retcode = await self._task
        except asyncio.TimeoutError:
            return JanitorResult(
                self.pkg, log_id=self.log_id, branch_url=branch_url,
                code='timeout',
                description='Run timed out after %d seconds' % overall_timeout,
                logfilenames=[])

        logfilenames = await import_logs(
            self.output_directory, logfile_manager, self.pkg, self.log_id)

        if retcode != 0:
            if retcode < 0:
                description = 'Worker killed with signal %d' % abs(retcode)
                code = 'worker-killed'
            else:
                code = 'worker-failure'
                try:
                    with open(log_path, 'r') as f:
                        description = list(f.readlines())[-1]
                except FileNotFoundError:
                    description = 'Worker exited with return code %d' % retcode

            return JanitorResult(
                self.pkg, log_id=self.log_id, branch_url=branch_url,
                code=code, description=description,
                logfilenames=logfilenames)

        json_result_path = os.path.join(self.output_directory, 'result.json')
        if os.path.exists(json_result_path):
            worker_result = WorkerResult.from_file(json_result_path)
        else:
            worker_result = WorkerResult(
                'worker-missing-result',
                'Worker failed and did not write a result file.')

        if worker_result.code is not None:
            return JanitorResult(
                self.pkg, log_id=self.log_id, branch_url=branch_url,
                worker_result=worker_result, logfilenames=logfilenames,
                branch_name=(
                    resume_branch_name
                    if worker_result.code == 'nothing-to-do' else None))

        result = JanitorResult(
            self.pkg, log_id=self.log_id, branch_url=branch_url,
            code='success', worker_result=worker_result,
            logfilenames=logfilenames)

        try:
            (result.changes_filename, result.build_version,
             result.build_distribution) = find_changes(
                 self.output_directory, result.package)
        except NoChangesFile as e:
            # Oh, well.
            note('No changes file found: %s', e)

        try:
            local_branch = open_branch(
                os.path.join(self.output_directory, self.pkg))
        except (BranchMissing, BranchUnavailable) as e:
            return JanitorResult(
                self.pkg, self.log_id, branch_url,
                description='result branch unavailable: %s' % e,
                code='result-branch-unavailable',
                worker_result=worker_result,
                logfilenames=logfilenames)

        enable_tag_pushing(local_branch)

        if vcs_manager:
            vcs_manager.import_branches(
                main_branch, local_branch,
                self.pkg, suite_config.branch_name,
                additional_colocated_branches=(
                    pick_additional_colocated_branches(main_branch)))
            result.branch_name = suite_config.branch_name

        if result.changes_filename:
            changes_path = os.path.join(
                self.output_directory, result.changes_filename)
            debsign(changes_path, debsign_keyid)
            if incoming_url is not None:
                try:
                    await upload_changes(changes_path, incoming_url)
                except UploadFailedError as e:
                    warning('Unable to upload changes file %s: %r',
                            result.changes_filename, e)
            if self.suite != 'unchanged':
                async with db.acquire() as conn:
                    run = await state.get_unchanged_run(
                        conn, worker_result.main_branch_revision)
                    if run is None:
                        note('Scheduling control run for %s.', self.pkg)
                        duration = datetime.now() - self.start_time
                        await state.add_to_queue(
                            conn, self.pkg, [
                                'just-build',
                                ('--revision=%s' %
                                 worker_result.main_branch_revision)
                            ],
                            'unchanged', offset=-10,
                            estimated_duration=duration, requestor='control')
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
            self, database, config, worker_kind, build_command, pre_check=None,
            post_check=None, dry_run=False, incoming_url=None,
            logfile_manager=None, debsign_keyid=None, vcs_manager=None,
            concurrency=1, use_cached_only=False, overall_timeout=None,
            committer=None):
        """Create a queue processor.

        Args:
          worker_kind: The kind of worker to run ('local', 'gcb')
          build_command: The command used to build packages
          pre_check: Function to run prior to modifying a package
          post_check: Function to run after modifying a package
          incoming_url: location to upload debian packages to
        """
        self.database = database
        self.config = config
        self.worker_kind = worker_kind
        self.build_command = build_command
        self.pre_check = pre_check
        self.post_check = post_check
        self.dry_run = dry_run
        self.incoming_url = incoming_url
        self.logfile_manager = logfile_manager
        self.debsign_keyid = debsign_keyid
        self.vcs_manager = vcs_manager
        self.concurrency = concurrency
        self.use_cached_only = use_cached_only
        self.topic_queue = Topic(repeat_last=True)
        self.topic_result = Topic()
        self.overall_timeout = overall_timeout
        self.committer = committer
        self.active_runs = {}

    def status_json(self):
        return {
            'processing':
                [active_run.json()
                 for active_run in self.active_runs.values()],
            'concurrency': self.concurrency}

    async def process_queue_item(self, item):
        with tempfile.TemporaryDirectory() as output_directory:
            active_run = ActiveRun(
                output_directory, pkg=item.package, suite=item.suite,
                estimated_duration=item.estimated_duration,
                queue_id=item.id)
            self.active_runs[item.id] = active_run
            self.topic_queue.publish(self.status_json())
            result = await active_run.process(
                self.database, self.config, self.worker_kind,
                item.branch_url, item.command,
                pre_check=self.pre_check,
                build_command=self.build_command, post_check=self.post_check,
                dry_run=self.dry_run, incoming_url=self.incoming_url,
                debsign_keyid=self.debsign_keyid, vcs_manager=self.vcs_manager,
                logfile_manager=self.logfile_manager,
                use_cached_only=self.use_cached_only, refresh=item.refresh,
                vcs_type=item.vcs_type, subpath=item.subpath,
                overall_timeout=self.overall_timeout,
                upstream_branch_url=item.upstream_branch_url,
                committer=self.committer)
        build_duration.labels(package=item.package, suite=item.suite).observe(
            active_run.finish_time.timestamp() -
            active_run.start_time.timestamp())
        if not self.dry_run:
            async with self.database.acquire() as conn:
                await state.store_run(
                    conn, result.log_id, item.package, result.branch_url,
                    active_run.start_time, active_run.finish_time,
                    item.command, result.description, item.context,
                    result.context, result.main_branch_revision, result.code,
                    build_version=result.build_version,
                    build_distribution=result.build_distribution,
                    branch_name=result.branch_name, revision=result.revision,
                    subworker_result=result.subworker_result, suite=item.suite,
                    logfilenames=result.logfilenames, value=result.value)
                await state.drop_queue_item(conn, item.id)
        self.topic_result.publish(result.json())
        del self.active_runs[item.id]
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
                                if item.id in self.active_runs:
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
        directory = queue_processor.active_runs[run_id].output_directory
    except KeyError:
        return web.Response(
            text='No such current run: %s' % run_id, status=404)
    return web.json_response([
        n for n in os.listdir(directory)
        if os.path.isfile(os.path.join(directory, n))])


async def handle_kill(request):
    queue_processor = request.app.queue_processor
    run_id = int(request.match_info['run_id'])
    try:
        ret = queue_processor.active_runs[run_id].json()
        queue_processor.active_runs[run_id].kill()
    except KeyError:
        return web.Response(
            text='No such current run: %s' % run_id, status=404)
    return web.json_response(ret)


async def handle_log(request):
    queue_processor = request.app.queue_processor
    run_id = int(request.match_info['run_id'])
    filename = request.match_info['filename']
    if '/' in filename:
        return web.Response(
            text='Invalid filename %s' % request.match_info['filename'],
            status=400)
    try:
        directory = queue_processor.active_runs[run_id].output_directory
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
    app.router.add_post('/kill/{run_id}', handle_kill)
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
        '--incoming-url', type=str,
        help='URL to upload built Debian packages to.')
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
        config,
        args.worker,
        args.build_command,
        args.pre_check, args.post_check,
        args.dry_run, args.incoming_url,
        logfile_manager,
        args.debsign_keyid,
        vcs_manager,
        args.concurrency,
        args.use_cached_only,
        overall_timeout=args.overall_timeout,
        committer=config.committer)
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
