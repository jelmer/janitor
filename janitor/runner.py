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

from aiohttp import (
    web, MultipartWriter, ClientSession, ClientConnectionError, WSMsgType,
    )
import asyncio
from contextlib import ExitStack
from datetime import datetime
from email.utils import parseaddr
import functools
import json
from io import BytesIO
import os
import re
import signal
import sys
import tempfile
from typing import List, Any, Optional, Iterable, BinaryIO, Dict, Tuple, Set
import uuid
import urllib.parse

from debian.deb822 import Changes

from breezy import debug, urlutils
from breezy.errors import PermissionDenied
from breezy.plugins.debian.util import (
    debsign,
    )
from breezy.propose import Hoster
from breezy.transport import Transport

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
from .config import read_config, get_suite_config, Config
from .logs import get_log_manager, ServiceUnavailable, LogFileManager
from .prometheus import setup_metrics
from .pubsub import Topic, pubsub_handler
from .trace import note, warning
from .vcs import (
    get_vcs_abbreviation,
    open_branch_ext,
    BranchOpenFailure,
    LocalVcsManager,
    RemoteVcsManager,
    VcsManager,
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
review_status_count = Gauge(
    'review_status_count', 'Last runs by review status.',
    labelnames=('review_status',))


class NoChangesFile(Exception):
    """No changes file found."""


class JanitorResult(object):

    def __init__(self, pkg, log_id, branch_url, description=None,
                 code=None, worker_result=None,
                 logfilenames=None, branch_name=None):
        self.package = pkg
        self.log_id = log_id
        self.description = description
        self.branch_url = branch_url
        self.code = code
        self.build_distribution = (
            worker_result.build_distribution if worker_result else None)
        self.build_version = (
            worker_result.build_version if worker_result else None)
        self.changes_filename = (
            worker_result.changes_filename if worker_result else None)
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
                 main_branch_revision=None, revision=None, value=None,
                 changes_filename=None, build_distribution=None,
                 build_version=None):
        self.code = code
        self.description = description
        self.context = context
        self.subworker = subworker
        self.main_branch_revision = main_branch_revision
        self.revision = revision
        self.value = value
        self.changes_filename = changes_filename
        self.build_distribution = build_distribution
        self.build_version = build_version

    @classmethod
    def from_file(cls, path):
        """create a WorkerResult object from a JSON file."""
        with open(path, 'r') as f:
            worker_result = json.load(f)
        return cls.from_json(worker_result)

    @classmethod
    def from_json(cls, worker_result):
        main_branch_revision = worker_result.get('main_branch_revision')
        if main_branch_revision is not None:
            main_branch_revision = main_branch_revision.encode('utf-8')
        revision = worker_result.get('revision')
        if revision is not None:
            revision = revision.encode('utf-8')
        return cls(
                worker_result.get('code'), worker_result.get('description'),
                worker_result.get('context'), worker_result.get('subworker'),
                main_branch_revision,
                revision, worker_result.get('value'),
                worker_result.get('changes_filename'),
                worker_result.get('build_distribution'),
                worker_result.get('build_version'))


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
        resume_branch=None, cached_branch_url=None,
        pre_check=None, post_check=None,
        build_command: Optional[str] = None,
        log_path=None,
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
    if cached_branch_url:
        args.append('--cached-branch-url=%s' % cached_branch_url)
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


async def upload_changes(changes_path: str, incoming_url: str):
    """Upload changes to the archiver.

    Args:
      changes_path: Changes path
      incoming_url: Incoming URL
    """
    async with ClientSession() as session:
        with ExitStack() as es:
            with MultipartWriter() as mpwriter:
                f = open(changes_path, 'r')
                es.enter_context(f)
                dsc = Changes(f)
                f.seek(0)
                mpwriter.append(f)
                for file_details in dsc['files']:
                    name = file_details['name']
                    path = os.path.join(os.path.dirname(changes_path), name)
                    g = open(path, 'rb')
                    es.enter_context(g)
                    mpwriter.append(g)
            try:
                async with session.post(incoming_url, data=mpwriter) as resp:
                    if resp.status != 200:
                        raise UploadFailedError(resp)
            except ClientConnectionError as e:
                raise UploadFailedError(e)


async def import_logs(output_directory: str,
                      logfile_manager: LogFileManager,
                      pkg: str,
                      log_id: str) -> List[str]:
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


