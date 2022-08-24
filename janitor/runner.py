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

import asyncio
from contextlib import AsyncExitStack
from datetime import datetime, timedelta
from email.utils import parseaddr
import functools
import json
from io import BytesIO
import logging
import os
import ssl
import sys
import tempfile
from typing import List, Any, Optional, Dict, Tuple, Type, Set
import uuid
import warnings

import aioredis
import aiozipkin
import asyncpg
import asyncpg.pool

from aiohttp import (
    web,
    ClientOSError,
    ClientSession,
    ClientTimeout,
    ClientConnectorError,
    ClientResponseError,
    ServerDisconnectedError,
    )

from yarl import URL

from aiohttp_openmetrics import Counter, Gauge, Histogram, setup_metrics

from breezy import debug, urlutils
from breezy.branch import Branch
from breezy.errors import PermissionDenied, ConnectionError, UnexpectedHttpStatus
from breezy.transport import UnusableRedirect

from silver_platter.debian import (
    select_preferred_probers,
)
from silver_platter.proposal import (
    Forge,
    find_existing_proposed,
    UnsupportedForge,
    ForgeLoginRequired,
    NoSuchProject,
    get_forge,
)
from silver_platter.utils import (
    BranchRateLimited,
    full_branch_url,
)

from . import (
    state,
    splitout_env,
)
from .artifacts import (
    ArtifactManager,
    get_artifact_manager,
    LocalArtifactManager,
    store_artifacts_with_backup,
    upload_backup_artifacts,
)
from .compat import to_thread
from .config import read_config, get_campaign_config, get_distribution, Config, Campaign
from .debian import (
    changes_filenames,
    find_changes,
    NoChangesFile,
    dpkg_vendor,
)
from .logs import (
    get_log_manager,
    ServiceUnavailable,
    LogFileManager,
    FileSystemLogFileManager,
)
from .policy import read_policy, PolicyConfig
from .pubsub import Topic, pubsub_handler
from .queue import QueueItem, Queue
from .schedule import do_schedule_control, do_schedule
from .vcs import (
    get_vcs_abbreviation,
    is_authenticated_url,
    open_branch_ext,
    BranchOpenFailure,
    get_vcs_managers_from_config,
    get_vcs_managers,
    UnsupportedVcs,
    VcsManager,
)

DEFAULT_RETRY_AFTER = 120
REMOTE_BRANCH_OPEN_TIMEOUT = 10.0
VCS_STORE_BRANCH_OPEN_TIMEOUT = 5.0


routes = web.RouteTableDef()
packages_processed_count = Counter("package_count", "Number of packages processed.")
last_success_gauge = Gauge(
    "job_last_success_unixtime", "Last time a batch job successfully finished"
)
build_duration = Histogram("build_duration", "Build duration", ["package", "suite"])
run_result_count = Counter("result", "Result counts", ["package", "suite", "result_code"])
active_run_count = Gauge("active_runs", "Number of active runs", ["worker"])
assignment_count = Counter("assignments", "Number of assignments handed out", ["worker"])
rate_limited_count = Counter("rate_limited_host", "Rate limiting per host", ["host"])
artifact_upload_failed_count = Counter(
    "artifact_upload_failed", "Number of failed artifact uploads")
primary_logfile_upload_failed = Counter(
    "primary_logfile_upload_failed", "Number of failed logs to primary logfile target")


async def to_thread_timeout(timeout, func, *args, **kwargs):
    cor = to_thread(func, *args, **kwargs)
    if timeout is not None:
        cor = asyncio.wait_for(cor, timeout=timeout)
    return await cor


class BuilderResult(object):

    kind: str

    def from_directory(self, path):
        raise NotImplementedError(self.from_directory)

    async def store(self, conn, run_id):
        raise NotImplementedError(self.store)

    def json(self):
        raise NotImplementedError(self.json)

    def artifact_filenames(self):
        raise NotImplementedError(self.artifact_filenames)


class Builder(object):
    """Abstract builder class."""

    kind: str

    result_cls: Type[BuilderResult] = BuilderResult

    async def build_env(
            self, conn: asyncpg.Connection,
            campaign_config: Campaign, queue_item: QueueItem) -> Dict[str, str]:
        raise NotImplementedError(self.build_env)


class GenericResult(BuilderResult):
    """Generic build result."""

    kind = "generic"

    @classmethod
    def from_json(cls, target_details):
        return cls()

    def from_directory(self, path):
        pass

    def json(self):
        return {}

    def artifact_filenames(self):
        return []

    async def store(self, conn, run_id):
        pass


class GenericBuilder(Builder):
    """Generic builder."""

    kind = "generic"

    result_cls = GenericResult

    def __init__(self):
        pass

    async def build_env(self, conn, campaign_config, queue_item):
        env = {}
        if campaign_config.generic_build.chroot:
            env["CHROOT"] = campaign_config.generic_build.chroot

        return env


class DebianResult(BuilderResult):

    kind = "debian"

    def __init__(
        self, source=None, build_version=None, build_distribution=None,
        changes_filenames=None, lintian_result=None, binary_packages=None
    ):
        self.source = source
        self.build_version = build_version
        self.build_distribution = build_distribution
        self.binary_packages = binary_packages
        self.changes_filenames = changes_filenames
        self.lintian_result = lintian_result

    def from_directory(self, path):
        try:
            self.output_directory = path
            (
                self.changes_filenames,
                self.source,
                self.build_version,
                self.build_distribution,
                self.binary_packages
            ) = find_changes(path)
        except NoChangesFile as e:
            # Oh, well.
            logging.info("No changes file found: %s", e)
        else:
            logging.info(
                "Found changes files %r, source %s, build version %s, "
                "distribution: %s, binary packages: %r",
                self.source, self.changes_filenames, self.build_version,
                self.build_distribution, self.binary_packages)

    def artifact_filenames(self):
        if not self.changes_filenames:
            return []
        ret = []
        for changes_filename in self.changes_filenames:
            changes_path = os.path.join(self.output_directory, changes_filename)
            ret.extend(changes_filenames(changes_path))
            ret.append(changes_filename)
        return ret

    @classmethod
    def from_json(cls, target_details):
        return cls(lintian_result=target_details.get('lintian'))

    async def store(self, conn, run_id):
        if self.build_version:
            await conn.execute(
                "INSERT INTO debian_build (run_id, source, version, distribution, lintian_result, binary_packages) "
                "VALUES ($1, $2, $3, $4, $5, $6)",
                run_id,
                self.source,
                self.build_version,
                self.build_distribution,
                self.lintian_result,
                self.binary_packages
            )

    def json(self):
        return {
            "build_distribution": self.build_distribution,
            "build_version": self.build_version,
            "changes_filenames": self.changes_filenames,
            "lintian": self.lintian_result,
            "binary_packages": self.binary_packages,
        }

    def __bool__(self):
        return self.changes_filenames is not None


class DebianBuilder(Builder):

    kind = "debian"

    result_cls = DebianResult

    def __init__(self, distro_config, apt_location):
        self.distro_config = distro_config
        self.apt_location = apt_location

    async def build_env(self, conn, campaign_config, queue_item):
        if self.apt_location.startswith("gs://"):
            bucket_name = URL(self.apt_location).host
            apt_location = "https://storage.googleapis.com/%s/" % bucket_name
        else:
            apt_location = self.apt_location
        extra_janitor_distributions = list(campaign_config.debian_build.extra_build_distribution)
        if queue_item.change_set:
            extra_janitor_distributions.append('cs/%s' % queue_item.change_set)
        env = {
            "EXTRA_REPOSITORIES": ":".join(
                [
                    "deb %s %s/ main" % (apt_location, suite)
                    for suite in extra_janitor_distributions
                ]
            )
        }

        if campaign_config.debian_build.chroot:
            env["CHROOT"] = campaign_config.debian_build.chroot
        elif self.distro_config.chroot:
            env["CHROOT"] = self.distro_config.chroot

        if self.distro_config.name:
            env["DISTRIBUTION"] = self.distro_config.name

        env["REPOSITORIES"] = "%s %s/ %s" % (
            self.distro_config.archive_mirror_uri,
            self.distro_config.name,
            " ".join(self.distro_config.component),
        )

        env["BUILD_DISTRIBUTION"] = campaign_config.debian_build.build_distribution or campaign_config.name
        env["BUILD_SUFFIX"] = campaign_config.debian_build.build_suffix or ""

        if campaign_config.debian_build.build_command:
            env["BUILD_COMMAND"] = campaign_config.debian_build.build_command
        elif self.distro_config.build_command:
            env["BUILD_COMMAND"] = self.distro_config.build_command

        last_build_version = await conn.fetchval(
            "SELECT version FROM debian_build WHERE "
            "version IS NOT NULL AND source = $1 AND "
            "distribution = $2 ORDER BY version DESC LIMIT 1",
            queue_item.package, env['BUILD_DISTRIBUTION']
        )

        if last_build_version:
            env["LAST_BUILD_VERSION"] = str(last_build_version)

        env['LINTIAN_PROFILE'] = self.distro_config.lintian_profile
        if self.distro_config.lintian_suppress_tag:
            env['LINTIAN_SUPPRESS_TAGS'] = ','.join(self.distro_config.lintian_suppress_tag)

        env.update([(env.key, env.value) for env in campaign_config.debian_build.sbuild_env])

        env['DEB_VENDOR'] = self.distro_config.vendor or dpkg_vendor()

        upstream_branch_url = await conn.fetchval(
            "SELECT upstream_branch_url FROM upstream WHERE name = $1",
            queue_item.package)
        if upstream_branch_url:
            env["UPSTREAM_BRANCH_URL"] = upstream_branch_url

        return env


