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
from dataclasses import dataclass
from datetime import datetime, timedelta
from email.utils import parseaddr
import json
from io import BytesIO
import logging
import os
import ssl
import sys
import tempfile
from typing import List, Any, Optional, Dict, Tuple, Type, Set, Iterator
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
from breezy.errors import ConnectionError, UnexpectedHttpStatus, PermissionDenied
from breezy.transport import UnusableRedirect, UnsupportedProtocol

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

from ._launchpad import override_launchpad_consumer_name
override_launchpad_consumer_name()


DEFAULT_RETRY_AFTER = 120
REMOTE_BRANCH_OPEN_TIMEOUT = 10.0
VCS_STORE_BRANCH_OPEN_TIMEOUT = 5.0


routes = web.RouteTableDef()
run_count = Counter("run_count", "Number of runs executed.")
last_success_gauge = Gauge(
    "job_last_success_unixtime", "Last time a batch job successfully finished"
)
build_duration = Histogram("build_duration", "Build duration", ["campaign"])
run_result_count = Counter("result", "Result counts", ["campaign", "result_code"])
active_run_count = Gauge("active_runs", "Number of active runs", ["worker"])
assignment_count = Counter("assignments", "Number of assignments handed out", ["worker"])
rate_limited_count = Counter("rate_limited_host", "Rate limiting per host", ["host"])
artifact_upload_failed_count = Counter(
    "artifact_upload_failed", "Number of failed artifact uploads")
primary_logfile_upload_failed_count = Counter(
    "primary_logfile_upload_failed", "Number of failed logs to primary logfile target")