class ActiveRun(object):
    """Tracks state of an active run."""

    queue_item: state.QueueItem
    log_id: str
    start_time: datetime
    worker_name: str

    def __init__(self, queue_item: state.QueueItem):
        self.queue_item = queue_item
        self.start_time = datetime.now()
        self.log_id = str(uuid.uuid4())

    @property
    def current_duration(self):
        return datetime.now() - self.start_time

    def kill(self) -> None:
        """Abort this run."""
        raise NotImplementedError(self.kill)

    def list_log_files(self) -> Iterable[str]:
        raise NotImplementedError(self.list_log_files)

    def get_log_file(self, name) -> Iterable[bytes]:
        raise NotImplementedError(self.get_log_file)

    def json(self) -> Any:
        """Return a JSON representation."""
        return {
            'queue_id': self.queue_item.id,
            'id': self.log_id,
            'package': self.queue_item.package,
            'suite': self.queue_item.suite,
            'estimated_duration':
                self.queue_item.estimated_duration.total_seconds()
                if self.queue_item.estimated_duration else None,
            'current_duration':
                self.current_duration.total_seconds(),
            'start_time': self.start_time.isoformat(),
            'worker': self.worker_name
            }


class ActiveRemoteRun(ActiveRun):

    log_files: Dict[str, BinaryIO]
    websockets: Set[web.WebSocketResponse]

    def __init__(self, queue_item: state.QueueItem, worker_name: str):
        super(ActiveRemoteRun, self).__init__(queue_item)
        self.worker_name = worker_name
        self.log_files = {}
        self.websockets = set()
        self.main_branch_url = self.queue_item.branch_url
        self.resume_branch_name = None

    def kill(self) -> None:
        for ws in self.websockets:
            pass  # TODO(jelmer): Send 'abort'

    def list_log_files(self):
        return self.log_files.keys()

    def get_log_file(self, name):
        try:
            return BytesIO(self.log_files[name].getvalue())
        except KeyError:
            raise FileNotFoundError


async def open_canonical_main_branch(
        conn, queue_item, possible_transports=None):
    try:
        main_branch = await open_branch_with_fallback(
            conn, queue_item.package,
            queue_item.vcs_type, queue_item.branch_url,
            possible_transports=possible_transports)
    except BranchOpenFailure as e:
        await state.update_branch_status(
            conn, queue_item.branch_url, None, status=e.code,
            description=e.description, revision=None)
        raise
    else:
        branch_url = main_branch.user_url
        await state.update_branch_status(
            conn, queue_item.branch_url, branch_url,
            status='success', revision=main_branch.last_revision())
        return main_branch


async def open_resume_branch(main_branch, branch_name, possible_hosters=None):
    try:
        hoster = get_hoster(
            main_branch, possible_hosters=possible_hosters)
    except UnsupportedHoster as e:
        # We can't figure out what branch to resume from when there's
        # no hoster that can tell us.
        return None
        warning('Unsupported hoster (%s)', e)
    else:
        try:
            (resume_branch, unused_overwrite,
             unused_existing_proposal) = find_existing_proposed(
                    main_branch, hoster, branch_name)
        except NoSuchProject as e:
            warning('Project %s not found', e.project)
            return None
        except PermissionDenied as e:
            warning('Unable to list existing proposals: %s', e)
            return None
        else:
            return resume_branch


async def check_resume_result(conn, suite, resume_branch):
    if resume_branch is not None:
        (resume_branch_result, resume_branch_name, resume_review_status
         ) = await state.get_run_result_by_revision(
            conn, suite, revision=resume_branch.last_revision())
        if resume_review_status == 'rejected':
            note('Unsetting resume branch, since last run was '
                 'rejected.')
            return (None, None, None)
        return (resume_branch, resume_branch_name, resume_branch_result)
    else:
        return (None, None, None)


def suite_build_env(suite_config, apt_location):
    env = {
        'EXTRA_REPOSITORIES': ':'.join([
            'deb %s %s/ main' % (apt_location, suite)
            for suite in suite_config.extra_build_suite])}

    env.update([(env.key, env.value) for env in suite_config.sbuild_env])
    return env