BUILDER_CLASSES: List[Type[Builder]] = [DebianBuilder, GenericBuilder]
RESULT_CLASSES = [builder_cls.result_cls for builder_cls in BUILDER_CLASSES]


def get_builder(config, campaign_config):
    if campaign_config.HasField('debian_build'):
        distribution = get_distribution(
            config, campaign_config.debian_build.base_distribution)
        return DebianBuilder(
            distribution,
            config.apt_location
            )
    elif campaign_config.HasField('generic_build'):
        return GenericBuilder()
    else:
        raise NotImplementedError('no supported build type')


class JanitorResult(object):

    package: str
    log_id: str
    branch_url: str
    code: str

    def __init__(
        self,
        pkg: str,
        log_id: str,
        branch_url: str,
        code: str,
        description: Optional[str] = None,
        worker_result=None,
        logfilenames=None,
        suite=None,
        start_time=None,
        finish_time=None,
        worker_name=None,
        vcs_type=None,
        resume_from=None,
        change_set=None,
    ):
        self.package = pkg
        self.suite = suite
        self.log_id = log_id
        self.description = description
        self.branch_url = branch_url
        self.code = code
        self.logfilenames = logfilenames or []
        self.worker_name = worker_name
        self.vcs_type = vcs_type
        self.change_set = change_set
        if worker_result is not None:
            self.context = worker_result.context
            self.code = worker_result.code or code
            if self.description is None:
                self.description = worker_result.description
            self.main_branch_revision = worker_result.main_branch_revision
            self.subworker_result = worker_result.subworker
            self.revision = worker_result.revision
            self.value = worker_result.value
            self.builder_result = worker_result.builder_result
            self.branches = worker_result.branches
            self.tags = worker_result.tags
            self.remotes = worker_result.remotes
            self.failure_details = worker_result.details
            self.start_time = worker_result.start_time
            self.finish_time = worker_result.finish_time
            self.followup_actions = worker_result.followup_actions
            if worker_result.refreshed:
                self.resume_from = None
            else:
                self.resume_from = resume_from
            self.target_branch_url = worker_result.target_branch_url
        else:
            self.start_time = start_time
            self.finish_time = finish_time
            self.context = None
            self.main_branch_revision = None
            self.revision = None
            self.subworker_result = None
            self.value = None
            self.builder_result = None
            self.branches = None
            self.tags = None
            self.failure_details = None
            self.target_branch_url = None
            self.remotes = {}
            self.followup_actions = []
            self.resume_from = None

    @property
    def duration(self):
        return self.finish_time - self.start_time

    def json(self):
        return {
            "package": self.package,
            "suite": self.suite,
            "log_id": self.log_id,
            "description": self.description,
            "code": self.code,
            "failure_details": self.failure_details,
            "target": ({
                "name": self.builder_result.kind,
                "details": self.builder_result.json(),
            } if self.builder_result else {}),
            "logfilenames": self.logfilenames,
            "subworker": self.subworker_result,
            "value": self.value,
            "remotes": self.remotes,
            "resume": {"run_id": self.resume_from} if self.resume_from else None,
            "branches": (
                [
                    (fn, n, br.decode("utf-8") if br else None,
                     r.decode("utf-8") if r else None)
                    for (fn, n, br, r) in self.branches
                ]
                if self.branches is not None
                else None
            ),
            "tags": (
                [(n, r.decode("utf-8")) for (n, r) in self.tags]
                if self.tags is not None
                else None
            ),
            "revision": self.revision.decode("utf-8") if self.revision else None,
            "main_branch_revision": self.main_branch_revision.decode("utf-8")
            if self.main_branch_revision
            else None,
        }


def committer_env(committer):
    env = {}
    if not committer:
        return env
    (user, email) = parseaddr(committer)
    if user:
        env["DEBFULLNAME"] = user
    if email:
        env["DEBEMAIL"] = email
    env["COMMITTER"] = committer
    env["BRZ_EMAIL"] = committer
    env["GIT_COMMITTER_NAME"] = user
    env["GIT_COMMITTER_EMAIL"] = email
    env["GIT_AUTHOR_NAME"] = user
    env["GIT_AUTHOR_EMAIL"] = email
    env["EMAIL"] = email
    return env


class WorkerResult(object):
    """The result from a worker."""

    def __init__(
        self,
        code: str,
        description: Optional[str],
        context=None,
        subworker=None,
        main_branch_revision=None,
        revision=None,
        value=None,
        branches=None,
        tags=None,
        remotes=None,
        details=None,
        builder_result=None,
        start_time=None,
        finish_time=None,
        queue_id=None,
        worker_name=None,
        followup_actions=None,
        refreshed=False,
        target_branch_url=None,
    ):
        self.code = code
        self.description = description
        self.context = context
        self.subworker = subworker
        self.main_branch_revision = main_branch_revision
        self.revision = revision
        self.value = value
        self.branches = branches
        self.tags = tags
        self.remotes = remotes
        self.details = details
        self.builder_result = builder_result
        self.start_time = start_time
        self.finish_time = finish_time
        self.queue_id = queue_id
        self.worker_name = worker_name
        self.followup_actions = followup_actions
        self.refreshed = refreshed
        self.target_branch_url = target_branch_url

    @classmethod
    def from_file(cls, path):
        """create a WorkerResult object from a JSON file."""
        with open(path, "r") as f:
            worker_result = json.load(f)
        return cls.from_json(worker_result)

    @classmethod
    def from_json(cls, worker_result):
        main_branch_revision = worker_result.get("main_branch_revision")
        if main_branch_revision is not None:
            main_branch_revision = main_branch_revision.encode("utf-8")
        revision = worker_result.get("revision")
        if revision is not None:
            revision = revision.encode("utf-8")
        branches = worker_result.get("branches")
        tags = worker_result.get("tags")
        if branches:
            branches = [
                (fn, n, br.encode("utf-8") if br else None, r.encode("utf-8"))
                for (fn, n, br, r) in branches
            ]
        if tags:
            tags = [(n, r.encode("utf-8")) for (n, r) in tags]
        target_kind = worker_result.get("target", {"name": None})["name"]
        for result_cls in RESULT_CLASSES:
            if target_kind == result_cls.kind:
                target_details = worker_result["target"]["details"]
                builder_result = result_cls.from_json(target_details)
                break
        else:
            if target_kind is None:
                builder_result = None
            else:
                raise NotImplementedError('unsupported build target %r' % target_kind)
        return cls(
            code=worker_result.get("code", "missing-result-code"),
            description=worker_result.get("description"),
            context=worker_result.get("context"),
            subworker=worker_result.get("subworker"),
            main_branch_revision=main_branch_revision,
            revision=revision,
            value=worker_result.get("value"),
            branches=branches,
            tags=tags,
            remotes=worker_result.get("remotes"),
            details=worker_result.get("details"),
            builder_result=builder_result,
            start_time=datetime.fromisoformat(worker_result['start_time'])
            if 'start_time' in worker_result else None,
            finish_time=datetime.fromisoformat(worker_result['finish_time'])
            if 'finish_time' in worker_result else None,
            queue_id=worker_result.get("queue_id"),
            worker_name=worker_result.get("worker_name"),
            followup_actions=worker_result.get("followup_actions"),
            refreshed=worker_result.get("refreshed", False),
            target_branch_url=worker_result.get("target_branch_url", None),
        )