logfile_uploaded_count = Counter(
    "logfile_uploads", "Number of uploaded log files")


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

    async def config(
            self, conn: asyncpg.Connection,
            campaign_config: Campaign, queue_item: QueueItem) -> Dict[str, str]:
        raise NotImplementedError(self.config)

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

    async def config(self, conn, campaign_config, queue_item):
        config = {}
        if campaign_config.generic_build.chroot:
            config["chroot"] = campaign_config.generic_build.chroot
        return config

    async def build_env(self, conn, campaign_config, queue_item):
        return {}


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

    async def config(self, conn, campaign_config, queue_item):
        config = {}
        config['lintian'] = {'profile': self.distro_config.lintian_profile}
        if self.distro_config.lintian_suppress_tag:
            config['lintian']['suppress-tags'] = list(self.distro_config.lintian_suppress_tag)

        if self.apt_location.startswith("gs://"):
            bucket_name = URL(self.apt_location).host
            apt_location = "https://storage.googleapis.com/%s/" % bucket_name
        else:
            apt_location = self.apt_location

        extra_janitor_distributions = list(campaign_config.debian_build.extra_build_distribution)
        if queue_item.change_set:
            extra_janitor_distributions.append('cs/%s' % queue_item.change_set)

        config['build-extra-repositories'] = [
            "deb %s %s main" % (apt_location, suite)
            for suite in extra_janitor_distributions
        ]
        # TODO(jelmer): Ship build-extra-repositories-keys

        config["build-distribution"] = campaign_config.debian_build.build_distribution or campaign_config.name

        config["build-suffix"] = campaign_config.debian_build.build_suffix or ""

        if campaign_config.debian_build.build_command:
            config["build-command"] = campaign_config.debian_build.build_command
        elif self.distro_config.build_command:
            config["build-command"] = self.distro_config.build_command

        last_build_version = await conn.fetchval(
            "SELECT version FROM debian_build WHERE "
            "version IS NOT NULL AND source = $1 AND "
            "distribution = $2 ORDER BY version DESC LIMIT 1",
            queue_item.package, config['build-distribution']
        )

        if last_build_version:
            config["last-build-version"] = str(last_build_version)

        if campaign_config.debian_build.chroot:
            config["chroot"] = campaign_config.debian_build.chroot
        elif self.distro_config.chroot:
            config["chroot"] = self.distro_config.chroot

        config["base-apt-repository"] = "%s %s %s" % (
            self.distro_config.archive_mirror_uri,
            self.distro_config.name,
            " ".join(self.distro_config.component),
        )
        config["base-apt-repository-signed-by"] = self.distro_config.signed_by

        return config

    async def build_env(self, conn, campaign_config, queue_item):
        env = {}

        if self.distro_config.name:
            env["DISTRIBUTION"] = self.distro_config.name

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
        campaign=None,
        start_time=None,
        finish_time=None,
        worker_name=None,
        vcs_type=None,
        resume_from=None,
        change_set=None,
    ):
        self.package = pkg
        self.campaign = campaign
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
            self.codemod_result = worker_result.codemod
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
            self.branch_url = worker_result.branch_url
            self.vcs_type = worker_result.vcs_type
        else:
            self.start_time = start_time
            self.finish_time = finish_time
            self.context = None
            self.main_branch_revision = None
            self.revision = None
            self.codemod_result = None
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

    @property
    def codebase(self):
        return self.package

    def json(self):
        return {
            "package": self.package,
            "codebase": self.codebase,
            "campaign": self.campaign,
            "log_id": self.log_id,
            "description": self.description,
            "code": self.code,
            "failure_details": self.failure_details,
            "target": ({
                "name": self.builder_result.kind,
                "details": self.builder_result.json(),
            } if self.builder_result else {}),
            "logfilenames": self.logfilenames,
            "codemod": self.codemod_result,
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


@dataclass
class WorkerResult(object):
    """The result from a worker."""

    code: str
    description: Optional[str]
    context: Any
    codemod: Optional[Any] = None
    main_branch_revision: Optional[bytes] = None
    revision: Optional[bytes] = None
    value: Optional[int] = None
    branches: Optional[List[
        Tuple[Optional[str], Optional[str],
              Optional[bytes], Optional[bytes]]]] = None
    tags: Optional[List[Tuple[str, Optional[bytes]]]] = None
    remotes: Optional[Dict[str, Dict[str, Any]]] = None
    details: Any = None
    builder_result: Any = None
    start_time: Optional[datetime] = None
    finish_time: Optional[datetime] = None
    queue_id: Optional[int] = None
    worker_name: Optional[str] = None
    followup_actions: Optional[List[Any]] = None
    refreshed: bool = False
    target_branch_url: Optional[str] = None
    branch_url: Optional[str] = None
    vcs_type: Optional[str] = None

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
                (fn, n, br.encode("utf-8") if br else None,
                 r.encode("utf-8") if r else None)
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
            codemod=worker_result.get("codemod"),
            main_branch_revision=main_branch_revision,
            revision=revision,
            value=int(worker_result["value"]) if worker_result.get("value") else None,
            branches=branches,
            tags=tags,
            remotes=worker_result.get("remotes"),
            details=worker_result.get("details"),
            builder_result=builder_result,
            start_time=datetime.fromisoformat(worker_result['start_time'])
            if 'start_time' in worker_result else None,
            finish_time=datetime.fromisoformat(worker_result['finish_time'])
            if 'finish_time' in worker_result else None,
            queue_id=(
                int(worker_result["queue_id"])
                if "queue_id" in worker_result else None),
            worker_name=worker_result.get("worker_name"),
            followup_actions=worker_result.get("followup_actions"),
            refreshed=worker_result.get("refreshed", False),
            target_branch_url=worker_result.get("target_branch_url", None),
            branch_url=worker_result.get("branch_url"),
            vcs_type=worker_result.get("vcs_type"),
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


def gather_logs(output_directory: str) -> Iterator[os.DirEntry]:
    """Scan a directory for log files.

    Args:
      output_directory: Directory to scan
    Returns:
      Iterator over DirEntry objects matching logs
    """
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
            primary_logfile_upload_failed_count.inc()
            if backup_logfile_manager:
                await backup_logfile_manager.import_log(pkg, log_id, entry.path, mtime=mtime)
        except asyncio.TimeoutError as e:
            logging.warning("Timeout uploading logfile %s: %s", entry.name, e)
            primary_logfile_upload_failed_count.inc()
            if backup_logfile_manager:
                await backup_logfile_manager.import_log(pkg, log_id, entry.path, mtime=mtime)
        except PermissionDenied as e:
            logging.warning(
                "Permission denied error while uploading logfile %s: %s",
                entry.name, e)
            primary_logfile_upload_failed_count.inc()
            if backup_logfile_manager:
                await backup_logfile_manager.import_log(pkg, log_id, entry.path, mtime=mtime)
        else:
            logfile_uploaded_count.inc()


class ActiveRunDisappeared(Exception):

    def __init__(self, reason):
        self.reason = reason


class Backchannel(object):

    async def kill(self) -> None:
        raise NotImplementedError(self.kill)

    async def list_log_files(self):
        raise NotImplementedError(self.list_log_files)

    async def get_log_file(self, name):
        raise NotImplementedError(self.get_log_file)

    async def ping(self, log_id):
        raise NotImplementedError(self.ping)

    def json(self):
        return {}


class ActiveRun(object):

    worker_name: str
    worker_link: Optional[str]
    queue_item: QueueItem
    queue_id: int
    log_id: str
    start_time: datetime
    finish_time: Optional[datetime]
    estimated_duration: Optional[timedelta]
    campaign: str
    package: str
    change_set: Optional[str]
    command: str
    backchannel: Backchannel

    def __init__(
        self,
        campaign: str,
        package: str,
        change_set: Optional[str],
        command: str,
        instigated_context: Any,
        estimated_duration: Optional[timedelta],
        queue_id: int,
        log_id: str,
        start_time: datetime,
        vcs_info: Dict[str, str],
        backchannel: Optional[Backchannel],
        worker_name: str,
        worker_link: Optional[str] = None,
    ):
        self.campaign = campaign
        self.package = package
        self.change_set = change_set
        self.command = command
        self.instigated_context = instigated_context
        self.estimated_duration = estimated_duration
        self.queue_id = queue_id
        self.start_time = start_time
        self.log_id = log_id
        self.worker_name = worker_name
        self.vcs_info = vcs_info
        self.backchannel = backchannel or Backchannel()
        self.resume_branch_name = None
        self.worker_link = worker_link
        self.resume_from = None
        self._watch_dog = None

    @classmethod
    def from_queue_item(
        cls,
        queue_item: QueueItem,
        vcs_info: Dict[str, str],
        backchannel: Optional[Backchannel],
        worker_name: str,
        worker_link: Optional[str] = None,
    ):
        return cls(
            campaign=queue_item.campaign,
            package=queue_item.package,
            change_set=queue_item.change_set,
            command=queue_item.command,
            instigated_context=queue_item.context,
            estimated_duration=queue_item.estimated_duration,
            queue_id=queue_item.id,
            start_time=datetime.utcnow(),
            log_id=str(uuid.uuid4()),
            backchannel=backchannel,
            vcs_info=vcs_info,
            worker_name=worker_name,
            worker_link=worker_link)

    @classmethod
    def from_json(cls, js):
        if 'jenkins' in js['backchannel']:
            backchannel = JenkinsBackchannel.from_json(js['backchannel'])
        elif 'my_url' in js['backchannel']:
            backchannel = PollingBackchannel.from_json(js['backchannel'])
        else:
            backchannel = Backchannel()
        return cls(
            campaign=js['campaign'],
            start_time=datetime.fromisoformat(js['start_time']),
            package=js['package'],
            change_set=js['change_set'],
            command=js['command'],
            instigated_context=js['instigated_context'],
            estimated_duration=(
                timedelta(seconds=js['estimated_duration'])
                if js.get('estimated_duration') else None),
            queue_id=js['queue_id'],
            log_id=js['id'],
            backchannel=backchannel,
            vcs_info=js['vcs'],
            worker_name=js['worker'],
            worker_link=js['worker_link'],
        )

    @property
    def current_duration(self):
        return datetime.utcnow() - self.start_time

    def create_result(self, **kwargs):
        return JanitorResult(
            pkg=self.package,
            campaign=self.campaign,
            start_time=self.start_time,
            finish_time=datetime.utcnow(),
            log_id=self.log_id,
            worker_name=self.worker_name,
            resume_from=self.resume_from,
            change_set=self.change_set,
            **kwargs)

    async def ping(self):
        return await self.backchannel.ping(self.log_id)

    @property
    def vcs_type(self):
        return self.vcs_info["vcs_type"]

    @property
    def main_branch_url(self):
        return self.vcs_info["branch_url"]

    def json(self) -> Any:
        """Return a JSON representation."""
        return {
            "queue_id": self.queue_id,
            "id": self.log_id,
            "package": self.package,
            "codebase": self.package,
            "change_set": self.change_set,
            "campaign": self.campaign,
            "command": self.command,
            "estimated_duration": self.estimated_duration.total_seconds()
            if self.estimated_duration
            else None,
            "current_duration": self.current_duration.total_seconds(),
            "start_time": self.start_time.isoformat(),
            "worker": self.worker_name,
            "worker_link": self.worker_link,
            "vcs": self.vcs_info,
            "backchannel": self.backchannel.json(),
            "instigated_context": self.instigated_context,
        }


class JenkinsBackchannel(Backchannel):

    KEEPALIVE_TIMEOUT = 60

    def __init__(self, my_url: URL, metadata=None):
        self.my_url = my_url
        self._metadata = metadata

    @classmethod
    def from_json(cls, js):
        return cls(
            my_url=URL(js['my_url']),
            metadata=js['jenkins']
        )

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

    async def _get_job(self, session):
        async with session.get(
                self.my_url / 'api/json', raise_for_status=True,
                timeout=ClientTimeout(self.KEEPALIVE_TIMEOUT)) as resp:
            return await resp.json()

    async def ping(self, expected_log_id):
        health_url = self.my_url / 'log-id'
        logging.info('Pinging URL %s', health_url)
        async with ClientSession() as session:
            try:
                await self._get_job(session)
            except (ClientConnectorError, ServerDisconnectedError,
                    asyncio.TimeoutError, ClientOSError) as e:
                logging.warning('Failed to ping client %s: %r', self.my_url, e)
                return False
            except ClientResponseError as e:
                if e.status == 404:
                    raise ActiveRunDisappeared('Jenkins job %s has disappeared' % self.my_url)
                else:
                    logging.warning('Failed to ping client %s: %r', self.my_url, e)
                    return False
            else:
                return True

    def json(self):
        return {
            'my_url': str(self.my_url),
            'jenkins': self._metadata,
        }


class PollingBackchannel(Backchannel):

    KEEPALIVE_TIMEOUT = 60

    def __init__(self, my_url: URL):
        self.my_url = my_url

    @classmethod
    def from_json(cls, js):
        return cls(
            my_url=URL(js['my_url']),
        )

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

    async def ping(self, expected_log_id):
        health_url = self.my_url / 'log-id'
        logging.info('Pinging URL %s', health_url)
        async with ClientSession() as session:
            try:
                async with session.get(
                        health_url, raise_for_status=True,
                        timeout=ClientTimeout(self.KEEPALIVE_TIMEOUT)) as resp:
                    log_id = (await resp.read()).decode()
            except (ClientConnectorError, ClientResponseError,
                    asyncio.TimeoutError, ClientOSError,
                    ServerDisconnectedError) as err:
                logging.warning(
                    'Failed to ping client %s: %r', self.my_url, err)
                return False

            if log_id != expected_log_id:
                raise ActiveRunDisappeared(
                    'Worker started processing new run %s rather than %s' %
                    (log_id, expected_log_id))

        return True

    def json(self):
        return {
            'my_url': str(self.my_url),
        }


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
                    unused_existing_proposals,
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
        conn: asyncpg.Connection, campaign: str,
        resume_branch: Branch) -> Optional["ResumeInfo"]:
    row = await conn.fetchrow(
        "SELECT id, result, review_status, "
        "array(SELECT row(role, remote_name, base_revision, revision) "
        "FROM new_result_branch WHERE run_id = run.id) AS result_branches "
        "FROM run "
        "WHERE suite = $1 AND revision = $2 AND result_code = 'success' "
        "ORDER BY finish_time DESC LIMIT 1",
        campaign,
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
    vcs_type: Optional[str],
    branch_url: Optional[str],
    start_time: datetime,
    finish_time: datetime,
    command: str,
    description: Optional[str],
    instigated_context: Optional[str],
    context: Optional[str],
    main_branch_revision: Optional[bytes],
    result_code: str,
    revision: Optional[bytes],
    codemod_result: Optional[Any],
    campaign: str,
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
      codemod_result: Subworker-specific result data (as json)
      campaign: The campaign
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
        codemod_result if codemod_result else None,
        campaign,
        vcs_type,
        branch_url,
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


def find_reverse_source_deps(apt, binary_packages):
    # TODO(jelmer): in the future, we may want to do more than trigger
    # control builds here, e.g. trigger fresh-releases
    # (or maybe just if the control build fails?)

    need_control = set()
    with apt:
        for source in apt.iter_sources():
            if any([has_build_relation(source, p) for p in binary_packages]):
                need_control.add(source['Package'])
                break

        for binary in apt.iter_binaries():
            if any([has_runtime_relation(binary, p) for p in binary_packages]):
                need_control.add(binary['Source'])
                break

    return need_control


async def followup_run(
        config: Config, database: asyncpg.pool.Pool, policy: PolicyConfig,
        active_run: ActiveRun, result: JanitorResult) -> None:
    if result.code == "success" and active_run.campaign not in ("unchanged", "debianize"):
        async with database.acquire() as conn:
            run = await conn.fetchrow(
                "SELECT 1 FROM last_runs WHERE package = $1 AND revision = $2 AND result_code = 'success'",
                result.package, result.main_branch_revision.decode('utf-8')
            )
            if run is None:
                logging.info("Scheduling control run for %s.", active_run.package)
                await do_schedule_control(
                    conn,
                    active_run.package,
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
                    "SELECT package, suite AS campaign FROM last_missing_apt_dependencies WHERE name = $1 AND suite = ANY($2::text[])",
                    active_run.package, dependent_suites)
                for run_to_retry in runs_to_retry:
                    await do_schedule(
                        conn, run_to_retry['package'],
                        change_set=result.change_set,
                        bucket='missing-deps', requestor='schedule-missing-deps (now newer %s is available)' % active_run.package,
                        campaign=run_to_retry['campaign'])

    if result.followup_actions and result.code != 'success':
        from .missing_deps import schedule_new_package, schedule_update_package
        requestor = 'schedule-missing-deps (needed by %s)' % active_run.package
        async with database.acquire() as conn:
            for scenario in result.followup_actions:
                for action in scenario:
                    if action['action'] == 'new-package':
                        await schedule_new_package(
                            conn, action['upstream-info'],
                            config,
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
    if active_run.campaign in ('fresh-releases', 'fresh-snapshots') and result.code == 'success':
        from breezy.plugins.debian.apt_repo import RemoteApt
        # Find all binaries that have changed in this run
        debian_result = result.builder_result
        if result.builder_result is None:
            logging.warning(
                'Missing debian result for run %s (%s/%s)',
                result.log_id, result.package, result.campaign)
            binary_packages = []
            new_build_version = None   # noqa: F841
            old_build_version = None   # noqa: F841
        else:
            binary_packages = debian_result.binary_packages
            new_build_version = debian_result.build_version   # noqa: F841
            # TODO(jelmer): Get old_build_version from base_distribution

        campaign_config = get_campaign_config(config, active_run.campaign)
        base_distribution = get_distribution(config, campaign_config.debian_build.base_distribution)
        apt = RemoteApt(
            base_distribution.archive_mirror_uri, base_distribution.name,
            base_distribution.component)

        need_control = await to_thread(
            find_reverse_source_deps(apt, binary_packages))

        # TODO(jelmer): check test dependencies?

        for source in need_control:
            logging.info("Scheduling control run for %s.", source)
            await do_schedule_control(
                conn, source, change_set=result.change_set,
                requestor="control")


class RunExists(Exception):
    """Run already exists."""

    def __init__(self, run_id):
        self.run_id = run_id


class QueueProcessor(object):

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
        self.committer = committer
        self.backup_artifact_manager = backup_artifact_manager
        self.backup_logfile_manager = backup_logfile_manager
        self.run_timeout = run_timeout
        self.avoid_hosts = avoid_hosts or set()
        self._watch_dog = None

    def start_watchdog(self):
        if self._watch_dog is not None:
            raise Exception("Watchdog already started")
        self._watch_dog = create_background_task(
            self._watchdog(), 'watchdog for %r' % self)

    def stop_watchdog(self):
        if self._watch_dog is None:
            return
        try:
            self._watch_dog.cancel()
        except asyncio.CancelledError:
            pass
        self._watch_dog = None

    KEEPALIVE_INTERVAL = 10

    async def _watchdog(self):
        while True:
            for serialized in (await self.redis.hgetall('active-runs')).values():
                js = json.loads(serialized)
                active_run = ActiveRun.from_json(js)
                lk = await self.redis.hget('last-keepalive', js['id'])
                if lk:
                    last_keepalive = datetime.fromisoformat(lk.decode('utf-8'))
                else:
                    last_keepalive = active_run.start_time
                keepalive_age = datetime.utcnow() - last_keepalive
                if keepalive_age < timedelta(minutes=(self.run_timeout // 3)):
                    continue
                try:
                    if await active_run.ping():
                        await self.redis.hset(
                            'last-keepalive', active_run.log_id,
                            datetime.utcnow().isoformat())
                        keepalive_age = timedelta(seconds=0)
                except NotImplementedError:
                    if keepalive_age > timedelta(days=1):
                        try:
                            await self.abort_run(
                                active_run, 'run-disappeared', "no support for ping")
                        except RunExists:
                            logging.warning('Run not properly cleaned up?')
                        continue
                except ActiveRunDisappeared as e:
                    if keepalive_age > timedelta(minutes=self.run_timeout):
                        try:
                            await self.abort_run(active_run, 'run-disappeared', e.reason)
                        except RunExists:
                            logging.warning('Run not properly cleaned up?')
                        continue
                if keepalive_age > timedelta(minutes=self.run_timeout):
                    logging.warning(
                        "No keepalives received from %s for %s in %s, aborting.",
                        active_run.worker_name,
                        active_run.log_id,
                        keepalive_age,
                    )
                    try:
                        await self.abort_run(
                            active_run, code='worker-timeout',
                            description=("No keepalives received in %s." % keepalive_age))
                    except RunExists:
                        logging.warning('Run not properly cleaned up?')
                    continue
            await asyncio.sleep(self.KEEPALIVE_INTERVAL)

    async def status_json(self) -> Any:
        rate_limit_hosts = {
            h.decode('utf-8'): datetime.fromisoformat(t.decode('utf-8'))
            for (h, t) in (await self.redis.hgetall('rate-limit-hosts')).items()}
        last_keepalives = {
            r.decode('utf-8'): datetime.fromisoformat(v.decode('utf-8'))
            for (r, v) in (await self.redis.hgetall('last-keepalive')).items()}
        processing = []
        for e in (await self.redis.hgetall('active-runs')).values():
            js = json.loads(e)
            last_keepalive = last_keepalives.get(js['id'])
            if last_keepalive:
                js['last-keepalive'] = last_keepalive.isoformat(timespec='seconds')
                js['keepalive_age'] = (datetime.utcnow() - last_keepalive).total_seconds()
                js['mia'] = js['keepalive_age'] > self.run_timeout * 60
            else:
                js['keepalive_age'] = None
                js['last-keepalive'] = None
                js['mia'] = None
            processing.append(js)
        return {
            "processing": processing,
            "avoid_hosts": list(self.avoid_hosts),
            "rate_limit_hosts": {
                h: t for (h, t) in rate_limit_hosts.items()
                if t > datetime.utcnow()},
        }

    async def register_run(self, active_run: ActiveRun) -> None:
        tr = self.redis.multi_exec()
        tr.hset(
            'active-runs', active_run.log_id, json.dumps(active_run.json()))
        tr.hset(
            'assigned-queue-items', str(active_run.queue_id), active_run.log_id)
        tr.hset(
            'last-keepalive', active_run.log_id, datetime.utcnow().isoformat())
        await tr.execute()
        await self.redis.publish_json('queue', await self.status_json())
        active_run_count.labels(worker=active_run.worker_name).inc()
        run_count.inc()

    async def get_run(self, log_id: str) -> Optional[ActiveRun]:
        serialized = await self.redis.hget('active-runs', log_id)
        if not serialized:
            return None
        js = json.loads(serialized)
        return ActiveRun.from_json(js)

    async def unclaim_run(self, log_id: str) -> None:
        active_run = await self.get_run(log_id)
        active_run_count.labels(worker=active_run.worker_name if active_run else None).dec()
        if not active_run:
            return
        tr = self.redis.multi_exec()
        tr.hdel('assigned-queue-items', str(active_run.queue_id))
        tr.hdel('active-runs', log_id)
        tr.hdel('last-keepalive', log_id)
        await tr.execute()

    async def abort_run(self, run: ActiveRun, code: str, description: str) -> None:
        result = run.create_result(
            branch_url=run.main_branch_url,
            vcs_type=run.vcs_type,
            description=description,
            code=code,
            logfilenames=[],
        )
        await self.finish_run(run, result)

    async def finish_run(self, active_run: ActiveRun, result: JanitorResult) -> None:
        run_result_count.labels(
            campaign=active_run.campaign,
            result_code=result.code).inc()
        build_duration.labels(campaign=active_run.campaign).observe(
            result.duration.total_seconds()
        )
        async with self.database.acquire() as conn, conn.transaction():
            if not self.dry_run:
                if not result.change_set:
                    result.change_set = result.log_id
                    await store_change_set(
                        conn, result.change_set, campaign=result.campaign)
                try:
                    await store_run(
                        conn,
                        run_id=result.log_id,
                        name=active_run.package,
                        vcs_type=result.vcs_type,
                        branch_url=result.branch_url,
                        start_time=result.start_time,
                        finish_time=result.finish_time,
                        command=active_run.command,
                        description=result.description,
                        instigated_context=active_run.instigated_context,
                        context=result.context,
                        main_branch_revision=result.main_branch_revision,
                        result_code=result.code,
                        revision=result.revision,
                        codemod_result=result.codemod_result,
                        campaign=active_run.campaign,
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
                    await self.unclaim_run(result.log_id)
                    raise RunExists(result.log_id)
                if result.builder_result:
                    await result.builder_result.store(conn, result.log_id)
                await conn.execute("DELETE FROM queue WHERE id = $1", active_run.queue_id)
        await followup_run(self.config, self.database, self.policy, active_run, result)

        await self.redis.publish_json('result', result.json())
        await self.unclaim_run(result.log_id)
        await self.redis.publish_json('queue', await self.status_json())
        last_success_gauge.set_to_current_time()

    async def rate_limited(self, host, retry_after):
        rate_limited_count.labels(host=host).inc()
        if not retry_after:
            retry_after = datetime.now() + timedelta(seconds=DEFAULT_RETRY_AFTER)
        await self.redis.hset(
            'rate-limit-hosts', host, retry_after.isoformat())

    async def can_process_url(self, url) -> bool:
        if url is None:
            return True
        host = urlutils.URL.from_string(url).host
        if host in self.avoid_hosts:
            return False
        until = await self.redis.hget('rate-limit-hosts', host)
        if until and datetime.fromisoformat(until.decode('utf-8')) > datetime.now():
            return False
        return True

    async def next_queue_item(self, conn, package=None, campaign=None):
        limit = (await self.redis.hlen('active-runs')) + 300
        queue = Queue(conn)
        async for item in queue.iter_queue(
                limit=limit, campaign=campaign, package=package):
            if await self.is_queue_item_assigned(item.id):
                continue
            vcs_info = await conn.fetchrow(
                'SELECT vcs_type, branch_url, subpath FROM package '
                'WHERE name = $1', item.package)
            if vcs_info and not (await self.can_process_url(vcs_info["branch_url"])):
                continue
            return item, dict(vcs_info) if vcs_info else None
        return None, None

    async def is_queue_item_assigned(self, queue_item_id: int) -> bool:
        """Check if a queue item has been assigned already."""
        return await self.redis.hexists('assigned-queue-items', str(queue_item_id))


@routes.get("/status", name="status")
async def handle_status(request):
    queue_processor = request.app['queue_processor']
    return web.json_response(await queue_processor.status_json())


async def _find_active_run(request):
    queue_processor = request.app['queue_processor']
    run_id = request.match_info["run_id"]
    queue_id = request.query.get('queue_id')  # noqa: F841
    worker_name = request.query.get('worker_name')  # noqa: F841
    active_run = await queue_processor.get_run(run_id)
    if not active_run:
        raise web.HTTPNotFound(text="No such current run: %s" % run_id)


@routes.get("/log/{run_id}", name="log-index")
async def handle_log_index(request):
    active_run = await _find_active_run(request)
    log_filenames = await active_run.backchannel.list_log_files()
    return web.json_response(log_filenames)


@routes.post("/kill/{run_id}", name="kill")
async def handle_kill(request):
    active_run = await _find_active_run(request)
    ret = active_run.json()
    await active_run.backchannel.kill()
    return web.json_response(ret)


@routes.get("/log/{run_id}/{filename}", name="log")
async def handle_log(request):
    queue_processor = request.app['queue_processor']
    run_id = request.match_info["run_id"]
    filename = request.match_info["filename"]

    if "/" in filename:
        return web.Response(text="Invalid filename %s" % filename, status=400)
    active_run = await queue_processor.get_run(run_id)
    if not active_run:
        return web.Response(text="No such current run: %s" % run_id, status=404)
    try:
        f = await active_run.backchannel.get_log_file(filename)
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


@routes.post("/candidates", name="upload-candidates")
async def handle_candidates(request):
    unknown_packages = []
    unknown_campaigns = []
    queue_processor = request.app['queue_processor']
    async with queue_processor.database.acquire() as conn, conn.transaction():
        known_packages = set()
        for record in (await conn.fetch('SELECT name FROM package')):
            known_packages.add(record[0])

        known_campaign_names = [
            campaign.name for campaign in queue_processor.config.campaign]

        entries = []
        for candidate in (await request.json()):
            if candidate['package'] not in known_packages:
                logging.warning(
                    'ignoring candidate %s/%s; package unknown',
                    candidate['package'], candidate['campaign'])
                unknown_packages.append(candidate['package'])
                continue
            if candidate['campaign'] not in known_campaign_names:
                logging.warning('unknown suite %r', candidate['campaign'])
                unknown_campaigns.append(candidate['campaign'])
                continue

            entries.append((
                candidate['package'], candidate['campaign'],
                candidate['command'],
                candidate.get('change_set'), candidate.get('context'),
                candidate.get('value'), candidate.get('success_chance')))
        if 'replace' in request.query:
            await conn.execute('DELETE FROM candidate')

        await conn.executemany(
            "INSERT INTO candidate "
            "(package, suite, command, change_set, context, value, success_chance) "
            "VALUES ($1, $2, $3, $4, $5, $6, $7) "
            "ON CONFLICT (package, suite, coalesce(change_set, ''::text)) "
            "DO UPDATE SET context = EXCLUDED.context, value = EXCLUDED.value, "
            "success_chance = EXCLUDED.success_chance, command = EXCLUDED.command",
            entries,
        )
    return web.json_response({
        'unknown_campaigns': unknown_campaigns,
        'unknown_packages': unknown_packages})


@routes.get("/active-runs", name="get-active-runs")
async def handle_get_active_runs(request):
    queue_processor = request.app['queue_processor']
    return web.json_response((await queue_processor.status_json())["processing"])


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
        for entry in await queue.iter_queue(limit=limit):
            response_obj.append(
                {
                    "queue_id": entry.id,
                    "package": entry.package,
                    "codebase": entry.codebase,
                    "campaign": entry.campaign,
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
            await queue_processor.finish_run(active_run, result)
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
                bc = PollingBackchannel(my_url=URL(backchannel['url']))
            elif backchannel and backchannel['kind'] == 'jenkins':
                bc = JenkinsBackchannel(my_url=URL(backchannel['url']))
            else:
                bc = None

            active_run = ActiveRun.from_queue_item(
                backchannel=bc,
                worker_name=worker,
                queue_item=item,
                vcs_info=vcs_info,
                worker_link=worker_link
            )

            await queue_processor.register_run(active_run)

            if vcs_info["branch_url"] is None:
                await abort(active_run, 'not-in-vcs', "No VCS URL known for package.")
                item = None
                continue

            try:
                campaign_config = get_campaign_config(queue_processor.config, item.campaign)
            except KeyError:
                logging.warning(
                    'Unable to find details for campaign %r', item.campaign)
                await abort(active_run, 'unknown-campaign', "Campaign %s unknown" % item.campaign)
                item = None
                continue

        # This is simple for now, since we only support one distribution.
        builder = get_builder(queue_processor.config, campaign_config)

        with span.new_child('build-env'):
            build_env = await builder.build_env(conn, campaign_config, item)

        with span.new_child('config'):
            build_config = await builder.config(conn, campaign_config, item)

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
            await queue_processor.rate_limited(host, e.retry_after)
            await abort(active_run, 'pull-rate-limited', str(e))
            return web.json_response(
                {'reason': str(e)}, status=429, headers={
                    'Retry-After': e.retry_after or DEFAULT_RETRY_AFTER})
        except BranchOpenFailure as e:
            logging.debug(
                'Error opening branch %s: %s', vcs_info['branch_url'],
                e)
            resume_branch = None
            vcs_type = vcs_info['vcs_type']
        except asyncio.TimeoutError:
            logging.debug('Timeout opening branch %s', vcs_info['branch_url'])
            resume_branch = None
            vcs_type = vcs_info['vcs_type']
        else:
            # We try the public branch first, since perhaps a maintainer
            # has made changes to the branch there.
            active_run.vcs_info["branch_url"] = full_branch_url(main_branch).rstrip('/')
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
                        await queue_processor.rate_limited(host, e.retry_after)
                        await abort(active_run, 'resume-rate-limited', str(e))
                        return web.json_response(
                            {'reason': str(e)}, status=429, headers={
                                'Retry-After': e.retry_after or DEFAULT_RETRY_AFTER})
                    except asyncio.TimeoutError:
                        logging.debug('Timeout opening resume branch')
                        resume_branch = None
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
                    try:
                        resume_branch = await to_thread_timeout(
                            VCS_STORE_BRANCH_OPEN_TIMEOUT,
                            vcs_manager.get_branch,
                            item.package, '%s/%s' % (campaign_config.name, 'main'))
                    except asyncio.TimeoutError:
                        logging.warning('Timeout opening resume branch')

        if resume_branch is not None:
            with span.new_child('resume-branch:check'):
                resume = await check_resume_result(conn, item.campaign, resume_branch)
                if resume is not None:
                    if is_authenticated_url(resume.branch.user_url):
                        raise AssertionError('invalid resume branch %r' % (
                            resume.branch))
                    active_run.resume_from = resume.run_id
                    logging.info(
                        'Resuming %s/%s from run %s', item.package, item.campaign,
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
                target_repository_url = None
            else:
                cached_branch_url = vcs_manager.get_branch_url(
                    item.package, branch_name)
                target_repository_url = vcs_manager.get_repository_url(item.package)
    except UnsupportedVcs:
        cached_branch_url = None
        target_repository_url = None

    env = {}
    env.update(build_env)
    env.update(queue_item_env(item))
    if queue_processor.committer:
        env.update(committer_env(queue_processor.committer))

    extra_env, command = splitout_env(item.command)
    env.update(extra_env)

    assignment = {
        "id": active_run.log_id,
        "description": "%s on %s" % (item.campaign, item.package),
        "queue_id": item.id,
        "branch": {
            "url": active_run.main_branch_url,
            "subpath": vcs_info['subpath'],
            "vcs_type": vcs_info['vcs_type'],
            "cached_url": cached_branch_url,
        },
        "resume": resume.json() if resume else None,
        "build": {
            "target": builder.kind,
            "environment": build_env,
            "config": build_config,
        },
        "command": command,
        "codemod": {"command": command, "environment": {}},
        "env": env,
        "campaign": item.campaign,
        "force-build": campaign_config.force_build,
        "target_repository": {
            "url": target_repository_url,
            "vcs_type": vcs_info['vcs_type'],
        }
    }

    if mode == 'assign':
        pass
    else:
        await queue_processor.unclaim_run(active_run.log_id)
    return web.json_response(assignment, status=201)


@routes.get("/health", name="health")
async def handle_health(request):
    return web.Response(text="OK")


@routes.get("/ready", name="ready")
async def handle_ready(request):
    return web.Response(text="OK")


@routes.post("/active-runs/{run_id}/finish", name="finish")
async def handle_finish(request):
    queue_processor = request.app['queue_processor']
    run_id = request.match_info["run_id"]
    active_run = await queue_processor.get_run(run_id)
    if not active_run:
        raise web.HTTPNotFound(text=' no such run %s' % run_id)
    worker_name = active_run.worker_name
    main_branch_url = active_run.main_branch_url
    vcs_type = active_run.vcs_type
    resume_from = active_run.resume_from

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

        if worker_name is None:
            worker_name = worker_result.worker_name

        logfiles = list(gather_logs(output_directory))

        logfilenames = [entry.name for entry in logfiles]

        result = JanitorResult(
            pkg=active_run.package,
            campaign=active_run.campaign,
            log_id=run_id,
            code='success',
            worker_name=worker_name,
            branch_url=main_branch_url,
            vcs_type=vcs_type,
            worker_result=worker_result,
            logfilenames=logfilenames,
            resume_from=resume_from,
            change_set=active_run.change_set,
        )

        await import_logs(
            logfiles,
            queue_processor.logfile_manager,
            queue_processor.backup_logfile_manager,
            active_run.package,
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
        await queue_processor.finish_run(active_run, result)
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
    aiozipkin.setup(app, tracer, skip_routes=[
        app.router['metrics'],
    ])
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
        default=60)
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

    try:
        public_vcs_managers = get_vcs_managers(args.public_vcs_location)
    except UnsupportedProtocol as e:
        parser.error(
            'Unsupported protocol in --public-vcs-location: %s' % e.url)
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

        queue_processor.start_watchdog()

        app = await create_app(queue_processor, tracer=tracer)
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, args.listen_address, port=args.port)
        await site.start()
        while True:
            await asyncio.sleep(3600)


if __name__ == "__main__":
    sys.exit(asyncio.run(main(sys.argv)))