class ActiveLocalRun(ActiveRun):

    # TODO(jelmer): Use short host name instead?
    worker_name = 'local'

    def __init__(self, queue_item: state.QueueItem,
                 output_directory: str):
        super(ActiveLocalRun, self).__init__(queue_item)
        self.output_directory = output_directory

    def kill(self) -> None:
        self._task.cancel()

    def list_log_files(self):
        return [
            n for n in os.listdir(self.output_directory)
            if os.path.isfile(os.path.join(self.output_directory, n))
            and n.endswith('.log')]

    def get_log_file(self, name):
        full_path = os.path.join(self.output_directory, name)
        return open(full_path, 'rb')

    async def process(
            self, db: state.Database, config: Config,
            vcs_manager: VcsManager,
            logfile_manager: LogFileManager,
            worker_kind: str,
            build_command: Optional[str],
            apt_location: str,
            pre_check=None,
            post_check=None,
            dry_run: bool = False,
            incoming_url: Optional[str] = None,
            debsign_keyid: Optional[str] = None,
            possible_transports: Optional[List[Transport]] = None,
            possible_hosters: Optional[List[Hoster]] = None,
            use_cached_only: bool = False,
            overall_timeout: Optional[int] = None,
            committer: Optional[str] = None) -> JanitorResult:
        note('Running %r on %s', self.queue_item.command,
             self.queue_item.package)

        if self.queue_item.branch_url is None:
            return JanitorResult(
                self.queue_item.package, log_id=self.log_id,
                branch_url=self.queue_item.branch_url,
                description='No VCS URL known for package.',
                code='not-in-vcs', logfilenames=[])

        env = {}
        env['PACKAGE'] = self.queue_item.package
        if committer:
            env['COMMITTER'] = committer
        if self.queue_item.upstream_branch_url:
            env['UPSTREAM_BRANCH_URL'] = self.queue_item.upstream_branch_url

        try:
            suite_config = get_suite_config(config, self.queue_item.suite)
        except KeyError:
            return JanitorResult(
                self.queue_item.package, log_id=self.log_id,
                code='unknown-suite',
                description='Suite %s not in configuration' %
                            self.queue_item.suite,
                logfilenames=[],
                branch_url=self.queue_item.branch_url)

        env.update(suite_build_env(suite_config, apt_location))

        if not use_cached_only:
            async with db.acquire() as conn:
                try:
                    main_branch = await open_canonical_main_branch(
                        conn, self.queue_item,
                        possible_transports=possible_transports)
                except BranchOpenFailure as e:
                    return JanitorResult(
                        self.queue_item.package, log_id=self.log_id,
                        branch_url=self.queue_item.branch_url,
                        description=e.description,
                        code=e.code, logfilenames=[])

            resume_branch = await open_resume_branch(
                main_branch, suite_config.branch_name,
                possible_hosters=possible_hosters)

            if resume_branch is None:
                resume_branch = vcs_manager.get_branch(
                    self.queue_item.package, suite_config.branch_name,
                    get_vcs_abbreviation(main_branch.repository))

            if resume_branch is not None:
                note('Resuming from %s', resume_branch.user_url)

            cached_branch_url = vcs_manager.get_branch_url(
                self.queue_item.package, 'master',
                get_vcs_abbreviation(main_branch.repository))
        else:
            main_branch = vcs_manager.get_branch(
                self.queue_item.package, 'master')
            if main_branch is None:
                return JanitorResult(
                    self.queue_item.package, log_id=self.log_id,
                    branch_url=self.queue_item.branch_url,
                    code='cached-branch-missing',
                    description='Missing cache branch for %s' %
                                self.queue_item.package,
                    logfilenames=[])
            note('Using cached branch %s', main_branch.user_url)
            resume_branch = vcs_manager.get_branch(
                self.queue_item.package, suite_config.branch_name)
            cached_branch_url = None

        if self.queue_item.refresh and resume_branch:
            note('Since refresh was requested, ignoring resume branch.')
            resume_branch = None

        async with db.acquire() as conn:
            (resume_branch, resume_branch_name,
             resume_branch_result) = await check_resume_result(
                conn, self.queue_item.suite, resume_branch)

            last_build_version = await state.get_last_build_version(
                conn, self.queue_item.package, self.queue_item.suite)

        log_path = os.path.join(self.output_directory, 'worker.log')
        try:
            self._task = asyncio.create_task(asyncio.wait_for(
                invoke_subprocess_worker(
                    worker_kind, main_branch, env, self.queue_item.command,
                    self.output_directory, resume_branch=resume_branch,
                    cached_branch_url=cached_branch_url, pre_check=pre_check,
                    post_check=post_check,
                    build_command=build_command,
                    log_path=log_path,
                    resume_branch_result=resume_branch_result,
                    last_build_version=last_build_version,
                    subpath=self.queue_item.subpath,
                    build_distribution=suite_config.build_distribution,
                    build_suffix=suite_config.build_suffix),
                timeout=overall_timeout), name=self.log_id)
            retcode = await self._task
        except asyncio.CancelledError:
            return JanitorResult(
                self.queue_item.package, log_id=self.log_id,
                branch_url=main_branch.user_url, code='cancelled',
                description='Job cancelled',
                logfilenames=[])
        except asyncio.TimeoutError:
            return JanitorResult(
                self.queue_item.package, log_id=self.log_id,
                branch_url=main_branch.user_url, code='timeout',
                description='Run timed out after %d seconds' %
                            overall_timeout,  # type: ignore
                logfilenames=[])

        logfilenames = await import_logs(
            self.output_directory, logfile_manager, self.queue_item.package,
            self.log_id)

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
                self.queue_item.package, log_id=self.log_id,
                branch_url=main_branch.user_url, code=code,
                description=description,
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
                self.queue_item.package, log_id=self.log_id,
                branch_url=main_branch.user_url, worker_result=worker_result,
                logfilenames=logfilenames, branch_name=(
                    resume_branch_name
                    if worker_result.code == 'nothing-to-do' else None))

        result = JanitorResult(
            self.queue_item.package, log_id=self.log_id,
            branch_url=main_branch.user_url,
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
                os.path.join(self.output_directory, self.queue_item.package))
        except (BranchMissing, BranchUnavailable) as e:
            return JanitorResult(
                self.queue_item.package, self.log_id, main_branch.user_url,
                description='result branch unavailable: %s' % e,
                code='result-branch-unavailable',
                worker_result=worker_result,
                logfilenames=logfilenames)

        enable_tag_pushing(local_branch)

        vcs_manager.import_branches(
            main_branch, local_branch,
            self.queue_item.package, suite_config.branch_name,
            additional_colocated_branches=(
                pick_additional_colocated_branches(main_branch)))
        result.branch_name = suite_config.branch_name

        if result.changes_filename:
            changes_path = os.path.join(
                self.output_directory, result.changes_filename)
            debsign(changes_path, debsign_keyid)
            if incoming_url is not None:
                run_incoming_url = urllib.parse.urljoin(
                    incoming_url, 'upload/%s' % self.log_id)
                try:
                    await upload_changes(changes_path, run_incoming_url)
                except UploadFailedError as e:
                    warning('Unable to upload changes file %s: %r',
                            result.changes_filename, e)
                    # TODO(jelmer): Copy to failed upload directory
        return result