async def update_branch_url(
    conn: asyncpg.Connection, package: str, vcs_type: str, vcs_url: str
) -> None:
    await conn.execute(
        "update package set vcs_type = $1, branch_url = $2 " "where name = $3",
        vcs_type.lower(),
        vcs_url,
        package,
    )


def create_background_task(fn, title):
    loop = asyncio.get_event_loop()
    task = loop.create_task(fn)

    def log_result(future):
        try:
            future.result()
        except asyncio.CancelledError:
            logging.debug('%s cancelled', title)
        except BaseException:
            logging.exception('%s failed', title)
        else:
            logging.debug('%s succeeded', title)
    task.add_done_callback(log_result)
    return task


def gather_logs(output_directory: str):
    for entry in os.scandir(output_directory):
        if entry.is_dir():
            continue
        parts = entry.name.split(".")
        if parts[-1] == "log" or (
            len(parts) == 3 and parts[-2] == "log" and parts[-1].isdigit()
        ):
            yield entry


async def import_logs(
    entries,
    logfile_manager: LogFileManager,
    backup_logfile_manager: Optional[LogFileManager],
    pkg: str,
    log_id: str,
    mtime: Optional[int] = None,
):
    for entry in entries:
        try:
            await logfile_manager.import_log(pkg, log_id, entry.path, mtime=mtime)
        except ServiceUnavailable as e:
            logging.warning("Unable to upload logfile %s: %s", entry.name, e)
            if backup_logfile_manager:
                primary_logfile_upload_failed.inc()
                await backup_logfile_manager.import_log(pkg, log_id, entry.path, mtime=mtime)
        except asyncio.TimeoutError as e:
            logging.warning("Timeout uploading logfile %s: %s", entry.name, e)
            if backup_logfile_manager:
                primary_logfile_upload_failed.inc()
                await backup_logfile_manager.import_log(pkg, log_id, entry.path, mtime=mtime)
        except PermissionDenied as e:
            logging.warning(
                "Permission denied error while uploading logfile %s: %s",
                entry.name, e)
            if backup_logfile_manager:
                primary_logfile_upload_failed.inc()
                await backup_logfile_manager.import_log(pkg, log_id, entry.path, mtime=mtime)


class ActiveRun(object):

    worker_name: str
    worker_link: Optional[str]
    queue_item: QueueItem
    log_id: str
    start_time: datetime
    last_keepalive: datetime

    def __init__(
        self,
        queue_item: QueueItem,
        vcs_info: Dict[str, str],
        worker_name: str,
        worker_link: Optional[str] = None,
    ):
        self.queue_item = queue_item
        self.start_time = datetime.utcnow()
        self.log_id = str(uuid.uuid4())
        self.worker_name = worker_name
        self.vcs_info = vcs_info
        self.resume_branch_name = None
        self.worker_link = worker_link
        self.resume_from = None
        self._reset_keepalive()

    def _reset_keepalive(self):
        self.last_keepalive = datetime.utcnow()

    @property
    def current_duration(self):
        return datetime.utcnow() - self.start_time

    async def kill(self) -> None:
        raise NotImplementedError(self.kill)

    async def list_log_files(self):
        raise NotImplementedError(self.list_log_files)

    async def get_log_file(self, name):
        raise NotImplementedError(self.get_log_file)

    def create_result(self, **kwargs):
        return JanitorResult(
            pkg=self.queue_item.package,
            suite=self.queue_item.suite,
            start_time=self.start_time,
            finish_time=datetime.utcnow(),
            log_id=self.log_id,
            worker_name=self.worker_name,
            resume_from=self.resume_from,
            change_set=self.queue_item.change_set,
            **kwargs)

    @property
    def keepalive_age(self):
        return datetime.utcnow() - self.last_keepalive

    @property
    def is_mia(self):
        return self.keepalive_age.total_seconds() > 60 * 60

    @property
    def vcs_type(self):
        return self.vcs_info["vcs_type"]

    @property
    def main_branch_url(self):
        return self.vcs_info["main_branch_url"]

    def json(self) -> Any:
        """Return a JSON representation."""
        return {
            "queue_id": self.queue_item.id,
            "id": self.log_id,
            "package": self.queue_item.package,
            "suite": self.queue_item.suite,
            "estimated_duration": self.queue_item.estimated_duration.total_seconds()
            if self.queue_item.estimated_duration
            else None,
            "current_duration": self.current_duration.total_seconds(),
            "start_time": self.start_time.isoformat(),
            "worker": self.worker_name,
            "worker_link": self.worker_link,
            "last-keepalive": self.last_keepalive.isoformat(
                timespec='seconds'),
            "keepalive_age": self.keepalive_age.total_seconds(),
            "mia": self.is_mia,
        }

    def start_watchdog(self, queue_processor):
        pass

    def stop_watchdog(self):
        pass


class JenkinsRun(ActiveRun):

    KEEPALIVE_INTERVAL = 10
    KEEPALIVE_TIMEOUT = 60

    def __init__(self, my_url: URL, *args, **kwargs):
        super(JenkinsRun, self).__init__(*args, **kwargs)
        self.my_url = my_url
        self._watch_dog = None
        self._metadata = None

    def __repr__(self):
        return "<%s(%r)>" % (type(self).__name__, self.my_url)

    async def kill(self) -> None:
        raise NotImplementedError(self.kill)

    async def list_log_files(self):
        return ['worker.log']

    async def get_log_file(self, name):
        if name != 'worker.log':
            raise FileNotFoundError(name)
        async with ClientSession() as session, \
                session.get(
                    self.my_url / 'logText/progressiveText',
                    raise_for_status=True) as resp:
            return BytesIO(await resp.read())

    def start_watchdog(self, queue_processor):
        if self._watch_dog is not None:
            raise Exception("Watchdog already started")
        self._watch_dog = create_background_task(
            self._watchdog(queue_processor), 'watchdog for %r' % self)

    def stop_watchdog(self):
        if self._watch_dog is None:
            return
        try:
            self._watch_dog.cancel()
        except asyncio.CancelledError:
            pass
        self._watch_dog = None

    async def _get_job(self, session):
        async with session.get(
                self.my_url / 'api/json', raise_for_status=True,
                timeout=ClientTimeout(self.KEEPALIVE_TIMEOUT)) as resp:
            return await resp.json()

    async def _watchdog(self, queue_processor):
        health_url = self.my_url / 'log-id'
        logging.info('Pinging URL %s', health_url)
        async with ClientSession() as session:
            while True:
                try:
                    await self._get_job(session)
                except (ClientConnectorError, ServerDisconnectedError,
                        asyncio.TimeoutError, ClientOSError) as e:
                    logging.warning('Failed to ping client %s: %s', self.my_url, e)
                except ClientResponseError as e:
                    if e.status == 404:
                        logging.warning(
                            "Jenkins job %s (worker %s) for run %s has disappeared.", self.my_url,
                            self.worker_name, self.log_id)
                        await queue_processor.abort_run(
                            self, 'run-disappeared',
                            'Jenkins job %s has disappeared' % self.my_url)
                        break
                    else:
                        logging.warning('Failed to ping client %s: %s', self.my_url, e)
                else:
                    self._reset_keepalive()
                if self.keepalive_age > timedelta(seconds=queue_processor.run_timeout * 60):
                    logging.warning(
                        "No keepalives received from %s for %s in %s, aborting.",
                        self.worker_name,
                        self.log_id,
                        self.keepalive_age,
                    )
                    try:
                        await queue_processor.timeout_run(self, self.keepalive_age)
                    except RunExists:
                        logging.warning('Watchdog was not stopped?')
                    break
                await asyncio.sleep(self.KEEPALIVE_INTERVAL)

    def json(self):
        ret = super(JenkinsRun, self).json()
        ret['jenkins'] = self._metadata
        return ret