async def export_queue_length(db: state.Database) -> None:
    while True:
        async with db.acquire() as conn:
            queue_length.set(await state.queue_length(conn))
            queue_duration.set(
                (await state.queue_duration(conn)).total_seconds())
            current_tick.set(await state.current_tick(conn))
        await asyncio.sleep(60)


async def export_stats(db: state.Database) -> None:
    while True:
        async with db.acquire() as conn:
            for suite, count in await state.get_published_by_suite(conn):
                apt_package_count.labels(suite=suite).set(count)

            by_suite: Dict[str, int] = {}
            by_suite_result: Dict[Tuple[str, str], int] = {}
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
            for review_status, count in await state.iter_review_status(conn):
                review_status_count.labels(review_status).set(count)

        # Every 30 minutes
        await asyncio.sleep(60 * 30)


class QueueProcessor(object):

    def __init__(
            self, database, config, worker_kind, build_command, pre_check=None,
            post_check=None, dry_run=False, incoming_url=None,
            logfile_manager=None, debsign_keyid=None, vcs_manager=None,
            public_vcs_manager=None, concurrency=1, use_cached_only=False,
            overall_timeout=None, committer=None, apt_location=None):
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
        self.public_vcs_manager = public_vcs_manager
        self.concurrency = concurrency
        self.use_cached_only = use_cached_only
        self.topic_queue = Topic(repeat_last=True)
        self.topic_result = Topic()
        self.overall_timeout = overall_timeout
        self.committer = committer
        self.active_runs = {}
        self.apt_location = apt_location

    def status_json(self) -> Any:
        return {
            'processing':
                [active_run.json()
                 for active_run in self.active_runs.values()],
            'concurrency': self.concurrency}

    async def process_queue_item(self, item: state.QueueItem) -> None:
        with tempfile.TemporaryDirectory() as output_directory:
            active_run = ActiveLocalRun(item, output_directory)
            self.register_run(active_run)
            result = await active_run.process(
                self.database, config=self.config,
                vcs_manager=self.vcs_manager,
                apt_location=self.apt_location,
                worker_kind=self.worker_kind,
                pre_check=self.pre_check,
                build_command=self.build_command, post_check=self.post_check,
                dry_run=self.dry_run, incoming_url=self.incoming_url,
                debsign_keyid=self.debsign_keyid,
                logfile_manager=self.logfile_manager,
                use_cached_only=self.use_cached_only,
                overall_timeout=self.overall_timeout,
                committer=self.committer)
            await self.finish_run(active_run, result)

    def register_run(self, active_run: ActiveRun) -> None:
        self.active_runs[active_run.log_id] = active_run
        self.topic_queue.publish(self.status_json())
        packages_processed_count.inc()

    async def finish_run(self,
                         active_run: ActiveRun,
                         result: JanitorResult) -> None:
        finish_time = datetime.now()
        item = active_run.queue_item
        duration = finish_time - active_run.start_time
        build_duration.labels(package=item.package, suite=item.suite).observe(
            duration.total_seconds())
        if result.changes_filename and item.suite != 'unchanged':
            async with self.database.acquire() as conn:
                run = await state.get_unchanged_run(
                    conn, result.main_branch_revision)
                if run is None:
                    note('Scheduling control run for %s.', item.package)
                    await state.add_to_queue(
                        conn, item.package, [
                            'just-build',
                            ('--revision=%s' %
                             result.main_branch_revision
                             .decode('utf-8'))
                        ],
                        'unchanged', offset=-10,
                        estimated_duration=duration, requestor='control')
        if not self.dry_run:
            async with self.database.acquire() as conn:
                await state.store_run(
                    conn, result.log_id, item.package, result.branch_url,
                    active_run.start_time, finish_time,
                    item.command, result.description, item.context,
                    result.context, result.main_branch_revision, result.code,
                    build_version=result.build_version,
                    build_distribution=result.build_distribution,
                    branch_name=result.branch_name, revision=result.revision,
                    subworker_result=result.subworker_result, suite=item.suite,
                    logfilenames=result.logfilenames, value=result.value,
                    worker_name=active_run.worker_name)
                await state.drop_queue_item(conn, item.id)
        self.topic_result.publish(result.json())
        del self.active_runs[active_run.log_id]
        self.topic_queue.publish(self.status_json())
        last_success_gauge.set_to_current_time()

    async def next_queue_item(self, n) -> List[state.QueueItem]:
        ret: List[state.QueueItem] = []
        async with self.database.acquire() as conn:
            limit = len(self.active_runs) + n + 2
            async for item in state.iter_queue(conn, limit=limit):
                if self.queue_item_assigned(item):
                    continue
                if len(ret) < n:
                    ret.append(item)
            return ret

    async def process(self) -> None:
        todo = set([
            self.process_queue_item(item)
            for item in await self.next_queue_item(self.concurrency)])

        def handle_sigterm():
            self.concurrency = None
            note('Received SIGTERM; not starting new jobs.')

        loop = asyncio.get_event_loop()
        loop.add_signal_handler(signal.SIGTERM, handle_sigterm)
        try:
            while True:
                if not todo:
                    if self.concurrency is None:
                        break
                    note('Nothing to do. Sleeping for 60s.')
                    await asyncio.sleep(60)
                    continue
                done, pending = await asyncio.wait(
                    todo, return_when='FIRST_COMPLETED')
                for task in done:
                    task.result()
                todo = pending  # type: ignore
                if self.concurrency:
                    todo.update([
                        self.process_queue_item(item)
                        for item in await self.next_queue_item(len(done))])
        finally:
            loop.remove_signal_handler(signal.SIGTERM)

    def queue_item_assigned(self, queue_item: state.QueueItem) -> bool:
        """Check if a queue item has been assigned already."""
        for active_run in self.active_runs.values():
            if active_run.queue_item.id == queue_item.id:
                return True
        return False


async def handle_status(request):
    queue_processor = request.app.queue_processor
    return web.json_response(queue_processor.status_json())


async def handle_log_index(request):
    queue_processor = request.app.queue_processor
    run_id = request.match_info['run_id']
    try:
        active_run = queue_processor.active_runs[run_id]
    except KeyError:
        return web.Response(
            text='No such current run: %s' % run_id, status=404)
    log_filenames = active_run.list_log_files()
    return web.json_response(log_filenames)


async def handle_kill(request):
    queue_processor = request.app.queue_processor
    run_id = request.match_info['run_id']
    try:
        ret = queue_processor.active_runs[run_id].json()
        queue_processor.active_runs[run_id].kill()
    except KeyError:
        return web.Response(
            text='No such current run: %s' % run_id, status=404)
    return web.json_response(ret)


async def handle_progress_ws(request):
    queue_processor = request.app.queue_processor
    run_id = request.match_info['run_id']
    try:
        active_run = queue_processor.active_runs[run_id]
    except KeyError:
        return web.Response(
            text='No such current run: %s' % run_id, status=404)

    ws = web.WebSocketResponse()
    await ws.prepare(request)

    active_run.websockets.append(ws)

    async for msg in ws:
        if msg.type == WSMsgType.TEXT:
            pass  # TODO(jelmer): Process

    return ws