class PollingActiveRun(ActiveRun):

    KEEPALIVE_INTERVAL = 10
    KEEPALIVE_TIMEOUT = 60

    def __init__(self, my_url: URL, *args, **kwargs):
        super(PollingActiveRun, self).__init__(*args, **kwargs)
        self.my_url = my_url
        self._watch_dog = None
        self._log_id_mismatch = None

    def __repr__(self):
        return "<%s(%r)>" % (type(self).__name__, self.my_url)

    async def kill(self) -> None:
        async with ClientSession() as session, \
                session.post(
                    self.my_url / 'kill', headers={
                        'Accept': 'application/json'},
                    raise_for_status=True):
            pass

    async def list_log_files(self):
        # TODO(jelmer)
        async with ClientSession() as session, \
                session.get(
                    self.my_url / 'logs', headers={
                        'Accept': 'application/json'},
                    raise_for_status=True) as resp:
            return await resp.json()

    async def get_log_file(self, name):
        async with ClientSession() as session, \
                session.get(
                    self.my_url / 'logs' / name,
                    raise_for_status=True) as resp:
            return BytesIO(await resp.read())

    def start_watchdog(self, queue_processor):
        if self._watch_dog is not None:
            raise Exception("Watchdog already started")
        self._watch_dog = create_background_task(
            self._watchdog(queue_processor), 'watchdog for %r' % self)

    def stop_watchdog(self):
        if self._watch_dog is None:
            return
        try:
            self._watch_dog.cancel()
        except asyncio.CancelledError:
            pass
        self._watch_dog = None

    @property
    def is_mia(self):
        if super(PollingActiveRun, self).is_mia:
            return True
        return self._log_id_mismatch is not None

    async def _watchdog(self, queue_processor):
        health_url = self.my_url / 'log-id'
        logging.info('Pinging URL %s', health_url)
        async with ClientSession() as session:
            while True:
                try:
                    async with session.get(
                            health_url, raise_for_status=True,
                            timeout=ClientTimeout(queue_processor.run_timeout * 60)) as resp:
                        log_id = (await resp.read()).decode()
                        if log_id != self.log_id:
                            logging.warning('Unexpected log id %s != %s', log_id, self.log_id)
                            self._log_id_mismatch = log_id
                        else:
                            self._reset_keepalive()
                            self._log_id_mismatch = None
                except (ClientConnectorError, ClientResponseError,
                        asyncio.TimeoutError, ClientOSError,
                        ServerDisconnectedError) as e:
                    logging.warning('Failed to ping client %s: %s', self.my_url, e)
                if self.keepalive_age > timedelta(seconds=queue_processor.run_timeout * 60):
                    logging.warning(
                        "No health checks to %s succeeded for %s in %s, aborting.",
                        self.worker_name,
                        self.log_id,
                        self.keepalive_age,
                    )
                    try:
                        await queue_processor.timeout_run(self, self.keepalive_age)
                    except RunExists:
                        logging.warning('Watchdog was not stopped?')
                    break
                if (self._log_id_mismatch is not None and
                        (datetime.now() - self.start_time).total_seconds() > 30):
                    logging.warning(
                        "Worker %s is now processing new run %s (age: %s). Marking run as MIA.",
                        self.worker_name,
                        self.log_id,
                        self.keepalive_age,
                    )
                    try:
                        await queue_processor.abort_run(
                            self, 'run-disappeared',
                            'Worker started processing new run %s rather than %s' %
                            (self._log_id_mismatch, self.log_id))
                    except RunExists:
                        logging.warning('Watchdog was not stopped?')
                    break
                await asyncio.sleep(self.KEEPALIVE_INTERVAL)


def open_resume_branch(
        main_branch: Branch, campaign_name: str, package: str,
        possible_forges: Optional[List[Forge]] = None) -> Optional[Branch]:
    try:
        forge = get_forge(main_branch, possible_forges=possible_forges)
    except UnsupportedForge as e:
        # We can't figure out what branch to resume from when there's
        # no forge that can tell us.
        logging.warning("Unsupported forge (%s)", e)
        return None
    except ForgeLoginRequired as e:
        logging.warning("No credentials for forge (%s)", e)
        return None
    except ssl.SSLCertVerificationError as e:
        logging.warning("SSL error probing for forge (%s)", e)
        return None
    except ConnectionError as e:
        logging.warning("Connection error opening resume branch (%s)", e)
        return None
    else:
        try:
            for option in [campaign_name, ('%s/main' % campaign_name), ('%s/main/%s' % (campaign_name, package))]:
                (
                    resume_branch,
                    unused_overwrite,
                    unused_existing_proposal,
                ) = find_existing_proposed(
                        main_branch, forge, option,
                        preferred_schemes=['https', 'git', 'bzr'])
                if resume_branch:
                    break
        except NoSuchProject as e:
            logging.warning("Project %s not found", e.project)
            return None
        except PermissionDenied as e:
            logging.warning("Unable to list existing proposals: %s", e)
            return None
        except UnusableRedirect as e:
            logging.warning("Unable to list existing proposals: %s", e)
            return None
        except UnexpectedHttpStatus as e:
            if e.code == 429:
                try:
                    retry_after = int(e.headers['Retry-After'])  # type: ignore
                except TypeError:
                    logging.warning(
                        'Unable to parse retry-after header: %s',
                        e.headers['Retry-After'])  # type: ignore
                    retry_after = None
                else:
                    retry_after = None
                raise BranchRateLimited(e.path, str(e), retry_after=retry_after)
            logging.warning(
                'Unexpected HTTP status for %s: %s %s', e.path,
                e.code, e.extra)
            # TODO(jelmer): Considering re-raising here for some errors?
            return None
        else:
            return resume_branch


async def check_resume_result(
        conn: asyncpg.Connection, suite: str,
        resume_branch: Branch) -> Optional["ResumeInfo"]:
    row = await conn.fetchrow(
        "SELECT id, result, review_status, "
        "array(SELECT row(role, remote_name, base_revision, revision) "
        "FROM new_result_branch WHERE run_id = run.id) AS result_branches "
        "FROM run "
        "WHERE suite = $1 AND revision = $2 AND result_code = 'success' "
        "ORDER BY finish_time DESC LIMIT 1",
        suite,
        resume_branch.last_revision().decode("utf-8"),
    )
    if row is not None:
        resume_run_id = row['id']
        resume_branch_result = row['result']
        resume_review_status = row['review_status']
        resume_result_branches = [
            (role, name,
             base_revision.encode("utf-8") if base_revision else None,
             revision.encode("utf-8") if revision else None)
            for (role, name, base_revision, revision) in row['result_branches']]
    else:
        logging.warning(
            'Unable to find resume branch %r in database',
            resume_branch)
        return None
    if resume_review_status == "rejected":
        logging.info("Unsetting resume branch, since last run was rejected.")
        return None
    return ResumeInfo(
        resume_run_id, resume_branch, resume_branch_result,
        resume_result_branches or [])


class ResumeInfo(object):
    def __init__(self, run_id, branch, result, resume_result_branches):
        self.run_id = run_id
        self.branch = branch
        self.result = result
        self.resume_result_branches = resume_result_branches

    @property
    def resume_branch_url(self):
        return full_branch_url(self.branch)

    def json(self):
        return {
            "run_id": self.run_id,
            "result": self.result,
            "branch_url": self.resume_branch_url,
            "branches": [
                (fn, n, br.decode("utf-8") if br is not None else None,
                 r.decode("utf-8") if r is not None else None)
                for (fn, n, br, r) in self.resume_result_branches
            ],
        }


def queue_item_env(queue_item):
    env = {}
    env["PACKAGE"] = queue_item.package
    return env


def cache_branch_name(distro_config, role):
    if role != 'main':
        raise ValueError(role)
    return "%s/latest" % (distro_config.vendor or dpkg_vendor().lower())


async def store_change_set(
        conn: asyncpg.Connection,
        name: str,
        campaign: str):
    await conn.execute(
        """INSERT INTO change_set (id, campaign) VALUES ($1, $2)
        ON CONFLICT DO NOTHING""",
        name, campaign)


async def store_run(
    conn: asyncpg.Connection,
    run_id: str,
    name: str,
    vcs_type: str,
    vcs_url: str,
    start_time: datetime,
    finish_time: datetime,
    command: str,
    description: Optional[str],
    instigated_context: Optional[str],
    context: Optional[str],
    main_branch_revision: Optional[bytes],
    result_code: str,
    revision: Optional[bytes],
    subworker_result: Optional[Any],
    suite: str,
    logfilenames: List[str],
    value: Optional[int],
    worker_name: str,
    result_branches: Optional[List[Tuple[str, str, bytes, bytes]]] = None,
    result_tags: Optional[List[Tuple[str, bytes]]] = None,
    resume_from: Optional[str] = None,
    failure_details: Optional[Any] = None,
    target_branch_url: Optional[str] = None,
    change_set: Optional[str] = None,
    followup_actions: Optional[Any] = None,
):
    """Store a run.

    Args:
      run_id: Run id
      name: Package name
      vcs_type: VCS type
      vcs_url: Upstream branch URL
      start_time: Start time
      finish_time: Finish time
      command: Command
      description: A human-readable description
      instigated_context: Context that instigated this run
      context: Subworker-specific context
      main_branch_revision: Main branch revision
      result_code: Result code (as constant string)
      revision: Resulting revision id
      subworker_result: Subworker-specific result data (as json)
      suite: Suite
      logfilenames: List of log filenames
      value: Value of the run (as int)
      worker_name: Name of the worker
      result_branches: Result branches
      result_tags: Result tags
      resume_from: Run this one was resumed from
      failure_details: Result failure details
      target_branch_url: Branch URL to target
      change_set: Change set id
    """
    if result_tags is None:
        result_tags_updated = None
    else:
        result_tags_updated = [(n, r.decode("utf-8")) for (n, r) in result_tags]

    await conn.execute(
        "INSERT INTO run (id, command, description, result_code, "
        "start_time, finish_time, package, instigated_context, context, "
        "main_branch_revision, "
        "revision, result, suite, vcs_type, branch_url, logfilenames, "
        "value, worker, result_tags, "
        "resume_from, failure_details, target_branch_url, change_set, "
        "followup_actions) "
        "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, "
        "$12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)",
        run_id,
        command,
        description,
        result_code,
        start_time,
        finish_time,
        name,
        instigated_context,
        context,
        main_branch_revision.decode("utf-8") if main_branch_revision else None,
        revision.decode("utf-8") if revision else None,
        subworker_result if subworker_result else None,
        suite,
        vcs_type,
        vcs_url,
        logfilenames,
        value,
        worker_name,
        result_tags_updated,
        resume_from,
        failure_details,
        target_branch_url,
        change_set,
        followup_actions,
    )

    if result_branches:
        await conn.executemany(
            "INSERT INTO new_result_branch "
            "(run_id, role, remote_name, base_revision, revision) "
            "VALUES ($1, $2, $3, $4, $5)",
            [
                (run_id, role, remote_name, br.decode("utf-8") if br else None, r.decode("utf-8") if r else None)
                for (role, remote_name, br, r) in result_branches
            ],
        )


def has_relation(v, pkg):
    from debian.deb822 import PkgRelation
    for r in PkgRelation.parse_relations(v):
        for o in r:
            if o['name'] == pkg:
                return True
    return False


def has_build_relation(c, pkg):
    for f in ["Build-Depends", "Build-Depends-Indep", "Build-Depends-Arch",
              "Build-Conflicts", "Build-Conflicts-Indep",
              "Build-Conflicts-Arch"]:
        if has_relation(c.get(f, ""), pkg):
            return True
    return False


def has_runtime_relation(c, pkg):
    for f in ["Depends", "Recommends", "Suggests",
              "Breaks", "Replaces"]:
        if has_relation(c.get(f, ""), pkg):
            return True
    return False



async def followup_run(
        config: Config, database: asyncpg.pool.Pool, policy: PolicyConfig,
        item: QueueItem, result: JanitorResult) -> None:
    if result.code == "success" and item.suite not in ("unchanged", "debianize"):
        async with database.acquire() as conn:
            run = await conn.fetchrow(
                "SELECT 1 FROM last_runs WHERE package = $1 AND revision = $2 AND result_code = 'success'",
                result.package, result.main_branch_revision.decode('utf-8')
            )
            if run is None:
                logging.info("Scheduling control run for %s.", item.package)
                await do_schedule_control(
                    conn,
                    item.package,
                    main_branch_revision=result.main_branch_revision,
                    estimated_duration=result.duration,
                    requestor="control",
                )
            # see if there are any packages that failed because
            # they lacked this one
            if getattr(result.builder_result, 'build_distribution', None) is not None:
                dependent_suites = [
                    campaign.name for campaign in config.campaign
                    if campaign.debian_build and result.builder_result.build_distribution in campaign.debian_build.extra_build_distribution]
                runs_to_retry = await conn.fetch(
                    "SELECT package, suite FROM last_missing_apt_dependencies WHERE name = $1 AND suite = ANY($2::text[])",
                    item.package, dependent_suites)
                for run_to_retry in runs_to_retry:
                    await do_schedule(
                        conn, run_to_retry['package'],
                        change_set=result.change_set,
                        bucket='missing-deps', requestor='schedule-missing-deps (now newer %s is available)' % item.package,
                        suite=run_to_retry['suite'])

    if result.followup_actions and result.code != 'success':
        from .missing_deps import schedule_new_package, schedule_update_package
        requestor = 'schedule-missing-deps (needed by %s)' % item.package
        async with database.acquire() as conn:
            for scenario in result.followup_actions:
                for action in scenario:
                    if action['action'] == 'new-package':
                        await schedule_new_package(
                            conn, action['upstream-info'],
                            policy,
                            requestor=requestor, change_set=result.change_set)
                    elif action['action'] == 'update-package':
                        await schedule_update_package(
                            conn, policy, action['package'], action['desired-version'],
                            requestor=requestor, change_set=result.change_set)
        from .missing_deps import reconstruct_problem, problem_to_upstream_requirement
        problem = reconstruct_problem(result.code, result.failure_details)
        if problem is not None:
            requirement = problem_to_upstream_requirement(problem)
        else:
            requirement = None
        if requirement:
            logging.info('TODO: attempt to find a resolution for %r', requirement)

    # If there was a successful run, trigger builds for any
    # reverse dependencies in the same changeset.
    if item.suite in ('fresh-releases', 'fresh-snapshots') and result.code == 'success':
        from breezy.plugins.debian.apt_repo import RemoteApt
        # Find all binaries that have changed in this run
        debian_result = result.builder_result
        if result.builder_result is None:
            logging.warning(
                'Missing debian result for run %s (%s/%s)',
                result.log_id, result.package, result.suite)
            binary_packages = []
            new_build_version = None   # noqa: F841
            old_build_version = None   # noqa: F841
        else:
            binary_packages = debian_result.binary_packages
            new_build_version = debian_result.build_version   # noqa: F841
            # TODO(jelmer): Get old_build_version from base_distribution

        campaign_config = get_campaign_config(config, item.suite)
        base_distribution = get_distribution(config, campaign_config.debian_build.base_distribution)
        apt = RemoteApt(base_distribution.archive_mirror_uri)

        # TODO(jelmer): in the future, we may want to do more than trigger
        # control builds here, e.g. trigger fresh-releases
        # (or maybe just if the control build fails?)

        need_control = set()

        try:
            for source in apt.iter_sources(base_distribution.name):
                if any([has_build_relation(source, p) for p in binary_packages]):
                    need_control.add(source)
                    break

            for binary in apt.iter_binaries(base_distribution.name):
                if any([has_runtime_relation(binary, p) for p in binary_packages]):
                    need_control.add(binary['Source'].split(' ')[0])
                    break
        except NotImplementedError:
            pass

        # TODO(jelmer): check test dependencies?

        for source in need_control:
            logging.info("Scheduling control run for %s.", source)
            await do_schedule_control(
                conn,
                source,
                change_set=result.change_set,
                requestor="control",
            )


class RunExists(Exception):
    """Run already exists."""

    def __init__(self, run_id):
        self.run_id = run_id


async def change_set_ready(conn, change_set_id):
    missing = await conn.fetch(
        "SELECT * FROM candidate WHERE change_set = $1 AND NOT EXISTS ("
        "  SELECT FROM last_runs WHERE change_set = candidate.change_set AND "
        "    package = candidate.package AND suite = candidate.suite AND "
        "    result_code in ('success', 'nothing-to-do', 'nothing-new-to-do'))",
        change_set_id)
    if missing:
        logging.info('More work to do for change set %s', change_set_id)
        for row in missing:
            logging.debug('  %s/%s', row['package'], row['suite'])
        return False
    logging.info('Change set %s ready', change_set_id)
    return True