async def handle_log(request):
    queue_processor = request.app.queue_processor
    run_id = request.match_info['run_id']
    filename = request.match_info['filename']
    if '/' in filename:
        return web.Response(
            text='Invalid filename %s' % filename,
            status=400)
    try:
        active_run = queue_processor.active_runs[run_id]
    except KeyError:
        return web.Response(
            text='No such current run: %s' % run_id, status=404)
    try:
        f = active_run.get_log_file(filename)
    except FileNotFoundError:
        return web.Response(text='No such log file: %s' % filename, status=404)

    try:
        response = web.StreamResponse(
            status=200, reason='OK', headers={'Content-Type', 'text/plain'})
        await response.prepare(request)
        for chunk in f:
            await response.write(chunk)
        await response.write_eof()
    finally:
        f.close()
    return response


async def handle_assign(request):
    json = await request.json()
    worker = json['worker']

    possible_transports = []
    possible_hosters = []

    queue_processor = request.app.queue_processor
    [item] = await queue_processor.next_queue_item(1)

    active_run = ActiveRemoteRun(worker_name=worker, queue_item=item)

    queue_processor.register_run(active_run)

    suite_config = get_suite_config(queue_processor.config, item.suite)

    async with queue_processor.database.acquire() as conn:
        if item.branch_url is None:
            await state.drop_queue_item(conn, item.id)
            return web.json_response({'queue_id': item.queue_id}, status=503)
        last_build_version = await state.get_last_build_version(
            conn, item.package, item.suite)

        try:
            main_branch = await open_canonical_main_branch(
                conn, item,
                possible_transports=possible_transports)
        except BranchOpenFailure:
            resume_branch = None
            vcs_type = item.vcs_type
        else:
            active_run.main_branch_url = main_branch.user_url
            vcs_type = get_vcs_abbreviation(main_branch.repository)
            if not item.refresh:
                resume_branch = await open_resume_branch(
                    main_branch, suite_config.branch_name,
                    possible_hosters=possible_hosters)
            else:
                resume_branch = None

        assert vcs_type in ('bzr', 'git')

        if resume_branch is None and not item.refresh:
            resume_branch = queue_processor.vcs_manager.get_branch(
                item.package, suite_config.branch_name, vcs_type)

        (resume_branch, active_run.resume_branch_name,
         resume_branch_result) = await check_resume_result(
            conn, item.suite, resume_branch)

        if resume_branch is not None:
            resume_branch_url = (
                queue_processor.public_vcs_manager.get_branch_url(
                    item.package, suite_config.branch_name, vcs_type))
            resume = {
                'result': resume_branch_result,
                'branch_url': resume_branch_url,
                'branch_name': active_run.resume_branch_name,
            }
        else:
            resume = None

    cached_branch_url = queue_processor.public_vcs_manager.get_branch_url(
        item.package, 'master', vcs_type)

    env = {
        'PACKAGE': item.package,
        }
    if queue_processor.committer:
        (user, email) = parseaddr(queue_processor.committer)
        if user:
            env['DEBFULLNAME'] = user
        if email:
            env['DEBEMAIL'] = email
        env['COMMITTER'] = queue_processor.committer
    if item.upstream_branch_url:
        env['UPSTREAM_BRANCH_URL'] = item.upstream_branch_url,

    result_branch_url = queue_processor.public_vcs_manager.get_branch_url(
        item.package, suite_config.branch_name, vcs_type.lower())

    assignment = {
        'id': active_run.log_id,
        'description': '%s on %s' % (item.suite, item.package),
        'queue_id': item.id,
        'branch': {
            'url': active_run.main_branch_url,
            'subpath': item.subpath,
            'vcs_type': item.vcs_type,
            'cached_url': cached_branch_url,
        },
        'resume': resume,
        'last_build_version': last_build_version,
        'build': {
            'distribution': suite_config.build_distribution,
            'suffix': suite_config.build_suffix,
            'environment':
                suite_build_env(suite_config, queue_processor.apt_location),
        },
        'env': env,
        'command': item.command,
        'suite': item.suite,
        'result_branch': {
            'url': result_branch_url,
         },
        }

    return web.json_response(assignment, status=201)