class QueueProcessor(object):

    rate_limit_hosts: Dict[str, datetime]
    avoid_hosts: Set[str]

    def __init__(
        self,
        database: asyncpg.pool.Pool,
        redis,
        policy: PolicyConfig,
        config: Config,
        run_timeout: int,
        dry_run: bool = False,
        logfile_manager: Optional[LogFileManager] = None,
        artifact_manager: Optional[ArtifactManager] = None,
        vcs_managers: Optional[Dict[str, VcsManager]] = None,
        public_vcs_managers: Optional[Dict[str, VcsManager]] = None,
        use_cached_only: bool = False,
        committer: Optional[str] = None,
        backup_artifact_manager: Optional[ArtifactManager] = None,
        backup_logfile_manager: Optional[LogFileManager] = None,
        avoid_hosts: Optional[Set[str]] = None
    ):
        """Create a queue processor.
        """
        self.database = database
        self.redis = redis
        self.policy = policy
        self.config = config
        self.dry_run = dry_run
        self.logfile_manager = logfile_manager
        self.artifact_manager = artifact_manager
        self.vcs_managers = vcs_managers
        self.public_vcs_managers = public_vcs_managers
        self.use_cached_only = use_cached_only
        self.topic_queue = Topic("queue", repeat_last=True)
        self.topic_result = Topic("result")
        self.committer = committer
        self.active_runs: Dict[str, ActiveRun] = {}
        self.backup_artifact_manager = backup_artifact_manager
        self.backup_logfile_manager = backup_logfile_manager
        self.rate_limit_hosts = {}
        self.run_timeout = run_timeout
        self.avoid_hosts = avoid_hosts or set()

    def status_json(self) -> Any:
        return {
            "processing": [
                active_run.json() for active_run in self.active_runs.values()
            ],
            "avoid_hosts": list(self.avoid_hosts),
            "rate_limit_hosts": {
                host: ts.isoformat()
                for (host, ts) in self.rate_limit_hosts.items()}
        }

    def register_run(self, active_run: ActiveRun) -> None:
        self.active_runs[active_run.log_id] = active_run
        self.topic_queue.publish(self.status_json())
        active_run_count.labels(worker=active_run.worker_name).inc()
        packages_processed_count.inc()

    async def unclaim_run(self, log_id: str) -> None:
        active_run = self.active_runs.get(log_id)
        active_run_count.labels(worker=active_run.worker_name if active_run else None).dec()
        try:
            del self.active_runs[log_id]
        except KeyError:
            pass

    async def timeout_run(self, run: ActiveRun, duration: timedelta) -> None:
        return await self.abort_run(
            run, code='worker-timeout',
            description=("No keepalives received in %s." % duration))

    async def abort_run(self, run: ActiveRun, code: str, description: str) -> None:
        result = run.create_result(
            branch_url=run.main_branch_url,
            vcs_type=run.vcs_type,
            description=description,
            code=code,
            logfilenames=[],
        )
        await self.finish_run(run.queue_item, result)

    async def finish_run(self, item: QueueItem, result: JanitorResult) -> None:
        run_result_count.labels(
            package=item.package,
            suite=item.suite,
            result_code=result.code).inc()
        build_duration.labels(package=item.package, suite=item.suite).observe(
            result.duration.total_seconds()
        )
        async with self.database.acquire() as conn, conn.transaction():
            if not self.dry_run:
                if not result.change_set:
                    result.change_set = result.log_id
                    await store_change_set(conn, result.change_set, campaign=result.suite)
                try:
                    await store_run(
                        conn,
                        run_id=result.log_id,
                        name=item.package,
                        vcs_type=result.vcs_type,
                        vcs_url=result.branch_url,
                        start_time=result.start_time,
                        finish_time=result.finish_time,
                        command=item.command,
                        description=result.description,
                        instigated_context=item.context,
                        context=result.context,
                        main_branch_revision=result.main_branch_revision,
                        result_code=result.code,
                        revision=result.revision,
                        subworker_result=result.subworker_result,
                        suite=item.suite,
                        logfilenames=result.logfilenames,
                        value=result.value,
                        worker_name=result.worker_name,
                        result_branches=result.branches,
                        result_tags=result.tags,
                        failure_details=result.failure_details,
                        resume_from=result.resume_from,
                        target_branch_url=result.target_branch_url,
                        change_set=result.change_set,
                        followup_actions=result.followup_actions,
                    )
                except asyncpg.UniqueViolationError as e:
                    logging.info('Unique violation error creating run: %r', e)
                    raise RunExists(result.log_id)
                if result.builder_result:
                    await result.builder_result.store(conn, result.log_id)
                await conn.execute("DELETE FROM queue WHERE id = $1", item.id)
        await followup_run(self.config, self.database, self.policy, item, result)

        # If there is no more work to be done for this change set, mark it as ready.
        async with self.database.acquire() as conn, conn.transaction():
            if await change_set_ready(conn, result.change_set):
                if not self.dry_run:
                    await conn.execute(
                        "UPDATE change_set SET state = 'ready' "
                        "WHERE id = $1 AND state = 'working'",
                        result.change_set)

        self.topic_result.publish(result.json())
        await self.unclaim_run(result.log_id)
        self.topic_queue.publish(self.status_json())
        last_success_gauge.set_to_current_time()

    def rate_limited(self, host, retry_after):
        rate_limited_count.labels(host=host).inc()
        self.rate_limit_hosts[host] = (
            retry_after or (datetime.now() + timedelta(seconds=DEFAULT_RETRY_AFTER)))

    def can_process_url(self, url) -> bool:
        if url is None:
            return True
        host = urlutils.URL.from_string(url).host
        if host in self.avoid_hosts:
            return False
        until = self.rate_limit_hosts.get(host)
        if until and until > datetime.now():
            return False
        return True

    async def next_queue_item(self, conn, package=None, campaign=None):
        limit = len(self.active_runs) + 300
        queue = Queue(conn)
        async for item in queue.iter_queue(
                limit=limit, campaign=campaign, package=package):
            if self.is_queue_item_assigned(item.id):
                continue
            vcs_info = await conn.fetchone(
                'SELECT vcs_type, branch_url, subpath FROM package '
                'WHERE name = $1', item.package)
            if not self.can_process_url(vcs_info["branch_url"]):
                continue
            return item, vcs_info
        return None, None

    def is_queue_item_assigned(self, queue_item_id: int) -> bool:
        """Check if a queue item has been assigned already."""
        for active_run in self.active_runs.values():
            if active_run.queue_item.id == queue_item_id:
                return True
        return False


@routes.get("/status", name="status")
async def handle_status(request):
    queue_processor = request.app['queue_processor']
    return web.json_response(queue_processor.status_json())


async def _find_active_run(request):
    queue_processor = request.app['queue_processor']
    run_id = request.match_info["run_id"]
    queue_id = request.query.get('queue_id')  # noqa: F841
    worker_name = request.query.get('worker_name')  # noqa: F841
    try:
        return queue_processor.active_runs[run_id]
    except KeyError:
        raise web.HTTPNotFound(text="No such current run: %s" % run_id)


@routes.get("/log/{run_id}", name="log-index")
async def handle_log_index(request):
    active_run = await _find_active_run(request)
    log_filenames = await active_run.list_log_files()
    return web.json_response(log_filenames)


@routes.post("/kill/{run_id}", name="kill")
async def handle_kill(request):
    active_run = await _find_active_run(request)
    ret = active_run.json()
    await active_run.kill()
    return web.json_response(ret)


@routes.get("/log/{run_id}/{filename}", name="log")
async def handle_log(request):
    queue_processor = request.app['queue_processor']
    run_id = request.match_info["run_id"]
    filename = request.match_info["filename"]

    if "/" in filename:
        return web.Response(text="Invalid filename %s" % filename, status=400)
    try:
        active_run = queue_processor.active_runs[run_id]
    except KeyError:
        return web.Response(text="No such current run: %s" % run_id, status=404)
    try:
        f = await active_run.get_log_file(filename)
    except FileNotFoundError:
        return web.Response(text="No such log file: %s" % filename, status=404)

    try:
        response = web.StreamResponse(
            status=200, reason="OK", headers=[("Content-Type", "text/plain")]
        )
        await response.prepare(request)
        for chunk in f:
            await response.write(chunk)
        await response.write_eof()
    finally:
        f.close()
    return response


@routes.get("/active-runs", name="get-active-runs")
async def handle_get_active_runs(request):
    queue_processor = request.app['queue_processor']
    return web.json_response(queue_processor.status_json()["processing"])


@routes.post("/active-runs", name="assign")
async def handle_assign(request):
    json = await request.json()
    assignment_count.labels(worker=json.get("worker")).inc()
    return await next_item(
        request, 'assign', worker=json.get("worker"),
        worker_link=json.get("worker_link"),
        backchannel=json['backchannel'],
        package=json.get('package'),
        campaign=json.get('campaign')
        )


@routes.get("/active-runs/+peek", name="peek")
async def handle_peek(request):
    return await next_item(request, 'peek')


@routes.get("/queue", name="queue")
async def handle_queue(request):
    response_obj = []
    queue_processor = request.app['queue_processor']
    if 'limit' in request.query:
        limit = int(request.query['limit'])
    else:
        limit = None
    async with queue_processor.database.acquire() as conn:
        queue = Queue(conn)
        for entry in await conn.iter_queue(limit=limit):
            response_obj.append(
                {
                    "queue_id": entry.id,
                    "package": entry.package,
                    "campaign": entry.suite,
                    "context": entry.context,
                    "command": entry.command,
                }
            )
    return web.json_response(response_obj)


async def next_item(request, mode, worker=None, worker_link=None, backchannel=None, package=None, campaign=None):
    possible_transports = []
    possible_forges = []

    span = aiozipkin.request_span(request)

    async def abort(active_run, code, description):
        result = active_run.create_result(
            branch_url=active_run.main_branch_url,
            vcs_type=active_run.vcs_type,
            code=code,
            description=description
        )
        try:
            await queue_processor.finish_run(active_run.queue_item, result)
        except RunExists:
            pass

    queue_processor = request.app['queue_processor']

    async with queue_processor.database.acquire() as conn:
        item = None
        while item is None:
            with span.new_child('sql:queue-item'):
                item, vcs_info = await queue_processor.next_queue_item(
                    conn, package=package, campaign=campaign)
            if item is None:
                return web.json_response({'reason': 'queue empty'}, status=503)

            if backchannel and backchannel['kind'] == 'http':
                active_run = PollingActiveRun(
                    my_url=URL(backchannel['url']),
                    worker_name=worker,
                    queue_item=item,
                    vcs_info=vcs_info,
                    worker_link=worker_link
                )
            elif backchannel and backchannel['kind'] == 'jenkins':
                active_run = JenkinsRun(
                    my_url=URL(backchannel['url']),
                    worker_name=worker,
                    queue_item=item,
                    vcs_info=vcs_info,
                    worker_link=worker_link)
            else:
                active_run = ActiveRun(
                    worker_name=worker,
                    queue_item=item,
                    vcs_info=vcs_info,
                    worker_link=worker_link
                )

            queue_processor.register_run(active_run)

            if vcs_info["branch_url"] is None:
                await abort(active_run, 'not-in-vcs', "No VCS URL known for package.")
                item = None
                continue

            try:
                campaign_config = get_campaign_config(queue_processor.config, item.suite)
            except KeyError:
                logging.warning(
                    'Unable to find details for suite %r', item.suite)
                await abort(active_run, 'unknown-suite', "Suite %s unknown" % item.suite)
                item = None
                continue

        # This is simple for now, since we only support one distribution.
        builder = get_builder(queue_processor.config, campaign_config)

        with span.new_child('build-env'):
            build_env = await builder.build_env(conn, campaign_config, item)

        try:
            with span.new_child('branch:open'):
                probers = select_preferred_probers(vcs_info['vcs_type'])
                logging.info(
                    'Opening branch %s with %r', vcs_info['branch_url'],
                    [p.__name__ for p in probers])
                main_branch = await to_thread_timeout(
                    REMOTE_BRANCH_OPEN_TIMEOUT, open_branch_ext,
                    vcs_info['branch_url'],
                    possible_transports=possible_transports, probers=probers)
        except BranchRateLimited as e:
            host = urlutils.URL.from_string(vcs_info['branch_url']).host
            logging.warning('Rate limiting for %s: %r', host, e)
            queue_processor.rate_limited(host, e.retry_after)
            await abort(active_run, 'pull-rate-limited', str(e))
            return web.json_response(
                {'reason': str(e)}, status=429, headers={
                    'Retry-After': e.retry_after or DEFAULT_RETRY_AFTER})
        except BranchOpenFailure:
            resume_branch = None
            vcs_type = vcs_info['vcs_type']
        else:
            # We try the public branch first, since perhaps a maintainer
            # has made changes to the branch there.
            active_run.main_branch_url = full_branch_url(main_branch).rstrip('/')
            vcs_type = get_vcs_abbreviation(main_branch.repository)
            if not item.refresh:
                with span.new_child('resume-branch:open'):
                    try:
                        resume_branch = await to_thread_timeout(
                            REMOTE_BRANCH_OPEN_TIMEOUT,
                            open_resume_branch,
                            main_branch,
                            campaign_config.branch_name,
                            item.package,
                            possible_forges=possible_forges)
                    except BranchRateLimited as e:
                        host = urlutils.URL.from_string(e.url).host
                        logging.warning('Rate limiting for %s: %r', host, e)
                        queue_processor.rate_limited(host, e.retry_after)
                        await abort(active_run, 'resume-rate-limited', str(e))
                        return web.json_response(
                            {'reason': str(e)}, status=429, headers={
                                'Retry-After': e.retry_after or DEFAULT_RETRY_AFTER})
            else:
                resume_branch = None

        if vcs_type is not None:
            vcs_type = vcs_type.lower()

        if resume_branch is None and not item.refresh:
            with span.new_child('resume-branch:open'):
                try:
                    vcs_manager = queue_processor.public_vcs_managers[vcs_type]
                except KeyError:
                    logging.warning(
                        'Unsupported vcs %s for resume branch of %s',
                        vcs_type, item.package)
                    resume_branch = None
                else:
                    resume_branch = await to_thread_timeout(
                        VCS_STORE_BRANCH_OPEN_TIMEOUT,
                        vcs_manager.get_branch,
                        item.package, '%s/%s' % (campaign_config.name, 'main'))

        if resume_branch is not None:
            with span.new_child('resume-branch:check'):
                resume = await check_resume_result(conn, item.suite, resume_branch)
                if resume is not None:
                    if is_authenticated_url(resume.branch.user_url):
                        raise AssertionError('invalid resume branch %r' % (
                            resume.branch))
                    active_run.resume_from = resume.run_id
                    logging.info(
                        'Resuming %s/%s from run %s', item.package, item.suite,
                        resume.run_id)
        else:
            resume = None

    try:
        with span.new_child('cache-branch:check'):
            if campaign_config.HasField('debian_build'):
                distribution = get_distribution(
                    queue_processor.config,
                    campaign_config.debian_build.base_distribution)
                branch_name = cache_branch_name(distribution, "main")
            else:
                branch_name = "main"
            try:
                vcs_manager = queue_processor.public_vcs_managers[vcs_type]
            except KeyError:
                cached_branch_url = None
            else:
                cached_branch_url = queue_processor.public_vcs_managers[vcs_type].get_branch_url(
                    item.package, branch_name
                )
    except UnsupportedVcs:
        cached_branch_url = None

    env = {}
    env.update(queue_item_env(item))
    if queue_processor.committer:
        env.update(committer_env(queue_processor.committer))

    extra_env, command = splitout_env(item.command)
    env.update(extra_env)

    assignment = {
        "id": active_run.log_id,
        "description": "%s on %s" % (item.suite, item.package),
        "queue_id": item.id,
        "branch": {
            "url": active_run.main_branch_url,
            "subpath": vcs_info['subpath'],
            "vcs_type": vcs_info['vcs_type'],
            "cached_url": cached_branch_url,
        },
        "resume": resume.json() if resume else None,
        "build": {"target": builder.kind, "environment": build_env},
        "env": env,
        "command": command,
        "suite": item.suite,
        "force-build": campaign_config.force_build,
        "vcs_store": {
            k: v.base_url
            for (k, v) in queue_processor.public_vcs_managers.items()},
    }

    if mode == 'assign':
        with span.new_child('start-watchdog'):
            active_run.start_watchdog(queue_processor)
    else:
        await queue_processor.unclaim_run(active_run.log_id)
    return web.json_response(assignment, status=201)


@routes.get("/health", name="health")
async def handle_health(request):
    return web.Response(text="OK")