async def handle_finish(request):
    queue_processor = request.app.queue_processor
    run_id = request.match_info['run_id']
    try:
        active_run = queue_processor.active_runs[run_id]
    except KeyError:
        return web.json_response(
            {'reason': 'No such current run: %s' % run_id}, status=404)

    reader = await request.multipart()
    worker_result = None

    with tempfile.TemporaryDirectory() as output_directory:
        filenames = []
        while True:
            part = await reader.next()
            if part is None:
                break
            if part.filename == 'result.json':
                worker_result = WorkerResult.from_json(await part.json())
            else:
                filenames.append(part.filename)
                output_path = os.path.join(output_directory, part.filename)
                with open(output_path, 'wb') as f:
                    f.write(await part.read())

        if worker_result is None:
            return web.json_response(
                {'reason': 'Missing result JSON'}, status=400)

        logfilenames = await import_logs(
            output_directory, queue_processor.logfile_manager,
            active_run.queue_item.package, run_id)

        if worker_result.code is not None:
            result = JanitorResult(
                active_run.queue_item.package, log_id=run_id,
                branch_url=active_run.main_branch_url,
                worker_result=worker_result,
                logfilenames=logfilenames, branch_name=(
                    active_run.resume_branch_name
                    if worker_result.code == 'nothing-to-do' else None))
        else:
            result = JanitorResult(
                active_run.queue_item.package, log_id=run_id,
                branch_url=active_run.main_branch_url,
                code='success', worker_result=worker_result,
                logfilenames=logfilenames)

    await queue_processor.finish_run(active_run, result)
    return web.json_response(
        {'id': active_run.log_id,
         'filenames': filenames,
         'result': result.json()}, status=201)


async def run_web_server(listen_addr, port, queue_processor):
    app = web.Application()
    app.queue_processor = queue_processor
    setup_metrics(app)
    app.router.add_get('/status', handle_status)
    app.router.add_get('/log/{run_id}', handle_log_index)
    app.router.add_get('/log/{run_id}/{filename}', handle_log)
    app.router.add_post('/kill/{run_id}', handle_kill)
    app.router.add_get('/progress-ws/{run_id}', handle_progress_ws)
    app.router.add_get('/ws/queue', functools.partial(
        pubsub_handler, queue_processor.topic_queue))
    app.router.add_get('/ws/result', functools.partial(
        pubsub_handler, queue_processor.topic_result))
    app.router.add_post('/assign', handle_assign)
    app.router.add_post('/finish/{run_id}', handle_finish)
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
        '--debsign-keyid', type=str, default=None,
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
    public_vcs_manager = RemoteVcsManager()
    if config.vcs_location:
        vcs_manager = LocalVcsManager(config.vcs_location)
    else:
        vcs_manager = public_vcs_manager
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
        public_vcs_manager,
        args.concurrency,
        args.use_cached_only,
        overall_timeout=args.overall_timeout,
        committer=config.committer,
        apt_location=config.apt_location)
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