@routes.post("/active-runs/{run_id}/finish", name="finish")
async def handle_finish(request):
    queue_processor = request.app['queue_processor']
    run_id = request.match_info["run_id"]
    active_run = queue_processor.active_runs.get(run_id)
    if active_run:
        active_run.stop_watchdog()
        queue_item = active_run.queue_item
        worker_name = active_run.worker_name
        main_branch_url = active_run.main_branch_url
        vcs_type = active_run.vcs_type
        resume_from = active_run.resume_from
    else:
        queue_item = None
        worker_name = None
        main_branch_url = None
        vcs_type = None
        resume_from = None

    reader = await request.multipart()
    worker_result = None

    filenames = []
    with tempfile.TemporaryDirectory() as output_directory:
        while True:
            part = await reader.next()
            if part is None:
                break
            if part.filename == "result.json":
                worker_result = WorkerResult.from_json(await part.json())
            elif part.filename is None:
                return web.json_response(
                    {"reason": "Part without filename", "headers": dict(part.headers)},
                    status=400,
                )
            else:
                filenames.append(part.filename)
                output_path = os.path.join(output_directory, part.filename)
                with open(output_path, "wb") as f:
                    f.write(await part.read())

        if worker_result is None:
            return web.json_response({"reason": "Missing result JSON"}, status=400)

        logging.debug('worker result: %r', worker_result)

        if queue_item is None:
            async with queue_processor.database.acquire() as conn:
                queue = Queue(conn)
                queue_item = await queue.get_item(worker_result.queue_id)
            if queue_item is None:
                return web.json_response(
                    {"reason": "Unable to find relevant queue item %r" % worker_result.queue_id}, status=404)
        if worker_name is None:
            worker_name = worker_result.worker_name

        logfiles = gather_logs(output_directory)

        logfilenames = [entry.name for entry in logfiles]

        result = JanitorResult(
            pkg=queue_item.package,
            suite=queue_item.suite,
            log_id=run_id,
            code='success',
            worker_name=worker_name,
            branch_url=main_branch_url,
            vcs_type=vcs_type,
            worker_result=worker_result,
            logfilenames=logfilenames,
            resume_from=resume_from,
            change_set=queue_item.change_set,
            )

        await import_logs(
            logfiles,
            queue_processor.logfile_manager,
            queue_processor.backup_logfile_manager,
            queue_item.package,
            run_id,
            mtime=result.finish_time.timestamp(),
        )

        if result.builder_result is not None:
            result.builder_result.from_directory(output_directory)

            artifact_names = result.builder_result.artifact_filenames()
            try:
                await store_artifacts_with_backup(
                    queue_processor.artifact_manager,
                    queue_processor.backup_artifact_manager,
                    output_directory,
                    run_id,
                    artifact_names,
                )
            except BaseException as e:
                result.code = "artifact-upload-failed"
                result.description = str(e)
                artifact_upload_failed_count.inc()
                # TODO(jelmer): Mark ourselves as unhealthy?
                artifact_names = None
        else:
            artifact_names = None

    try:
        await queue_processor.finish_run(queue_item, result)
    except RunExists as e:
        return web.json_response(
            {"id": run_id, "filenames": filenames, "artifacts": artifact_names,
             "logs": logfilenames,
             "result": result.json(), 'reason': str(e)},
            status=409,
        )

    return web.json_response(
        {"id": run_id, "filenames": filenames,
         "logs": logfilenames,
         "artifacts": artifact_names, "result": result.json()},
        status=201,
    )


async def create_app(queue_processor, tracer=None):
    app = web.Application()
    app.router.add_routes(routes)
    app['rate-limited'] = {}
    app['queue_processor'] = queue_processor
    setup_metrics(app)
    app.router.add_get(
        "/ws/queue", functools.partial(pubsub_handler, queue_processor.topic_queue),
        name="ws-queue"
    )
    app.router.add_get(
        "/ws/result", functools.partial(pubsub_handler, queue_processor.topic_result),
        name="ws-result"
    )
    aiozipkin.setup(app, tracer, skip_routes=[
        app.router['metrics'],
        app.router['ws-queue'],
        app.router['ws-result'],
        ]
    )
    return app


async def main(argv=None):
    import argparse

    parser = argparse.ArgumentParser(prog="janitor.runner")
    parser.add_argument(
        "--listen-address", type=str, help="Listen address", default="localhost"
    )
    parser.add_argument("--port", type=int, help="Listen port", default=9911)
    parser.add_argument(
        "--pre-check",
        help="Command to run to check whether to process package.",
        type=str,
    )
    parser.add_argument(
        "--post-check", help="Command to run to check package before pushing.", type=str
    )
    parser.add_argument(
        "--dry-run",
        help="Create branches but don't push or propose anything.",
        action="store_true",
        default=False,
    )
    parser.add_argument(
        "--use-cached-only", action="store_true", help="Use cached branches only."
    )
    parser.add_argument(
        "--config", type=str, default="janitor.conf", help="Path to configuration."
    )
    parser.add_argument(
        "--backup-directory",
        type=str,
        default=None,
        help=(
            "Backup directory to write files to if artifact or log "
            "manager is unreachable"
        ),
    )
    parser.add_argument(
        "--public-vcs-location", type=str, default="https://janitor.debian.net/",
        help="Public vcs location (used for URLs handed to worker)"
    )
    parser.add_argument(
        "--policy", type=str, default="policy.conf", help="Path to policy."
    )
    parser.add_argument("--gcp-logging", action='store_true', help='Use Google cloud logging.')
    parser.add_argument("--debug", action="store_true", help="Print debugging info")
    parser.add_argument(
        "--run-timeout", type=int, help="Time before marking a run as having timed out (minutes)",
        default=60 * 10)
    parser.add_argument(
        "--avoid-host", type=str,
        help="Avoid processing runs on a host (e.g. 'salsa.debian.org')",
        default=[], action='append')
    args = parser.parse_args()

    if args.gcp_logging:
        import google.cloud.logging
        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    else:
        if args.debug:
            logging.basicConfig(level=logging.DEBUG)
        else:
            logging.basicConfig(level=logging.INFO)

    debug.set_debug_flags_from_config()

    with open(args.config, "r") as f:
        config = read_config(f)

    state.DEFAULT_URL = config.database_location
    public_vcs_managers = get_vcs_managers(args.public_vcs_location)
    vcs_managers = get_vcs_managers_from_config(config)

    endpoint = aiozipkin.create_endpoint("janitor.runner", ipv4=args.listen_address, port=args.port)
    if config.zipkin_address:
        tracer = await aiozipkin.create(config.zipkin_address, endpoint, sample_rate=1.0)
    else:
        tracer = await aiozipkin.create_custom(endpoint)
    trace_configs = [aiozipkin.make_trace_config(tracer)]

    logfile_manager = get_log_manager(config.logs_location, trace_configs=trace_configs)
    artifact_manager = get_artifact_manager(config.artifact_location, trace_configs=trace_configs)

    loop = asyncio.get_event_loop()
    if args.debug:
        loop.set_debug(True)
        loop.slow_callback_duration = 0.001
        warnings.simplefilter('always', ResourceWarning)

    async with AsyncExitStack() as stack:
        await stack.enter_async_context(artifact_manager)
        if args.backup_directory:
            backup_logfile_directory = os.path.join(args.backup_directory, "logs")
            backup_artifact_directory = os.path.join(args.backup_directory, "artifacts")
            if not os.path.isdir(backup_logfile_directory):
                os.mkdir(backup_logfile_directory)
            if not os.path.isdir(backup_artifact_directory):
                os.mkdir(backup_artifact_directory)
            backup_artifact_manager = LocalArtifactManager(backup_artifact_directory)
            await stack.enter_async_context(backup_artifact_manager)
            backup_logfile_manager = FileSystemLogFileManager(backup_logfile_directory)
            loop.create_task(
                upload_backup_artifacts(
                    backup_artifact_manager, artifact_manager, timeout=60 * 15
                )
            )
        else:
            backup_artifact_manager = None
            backup_logfile_manager = None
        db = await state.create_pool(config.database_location)
        redis = await aioredis.create_redis(config.redis_location)
        stack.callback(redis.close)
        with open(args.policy, 'r') as f:
            policy = read_policy(f)
        queue_processor = QueueProcessor(
            db,
            redis,
            policy,
            config,
            run_timeout=args.run_timeout,
            dry_run=args.dry_run,
            logfile_manager=logfile_manager,
            artifact_manager=artifact_manager,
            vcs_managers=vcs_managers,
            public_vcs_managers=public_vcs_managers,
            use_cached_only=args.use_cached_only,
            committer=config.committer,
            backup_artifact_manager=backup_artifact_manager,
            backup_logfile_manager=backup_logfile_manager,
            avoid_hosts=set(args.avoid_host),
        )

        app = await create_app(queue_processor, tracer=tracer)
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, args.listen_address, port=args.port)
        await site.start()
        while True:
            await asyncio.sleep(3600)


if __name__ == "__main__":
    sys.exit(asyncio.run(main(sys.argv)))
