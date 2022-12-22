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
from typing import (
    List,
    Any,
    Optional,
    Dict,
    Tuple,
    Type,
    Set,
    Iterator,
)
import uuid
import warnings

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
    MultipartReader,
    ServerDisconnectedError,
)
from redis.asyncio import Redis

from yarl import URL

from aiohttp_openmetrics import Counter, Gauge, Histogram, metrics_middleware, metrics

from breezy import debug, urlutils
from breezy.branch import Branch
from breezy.errors import ConnectionError, UnexpectedHttpStatus, PermissionDenied
from breezy.transport import UnusableRedirect, UnsupportedProtocol, Transport

from silver_platter.probers import (
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
    set_user_agent,
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
from .config import read_config, get_campaign_config, get_distribution, Campaign
from .debian import (
    dpkg_vendor,
)
from .logs import (
    get_log_manager,
    ServiceUnavailable,
    LogFileManager,
    FileSystemLogFileManager,
)
from .queue import QueueItem, Queue
from .schedule import do_schedule_control, do_schedule, CandidateUnavailable
from .vcs import (
    get_vcs_abbreviation,
    is_authenticated_url,
    open_branch_ext,
    BranchOpenFailure,
    get_vcs_managers,
    UnsupportedVcs,
    VcsManager,
)
from .worker_creds import check_worker_creds

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
queue_empty_count = Counter(
    "queue_empty",
    "Number of times the queue was empty when an assignment was requested")


async def to_thread_timeout(timeout, func, *args, **kwargs):
    cor = asyncio.to_thread(func, *args, **kwargs)
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

    @classmethod
    def from_json(cls, json):
        raise NotImplementedError(cls.from_json)


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

    def additional_colocated_branches(self, main_branch):
        raise NotImplementedError(self.additional_colocated_branches)


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

    def __init__(self, dep_server_url):
        self.dep_server_url = dep_server_url

    async def config(self, conn, campaign_config, queue_item):
        config = {}
        if campaign_config.generic_build.chroot:
            config["chroot"] = campaign_config.generic_build.chroot
        config["dep_server_url"] = self.dep_server_url
        return config

    async def build_env(self, conn, campaign_config, queue_item):
        return {}

    def additional_colocated_branches(self, main_branch):
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
        from .debian import (
            find_changes,
            NoChangesFile,
        )
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
        from .debian import changes_filenames
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

    def __init__(self, distro_config, apt_location: Optional[str] = None,
                 dep_server_url: Optional[str] = None):
        self.distro_config = distro_config
        self.apt_location = apt_location
        self.dep_server_url = dep_server_url

    async def config(self, conn, campaign_config, queue_item):
        config: Dict[str, Any] = {}
        config['lintian'] = {'profile': self.distro_config.lintian_profile}
        if self.distro_config.lintian_suppress_tag:
            config['lintian']['suppress-tags'] = list(self.distro_config.lintian_suppress_tag)

        extra_janitor_distributions = list(campaign_config.debian_build.extra_build_distribution)
        if queue_item.change_set:
            extra_janitor_distributions.append('cs/%s' % queue_item.change_set)

        # TODO(jelmer): Ship build-extra-repositories-keys, and specify [signed-by] here
        config['build-extra-repositories'] = []
        if self.apt_location:
            config['build-extra-repositories'].extend([
                "deb [trusted=yes] %s %s main" % (self.apt_location, suite)
                for suite in extra_janitor_distributions
            ])

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
        config["dep_server_url"] = self.dep_server_url

        return config

    async def build_env(self, conn, campaign_config, queue_item):
        env = {}

        if self.distro_config.name:
            env["DISTRIBUTION"] = self.distro_config.name

        env['DEB_VENDOR'] = self.distro_config.vendor or dpkg_vendor()

        if campaign_config.debian_build.chroot:
            env["CHROOT"] = campaign_config.debian_build.chroot
        elif self.distro_config.chroot:
            env["CHROOT"] = self.distro_config.chroot

        env["APT_REPOSITORY"] = "%s %s %s" % (
            self.distro_config.archive_mirror_uri,
            self.distro_config.name,
            " ".join(self.distro_config.component),
        )
        # TODO(jelmer): Set env["APT_REPOSITORY_KEY"]

        upstream_branch_url = await conn.fetchval(
            "SELECT upstream_branch_url FROM upstream WHERE name = $1",
            queue_item.package)
        if upstream_branch_url:
            env["UPSTREAM_BRANCH_URL"] = upstream_branch_url

        return env

    def additional_colocated_branches(self, main_branch):
        from silver_platter.debian import pick_additional_colocated_branches
        return pick_additional_colocated_branches(main_branch)


BUILDER_CLASSES: List[Type[Builder]] = [DebianBuilder, GenericBuilder]
RESULT_CLASSES = [builder_cls.result_cls for builder_cls in BUILDER_CLASSES]


def get_builder(config, campaign_config, apt_archive_url=None, dep_server_url=None):
    if campaign_config.HasField('debian_build'):
        try:
            distribution = get_distribution(
                config, campaign_config.debian_build.base_distribution)
        except KeyError as e:
            raise NotImplementedError(
                "Unsupported distribution: "
                f"{campaign_config.debian_build.base_distribution}") from e
        return DebianBuilder(
            distribution,
            apt_archive_url,
            dep_server_url,
        )
    elif campaign_config.HasField('generic_build'):
        return GenericBuilder(dep_server_url)
    else:
        raise NotImplementedError('no supported build type')


class JanitorResult(object):

    package: str
    log_id: str
    branch_url: str
    subpath: str
    code: str
    transient: Optional[bool]
    codebase: Optional[str]

    def __init__(
        self,
        *,
        pkg: str,
        codebase: str,
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
        subpath=None,
        resume_from=None,
        change_set=None,
        transient=None,
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
        self.subpath = subpath
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
            self.failure_stage = worker_result.stage
            self.start_time = worker_result.start_time
            self.finish_time = worker_result.finish_time
            if worker_result.refreshed:
                self.resume_from = None
            else:
                self.resume_from = resume_from
            self.target_branch_url = worker_result.target_branch_url
            self.branch_url = worker_result.branch_url
            self.vcs_type = worker_result.vcs_type
            self.subpath = worker_result.subpath
            self.transient = worker_result.transient
            self.codebase = worker_result.codebase
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
            self.failure_stage = None
            self.target_branch_url = None
            self.remotes = {}
            self.resume_from = None
            self.transient = transient
            self.codebase = codebase

    @property
    def duration(self):
        return self.finish_time - self.start_time

    def json(self):
        return {
            "package": self.package,
            "codebase": self.codebase,
            "campaign": self.campaign,
            "change_set": self.change_set,
            "log_id": self.log_id,
            "description": self.description,
            "code": self.code,
            "failure_details": self.failure_details,
            "failure_stage": self.failure_stage,
            "duration": self.duration.total_seconds(),
            "finish_time": self.finish_time.isoformat(),
            "start_time": self.start_time.isoformat(),
            "transient": self.transient,
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


def committer_env(committer: str) -> Dict[str, str]:
    env: Dict[str, str] = {}
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
    stage: Optional[str] = None
    builder_result: Any = None
    start_time: Optional[datetime] = None
    finish_time: Optional[datetime] = None
    queue_id: Optional[int] = None
    worker_name: Optional[str] = None
    refreshed: bool = False
    target_branch_url: Optional[str] = None
    branch_url: Optional[str] = None
    vcs_type: Optional[str] = None
    subpath: Optional[str] = None
    transient: Optional[bool] = None
    codebase: Optional[str] = None

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
            stage=worker_result.get("stage"),
            builder_result=builder_result,
            start_time=datetime.fromisoformat(worker_result['start_time'])
            if 'start_time' in worker_result else None,
            finish_time=datetime.fromisoformat(worker_result['finish_time'])
            if 'finish_time' in worker_result else None,
            queue_id=(
                int(worker_result["queue_id"])
                if "queue_id" in worker_result else None),
            worker_name=worker_result.get("worker_name"),
            refreshed=worker_result.get("refreshed", False),
            target_branch_url=worker_result.get("target_branch_url", None),
            branch_url=worker_result.get("branch_url"),
            subpath=worker_result.get("subpath"),
            vcs_type=worker_result.get("vcs_type"),
            transient=worker_result.get("transient"),
            codebase=worker_result.get("codebase"),
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


def is_log_filename(name):
    parts = name.split(".")
    return parts[-1] == "log" or (
        len(parts) == 3 and parts[-2] == "log" and parts[-1].isdigit())


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
        if is_log_filename(entry.name):
            yield entry


async def import_log(
        logfile_manager: LogFileManager, pkg: str, log_id: str, name: str,
        path: str, *, mtime: Optional[int] = None,
        backup_logfile_manager: Optional[LogFileManager] = None):

    try:
        await logfile_manager.import_log(pkg, log_id, path, mtime=mtime)
    except ServiceUnavailable as e:
        logging.warning("Unable to upload logfile %s: %s", name, e)
        primary_logfile_upload_failed_count.inc()
        if backup_logfile_manager:
            await backup_logfile_manager.import_log(pkg, log_id, path, mtime=mtime)
    except asyncio.TimeoutError as e:
        logging.warning("Timeout uploading logfile %s: %s", name, e)
        primary_logfile_upload_failed_count.inc()
        if backup_logfile_manager:
            await backup_logfile_manager.import_log(pkg, log_id, path, mtime=mtime)
    except PermissionDenied as e:
        logging.warning(
            "Permission denied error while uploading logfile %s: %s",
            name, e)
        primary_logfile_upload_failed_count.inc()
        if backup_logfile_manager:
            await backup_logfile_manager.import_log(pkg, log_id, path, mtime=mtime)
    else:
        logfile_uploaded_count.inc()


async def import_logs(
    entries,
    logfile_manager: LogFileManager,
    pkg: str,
    log_id: str,
    *,
    backup_logfile_manager: Optional[LogFileManager] = None,
    mtime: Optional[int] = None,
):
    await asyncio.gather(
        *[import_log(
            logfile_manager, pkg, log_id, entry.name, entry.path,
            mtime=mtime, backup_logfile_manager=backup_logfile_manager)
          for entry in entries])


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
    vcs_info: Optional[Dict[str, str]]

    def __init__(
        self,
        *,
        campaign: str,
        package: str,
        codebase: str,
        change_set: Optional[str],
        command: str,
        instigated_context: Any,
        estimated_duration: Optional[timedelta],
        queue_id: int,
        log_id: str,
        start_time: datetime,
        vcs_info: Optional[Dict[str, str]],
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
        self.codebase = codebase

    @classmethod
    def from_queue_item(
        cls,
        queue_item: QueueItem,
        vcs_info: Optional[Dict[str, str]],
        backchannel: Optional[Backchannel],
        worker_name: str,
        worker_link: Optional[str] = None,
    ):
        return cls(
            campaign=queue_item.campaign,
            package=queue_item.package,
            codebase=queue_item.codebase,
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
        backchannel: Backchannel
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
            codebase=js.get('codebase'),
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
            codebase=self.codebase,
            **kwargs)

    async def ping(self):
        return await self.backchannel.ping(self.log_id)

    @property
    def vcs_type(self):
        if self.vcs_info is None:
            return None
        return self.vcs_info["vcs_type"]

    @property
    def main_branch_url(self):
        if self.vcs_info is None:
            return None
        return self.vcs_info["branch_url"]

    @property
    def subpath(self):
        if self.vcs_info is None:
            return None
        return self.vcs_info["subpath"]

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.json() == other.json()

    def json(self) -> Any:
        """Return a JSON representation."""
        return {
            "queue_id": self.queue_id,
            "id": self.log_id,
            "package": self.package,
            "codebase": self.codebase,
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
                    raise ActiveRunDisappeared('Jenkins job %s has disappeared' % self.my_url) from e
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


def _parse_unexpected_http_status(e):
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
        raise BranchRateLimited(e.path, str(e), retry_after=retry_after) from e


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
    except UnexpectedHttpStatus as e:
        _parse_unexpected_http_status(e)
        raise e
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
            _parse_unexpected_http_status(e)
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
    *,
    run_id: str,
    name: str,
    vcs_type: Optional[str],
    branch_url: Optional[str],
    subpath: Optional[str],
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
    failure_stage: Optional[str] = None,
    target_branch_url: Optional[str] = None,
    change_set: Optional[str] = None,
    failure_transient: Optional[bool] = None,
    codebase: Optional[str] = None,
):
    """Store a run in the database."""
    if result_tags is None:
        result_tags_updated = None
    else:
        result_tags_updated = [(n, r.decode("utf-8")) for (n, r) in result_tags]

    await conn.execute(
        "INSERT INTO run (id, command, description, result_code, "
        "start_time, finish_time, package, instigated_context, context, "
        "main_branch_revision, "
        "revision, result, suite, vcs_type, branch_url, subpath, logfilenames, "
        "value, worker, result_tags, "
        "resume_from, failure_details, failure_stage, target_branch_url, change_set, "
        "failure_transient, codebase) "
        "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, "
        "$12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, "
        "$24, $25, $26, $27)",
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
        subpath,
        logfilenames,
        value,
        worker_name,
        result_tags_updated,
        resume_from,
        failure_details,
        failure_stage,
        target_branch_url,
        change_set,
        failure_transient,
        codebase,
    )

    if result_branches:
        roles = [role for (role, remote_name, br, r) in result_branches]
        assert len(roles) == len(set(roles)), "Duplicate result branches: %r" % result_branches
        await conn.executemany(
            "INSERT INTO new_result_branch "
            "(run_id, role, remote_name, base_revision, revision) "
            "VALUES ($1, $2, $3, $4, $5)",
            [
                (run_id, role, remote_name, br.decode("utf-8") if br else None, r.decode("utf-8") if r else None)
                for (role, remote_name, br, r) in result_branches
            ],
        )


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
        run_timeout: int,
        logfile_manager: LogFileManager,
        artifact_manager: Optional[ArtifactManager] = None,
        public_vcs_managers: Optional[Dict[str, VcsManager]] = None,
        use_cached_only: bool = False,
        committer: Optional[str] = None,
        backup_artifact_manager: Optional[ArtifactManager] = None,
        backup_logfile_manager: Optional[LogFileManager] = None,
        avoid_hosts: Optional[Set[str]] = None,
        dep_server_url: Optional[str] = None,
        apt_archive_url: Optional[str] = None,
    ):
        """Create a queue processor.
        """
        self.database = database
        self.redis = redis
        self.logfile_manager = logfile_manager
        self.artifact_manager = artifact_manager
        self.public_vcs_managers = public_vcs_managers
        self.use_cached_only = use_cached_only
        self.committer = committer
        self.backup_artifact_manager = backup_artifact_manager
        self.backup_logfile_manager = backup_logfile_manager
        self.run_timeout = run_timeout
        self.dep_server_url = dep_server_url
        self.avoid_hosts = avoid_hosts or set()
        self.apt_archive_url = apt_archive_url
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

    async def _healthcheck_active_run(self, active_run, keepalive_age):
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
                        active_run, 'run-disappeared', "no support for ping", transient=True)
                except RunExists:
                    logging.warning('Run exists. Not properly cleaned up?')
                return
        except ActiveRunDisappeared as e:
            if keepalive_age > timedelta(minutes=self.run_timeout):
                try:
                    await self.abort_run(active_run, 'run-disappeared', e.reason,
                                         transient=True)
                except RunExists:
                    logging.warning('Run not properly cleaned up?')
                return
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
                    description=("No keepalives received in %s." % keepalive_age),
                    transient=True)
            except RunExists:
                logging.warning('Run exists. Not properly cleaned up?')
            return

    async def _watchdog(self):
        while True:
            tasks = []
            for serialized in (await self.redis.hgetall('active-runs')).values():
                js = json.loads(serialized)
                active_run = ActiveRun.from_json(js)
                lk = await self.redis.hget('last-keepalive', active_run.log_id)
                if lk:
                    last_keepalive = datetime.fromisoformat(lk.decode('utf-8'))
                else:
                    last_keepalive = active_run.start_time
                keepalive_age = datetime.utcnow() - last_keepalive
                if keepalive_age < timedelta(minutes=(self.run_timeout // 3)):
                    continue
                tasks.append(self._healthcheck_active_run(
                    active_run, keepalive_age))
            if tasks:
                done, _ = await asyncio.wait(tasks)
                for task in done:
                    try:
                        await task
                    except Exception as e:
                        logging.exception(
                            'Failed to healthcheck %s: %r', active_run.log_id, e)
            await asyncio.sleep(self.KEEPALIVE_INTERVAL)

    async def rate_limited_hosts(self):
        for h, t in (await self.redis.hgetall('rate-limit-hosts')).items():
            dt = datetime.fromisoformat(t.decode('utf-8'))
            if dt > datetime.utcnow():
                yield h.decode('utf-8'), dt

    async def active_run_count(self):
        return await self.redis.hlen('active-runs')

    async def estimate_wait(self, package, campaign):
        async with self.database.acquire() as conn:
            queue = Queue(conn)
            (position, wait_time) = await queue.get_position(
                campaign, package)
        active_run_count = await self.active_run_count()
        return (position,
                (wait_time / active_run_count)
                if wait_time is not None else None,
                wait_time)

    async def status_json(self) -> Any:
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
                h: t.isoformat(timespec='seconds')
                async for (h, t) in self.rate_limited_hosts()},
        }

    async def register_run(self, active_run: ActiveRun) -> None:
        async with self.redis.pipeline() as tr:
            tr.hset(
                'active-runs', active_run.log_id, json.dumps(active_run.json()))
            tr.hset(
                'assigned-queue-items', str(active_run.queue_id), active_run.log_id)
            tr.hset(
                'last-keepalive', active_run.log_id, datetime.utcnow().isoformat())
            await tr.execute()
        await self.redis.publish('queue', json.dumps(await self.status_json()))
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
        async with self.redis.pipeline() as tr:
            tr.hdel('assigned-queue-items', str(active_run.queue_id))
            tr.hdel('active-runs', log_id)
            tr.hdel('last-keepalive', log_id)
            await tr.execute()

    async def abort_run(self, run: ActiveRun, code: str, description: str, transient=None) -> None:
        result = run.create_result(
            branch_url=run.main_branch_url,
            vcs_type=run.vcs_type,
            description=description,
            code=code,
            logfilenames=[],
            transient=transient
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
                    subpath=result.subpath,
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
                    failure_stage=result.failure_stage,
                    resume_from=result.resume_from,
                    target_branch_url=result.target_branch_url,
                    change_set=result.change_set,
                    failure_transient=result.transient,
                    codebase=result.codebase,
                )
            except asyncpg.UniqueViolationError as e:
                if ((e.table_name == 'run' and e.column_name == 'id')
                        or e.constraint_name == 'run_pkey'):
                    logging.info('Unique violation error creating run: %r', e)
                    await self.unclaim_run(result.log_id)
                    raise RunExists(result.log_id) from e
                raise
            if result.builder_result:
                await result.builder_result.store(conn, result.log_id)
            await conn.execute("DELETE FROM queue WHERE id = $1", active_run.queue_id)

        await self.redis.publish('result', json.dumps(result.json()))
        await self.unclaim_run(result.log_id)
        await self.redis.publish('queue', json.dumps(await self.status_json()))
        last_success_gauge.set_to_current_time()

    async def rate_limited(self, host, retry_after):
        rate_limited_count.labels(host=host).inc()
        if not retry_after:
            retry_after = datetime.utcnow() + timedelta(seconds=DEFAULT_RETRY_AFTER)
        await self.redis.hset(
            'rate-limit-hosts', host, retry_after.isoformat())

    async def next_queue_item(
            self, conn, package: Optional[str] = None,
            campaign: Optional[str] = None) -> QueueItem:
        queue = Queue(conn)
        exclude_hosts = set(self.avoid_hosts)
        async for host, retry_after in self.rate_limited_hosts():
            exclude_hosts.add(host)
        assigned_queue_items = set([
            int(i.decode('utf-8'))
            for i in await self.redis.hkeys('assigned-queue-items')])
        return await queue.next_item(
            campaign=campaign, package=package,
            assigned_queue_items=assigned_queue_items)


@routes.get("/queue/position", name="queue-position")
async def handle_queue_position(request):
    span = aiozipkin.request_span(request)
    package = request.query['package']
    campaign = request.query['campaign']
    with span.new_child('sql:queue-position'):
        (position, wait_time,
         cum_wait_time) = await request.app['queue_processor'].estimate_wait(
            package, campaign)

    return web.json_response({
        "position": position,
        "wait_time":
            wait_time.total_seconds() if wait_time is not None else None,
        "cumulative_wait_time":
            cum_wait_time.total_seconds()
            if cum_wait_time is not None else None,
    })


@routes.post("/schedule-control", name="schedule-control")
async def handle_schedule_control(request):
    span = aiozipkin.request_span(request)
    json = await request.json()
    change_set = json.get('change_set')
    offset = json.get('offset')
    requestor = json['requestor']
    refresh = json.get('refresh', False)
    bucket = json.get('bucket')
    estimated_duration = (
        timedelta(seconds=json['estimated_duration'])
        if json.get('estimated_duration') else None)

    async with request.app['database'].acquire() as conn:
        try:
            run_id = json['run_id']
        except KeyError:
            package = json['package']
            codebase = json.get('codebase')
            main_branch_revision = json['main_branch_revision'].encode('utf-8')
        else:
            with span.new_child('sql:find-run'):
                run = await conn.fetchrow(
                    "SELECT main_branch_revision, package, codebase FROM run "
                    "WHERE id = $1",
                    run_id)
            if run is None:
                return web.json_response({"reason": "Run not found"}, status=404)
            package = run['package']
            codebase = run['codebase']
            main_branch_revision = run['main_branch_revision'].encode('utf-8')
        with span.new_child('do-schedule-control'):
            offset, estimated_duration, queue_id = await do_schedule_control(
                conn,
                package=package,
                change_set=change_set,
                main_branch_revision=main_branch_revision,
                offset=offset,
                refresh=refresh,
                bucket=bucket,
                requestor=requestor,
                codebase=codebase,
                estimated_duration=estimated_duration)

    response_obj = {
        "package": package,
        "campaign": "control",
        "offset": offset,
        "bucket": bucket,
        "queue_id": queue_id,
        "estimated_duration_seconds":
            estimated_duration.total_seconds() if estimated_duration else None,
    }
    return web.json_response(response_obj)


@routes.post("/schedule", name="schedule")
async def handle_schedule(request):
    span = aiozipkin.request_span(request)
    json = await request.json()
    async with request.app['database'].acquire() as conn:
        try:
            run_id = json['run_id']
        except KeyError:
            package = json['package']
            campaign = json['campaign']
            codebase = json.get('codebase')
            run = None
        else:
            run = await conn.fetchrow(
                "SELECT suite AS campaign, package, codebase, command FROM run WHERE id = $1",
                run_id)
            if run is None:
                return web.json_response({"reason": "Run not found"}, status=404)
            package = run['package']
            campaign = run['campaign']
            codebase = run.get('codebase')
        refresh = json.get('refresh', False)
        change_set = json.get('change_set')
        requestor = json.get('requestor')
        bucket = json.get('bucket')
        offset = json.get('offset')
        estimated_duration = (
            timedelta(seconds=json['estimated_duration'])
            if json.get('estimated_duration') else None)
        command = await conn.fetchval(
            "SELECT command "
            "FROM candidate WHERE package = $1 AND suite = $2",
            package, campaign)
        if command is None:
            command = get_campaign_config(
                request.app['config'], campaign).command
        if command is None and run is not None:
            command = run['command']
        if command is None:
            raise web.HTTPBadRequest(text="no command specified")

        try:
            with span.new_child('do-schedule'):
                offset, estimated_duration, queue_id, = await do_schedule(
                    conn,
                    package,
                    campaign,
                    offset=offset,
                    change_set=change_set,
                    refresh=refresh,
                    requestor=requestor,
                    estimated_duration=estimated_duration,
                    codebase=codebase,
                    command=command,
                    bucket=bucket)
        except CandidateUnavailable as e:
            raise web.HTTPBadRequest(text="Candidate not available") from e

    response_obj = {
        "package": package,
        "campaign": campaign,
        "offset": offset,
        "bucket": bucket,
        "queue_id": queue_id,
        "estimated_duration_seconds":
            estimated_duration.total_seconds() if estimated_duration else None,
    }
    return web.json_response(response_obj)


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
    return active_run


@routes.get("/log/{run_id}", name="log-index")
async def handle_log_index(request):
    active_run = await _find_active_run(request)
    log_filenames = await active_run.backchannel.list_log_files()
    return web.json_response(log_filenames)


@routes.post("/kill/{run_id}", name="kill")
async def handle_kill(request):
    active_run = await _find_active_run(request)
    ret = active_run.json()
    try:
        await active_run.backchannel.kill()
    except NotImplementedError as e:
        raise web.HTTPNotImplemented(
            text='kill not supported for this type of run') from e
    else:
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
            status=200, reason="OK", headers={"Content-Type": "text/plain"}
        )
        await response.prepare(request)
        for chunk in f:
            await response.write(chunk)
        await response.write_eof()
    finally:
        f.close()
    return response


@routes.get("/codebases", name="download-codebases")
async def handle_codebases_download(request):
    queue_processor = request.app['queue_processor']
    codebases = []

    async with queue_processor.database.acquire() as conn:
        for row in await conn.fetch(
                'SELECT name, branch_url, url, branch, subpath, vcs_type, '
                'vcs_last_revision, value FROM codebase'):
            codebases.append(dict(row))

    return web.json_response(codebases)


@routes.post("/codebases", name="upload-codebases")
async def handle_codebases_upload(request):
    queue_processor = request.app['queue_processor']

    codebases = []
    for entry in await request.json():
        if 'branch_url' in entry:
            entry['url'], params = urlutils.split_segment_parameters(
                entry['branch_url'])
            if 'branch' in params:
                entry['branch'] = urlutils.unescape(params['branch'])
        elif 'branch' in entry:
            entry['branch_url'] = urlutils.join_segment_parameters(
                entry['url'], {'branch': urlutils.escape(entry['branch'])})
        elif 'url' in entry:
            entry['branch_url'] = entry['url']
        else:
            entry['branch_url'] = entry['url'] = None

        codebases.append((
            entry.get('name'),
            entry['branch_url'],
            entry['url'],
            entry.get('branch'),
            entry.get('subpath'),
            entry.get('vcs_type'),
            entry.get('vcs_last_revision'),
            entry.get('value')))

    async with queue_processor.database.acquire() as conn:
        # TODO(jelmer): When a codebase with a certain name already exists,
        # steal its name
        await conn.executemany(
            "INSERT INTO codebase "
            "(name, branch_url, url, branch, subpath, vcs_type, "
            "vcs_last_revision, value) "
            "VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
            "ON CONFLICT (name) DO UPDATE SET "
            "branch_url = EXCLUDED.branch_url, subpath = EXCLUDED.subpath, "
            "vcs_type = EXCLUDED.vcs_type, "
            "vcs_last_revision = EXCLUDED.vcs_last_revision, "
            "value = EXCLUDED.value, url = EXCLUDED.url, "
            "branch = EXCLUDED.branch",
            codebases)

    return web.json_response({})


@routes.delete("/candidates/{id}", name="delete-candidate")
async def handle_candidate_delete(request):
    queue_processor = request.app['queue_processor']
    candidate_id = int(request.match_info['id'])
    async with queue_processor.database.acquire() as conn, conn.transaction():
        await conn.fetchrow(
            'DELETE FROM followup WHERE candidate = $1', candidate_id)
        (suite, codebase) = await conn.fetchrow(
            'DELETE FROM candidate WHERE id = $1 RETURNING suite, codebase', candidate_id)
        await conn.execute(
            'DELETE FROM queue WHERE suite = $1 AND codebase = $2',
            suite, codebase)
        return web.json_response({})


@routes.get("/candidates", name="download-candidates")
async def handle_candidate_download(request):
    queue_processor = request.app['queue_processor']
    ret = []
    async with queue_processor.database.acquire() as conn:
        for row in await conn.fetch('SELECT * FROM candidate'):
            ret.append({
                'id': row['id'],
                'package': row['package'],
                'codebase': row['codebase'],
                'campaign': row['suite'],
                'command': row['command'],
                'publish-policy': row['publish_policy'],
                'change_set': row['change_set'],
                'context': row['context'],
                'value': row['value'],
                'success_chance': row['success_chance'],
            })
    return web.json_response(ret)


@routes.post("/candidates", name="upload-candidates")
async def handle_candidates_upload(request):
    unknown_packages = []
    unknown_codebases = []
    unknown_campaigns = []
    invalid_command = []
    unknown_publish_policies = []
    queue_processor = request.app['queue_processor']
    to_schedule = []
    async with queue_processor.database.acquire() as conn:
        async with conn.transaction():
            known_packages = set()
            for record in (await conn.fetch('SELECT name FROM package')):
                known_packages.add(record[0])

            known_codebases = set()
            for record in (await conn.fetch('SELECT name FROM codebase WHERE name IS NOT NULL')):
                known_codebases.add(record[0])

            known_campaign_names = [
                campaign.name for campaign in request.app['config'].campaign]

            known_publish_policies = set()
            for record in (await conn.fetch(
                    'SELECT name FROM named_publish_policy')):
                known_publish_policies.add(record[0])

            candidate_rows = []
            followups = []
            for candidate in (await request.json()):
                if candidate['package'] not in known_packages:
                    logging.warning(
                        'ignoring candidate %s/%s; package unknown',
                        candidate['package'], candidate['campaign'])
                    unknown_packages.append(candidate['package'])
                    continue
                if candidate['codebase'] not in known_codebases:
                    logging.warning(
                        'ignoring candidate %s/%s; codebase unknown',
                        candidate['codebase'], candidate['campaign'])
                    unknown_codebases.append(candidate['codebase'])
                    continue
                if candidate['campaign'] not in known_campaign_names:
                    logging.warning('unknown suite %r', candidate['campaign'])
                    unknown_campaigns.append(candidate['campaign'])
                    continue

                command = candidate.get('command')
                if not command:
                    campaign_config = get_campaign_config(
                        request.app['config'], candidate['campaign'])
                    command = campaign_config.command
                    if not command:
                        logging.warning(
                            'No command in candidate or campaign config')
                        invalid_command.append(command)
                        continue

                publish_policy = candidate.get('publish-policy')
                if (publish_policy is not None
                        and publish_policy not in known_publish_policies):
                    logging.warning('unknown publish policy %s', publish_policy)
                    unknown_publish_policies.append(publish_policy)
                    continue

                candidate_rows.append((
                    candidate['package'], candidate['campaign'],
                    command,
                    candidate.get('change_set'), candidate.get('context'),
                    candidate.get('value'), candidate.get('success_chance'),
                    publish_policy, candidate['codebase']))

                followups.append(candidate.get('followup_for', []))

                # Adjust bucket if there are any open merge proposals with a
                # different command
                existing_runs = await conn.fetch(
                    "SELECT merge_proposal.url AS mp_url, "
                    "last_effective_runs.command AS command "
                    "FROM last_effective_runs "
                    "LEFT JOIN merge_proposal "
                    "ON last_effective_runs.revision = merge_proposal.revision "
                    "WHERE merge_proposal.status = 'open' "
                    "AND last_effective_runs.codebase = $1 "
                    "AND last_effective_runs.suite = $2 "
                    "AND last_effective_runs.command != $3",
                    candidate['codebase'], candidate['campaign'], command)
                if any(existing_runs):
                    refresh = True
                    if existing_runs[0]['mp_url']:
                        bucket = 'update-existing-mp'
                        requestor = 'command changed for existing mp: %r ⇒ %r' % (
                            existing_runs[0]['command'], command)
                    else:
                        bucket = None
                        requestor = 'command changed: %r ⇒ %r' % (
                            existing_runs[0]['command'], command)
                else:
                    bucket = candidate.get('bucket')
                    refresh = False
                    requestor = "candidate update"

                if candidate.get('requestor'):
                    requestor += f' {candidate["requestor"]}' 

                to_schedule.append((
                    candidate['package'],
                    candidate['codebase'],
                    candidate['campaign'],
                    candidate.get('change_set'),
                    command,
                    bucket,
                    requestor,
                    refresh,
                ))

            candidates = await conn.fetch(
                "INSERT INTO candidate "
                "(package, suite, command, change_set, context, value, "
                "success_chance, publish_policy, codebase) "
                "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) "
                "ON CONFLICT (package, suite, coalesce(change_set, ''::text)) "
                "DO UPDATE SET context = EXCLUDED.context, value = EXCLUDED.value, "
                "success_chance = EXCLUDED.success_chance, "
                "command = EXCLUDED.command, "
                "publish_policy = EXCLUDED.publish_policy, "
                "codebase = EXCLUDED.codebase RETURNING id",
                candidate_rows,
            )

            followup_rows = []
            for candidate, followup_ids in zip(candidates, followups):
                if not followup_ids:
                    continue
                for followup_id in followup_ids:
                    followup_rows.append((candidate['id'], followup_id))

            if followup_rows:
                await conn.executemany(
                    "INSERT INTO followup (origin, candidate) VALUES ($1, $2) "
                    "ON CONFLICT DO NOTHING",
                    followup_rows)

        ret = []

        for (package, codebase, campaign, change_set, command, bucket,
             requestor, refresh) in to_schedule:
            offset, estimated_duration, queue_id, = await do_schedule(
                conn,
                package,
                campaign,
                change_set=change_set,
                bucket=bucket,
                requestor=requestor,
                command=command,
                codebase=codebase,
                refresh=refresh)
            ret.append({
                'campaign': campaign,
                'codebase': codebase,
                'bucket': bucket,
                'change_set': change_set,
                'offset': offset,
                'estimated_duration': estimated_duration.total_seconds()
                if estimated_duration is not None else None,
                'queue-id': queue_id,
                'refresh': refresh
            })

    return web.json_response({
        'success': ret,
        'invalid_command': invalid_command,
        'unknown_campaigns': unknown_campaigns,
        'unknown_codebases': unknown_codebases,
        'unknown_publish_policies': unknown_publish_policies,
        'unknown_packages': unknown_packages})


@routes.get("/active-runs", name="get-active-runs")
async def handle_get_active_runs(request):
    queue_processor = request.app['queue_processor']
    return web.json_response((await queue_processor.status_json())["processing"])


@routes.get("/active-runs/{run_id}", name="get-active-run")
async def handle_get_active_run(request):
    queue_processor = request.app['queue_processor']
    run_id = request.match_info['run_id']
    active_run = await queue_processor.get_run(run_id)
    if not active_run:
        raise web.HTTPNotFound(text='no such run %s' % run_id)
    return web.json_response(active_run.json())


@routes.post("/active-runs", name="assign")
async def handle_assign(request):
    json = await request.json()
    assignment_count.labels(worker=json.get("worker")).inc()
    span = aiozipkin.request_span(request)
    queue_processor = request.app['queue_processor']
    try:
        assignment = await next_item(
            queue_processor, request.app['config'],
            span, 'assign', worker=json.get("worker"),
            worker_link=json.get("worker_link"),
            backchannel=json.get('backchannel'),
            package=json.get('package'),
            campaign=json.get('campaign')
        )
    except QueueEmpty:
        return web.json_response({'reason': 'queue empty'}, status=503)
    except QueueRateLimiting as e:
        return web.json_response(
            {'reason': str(e)}, status=429, headers={
                'Retry-After': str(e.retry_after or DEFAULT_RETRY_AFTER)})
    return web.json_response(
        assignment, status=201, headers={
            'Location': str(request.app.router['get-active-run'].url_for(
                run_id=assignment['id']))
        })


async def handle_public_assign(request):
    json = await request.json()
    span = aiozipkin.request_span(request)
    with span.new_child('check-worker-creds'):
        worker_name = await check_worker_creds(request.app['database'], request)
    assignment_count.labels(worker=worker_name).inc()
    queue_processor = request.app['queue_processor']
    try:
        assignment = await next_item(
            queue_processor, request.app['config'],
            span, 'assign', worker=worker_name,
            worker_link=json.get("worker_link"),
            backchannel=json.get('backchannel'),
            package=json.get('package'),
            campaign=json.get('campaign')
        )
    except QueueEmpty:
        return web.json_response({'reason': 'queue empty'}, status=503)
    except QueueRateLimiting as e:
        return web.json_response(
            {'reason': str(e)}, status=429, headers={
                'Retry-After': str(e.retry_after or DEFAULT_RETRY_AFTER)})
    return web.json_response(
        assignment, status=201, headers={
            'Location': str(request.app.router['get-active-run'].url_for(
                run_id=assignment['id']))
        })


@routes.get("/active-runs/+peek", name="peek")
async def handle_peek(request):
    span = aiozipkin.request_span(request)
    queue_processor = request.app['queue_processor']
    try:
        assignment = await next_item(
            queue_processor, request.app['config'], span, 'peek')
    except QueueEmpty:
        return web.json_response({'reason': 'queue empty'}, status=503)
    except QueueRateLimiting as e:
        return web.json_response(
            {'reason': str(e)}, status=429, headers={
                'Retry-After': str(e.retry_after or DEFAULT_RETRY_AFTER)})
    return web.json_response(
        assignment, status=201, headers={
            'Location': str(request.app.router['get-active-run'].url_for(
                run_id=assignment['id']))
        })


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


class QueueEmpty(Exception):
    """Queue is empty."""


class QueueRateLimiting(Exception):
    """Rate limiting encountered while getting queue item."""

    def __init__(self, retry_after):
        self.retry_after = retry_after


async def next_item(
        queue_processor, config, span, mode, *, worker=None,
        worker_link: Optional[str] = None,
        backchannel: Optional[Dict[str, str]] = None,
        package: Optional[str] = None, campaign: Optional[str] = None):
    possible_transports: List[Transport] = []
    possible_forges: List[Forge] = []

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

    async with queue_processor.database.acquire() as conn:
        item = None
        while item is None:
            with span.new_child('sql:queue-item'):
                item, vcs_info = await queue_processor.next_queue_item(
                    conn, package=package, campaign=campaign)
            if item is None:
                queue_empty_count.inc()
                raise QueueEmpty()

            bc: Backchannel
            if backchannel and backchannel['kind'] == 'http':
                bc = PollingBackchannel(my_url=URL(backchannel['url']))
            elif backchannel and backchannel['kind'] == 'jenkins':
                bc = JenkinsBackchannel(my_url=URL(backchannel['url']))
            else:
                bc = Backchannel()

            active_run = ActiveRun.from_queue_item(
                backchannel=bc,
                worker_name=worker,
                queue_item=item,
                vcs_info=vcs_info,
                worker_link=worker_link
            )

            await queue_processor.register_run(active_run)

            try:
                campaign_config = get_campaign_config(config, item.campaign)
            except KeyError:
                logging.warning(
                    'Unable to find details for campaign %r', item.campaign)
                await abort(active_run, 'unknown-campaign',
                            "Campaign %s unknown" % item.campaign)
                item = None
                continue

            if not campaign_config.default_empty and (
                    vcs_info is None or vcs_info["branch_url"] is None):
                await abort(active_run, 'not-in-vcs', "No VCS URL known for package.")
                item = None
                continue

        # TODO(jelmer): Handle exceptions from get_builder
        builder = get_builder(
            config, campaign_config,
            queue_processor.apt_archive_url,
            queue_processor.dep_server_url)

        with span.new_child('build-env'):
            build_env = await builder.build_env(conn, campaign_config, item)

        with span.new_child('config'):
            build_config = await builder.config(conn, campaign_config, item)

        if vcs_info and vcs_info["branch_url"] is not None:
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
                raise QueueRateLimiting(e.retry_after) from e
            except BranchOpenFailure as e:
                logging.debug(
                    'Error opening branch %s: %s', vcs_info['branch_url'],
                    e)
                resume_branch = None
                additional_colocated_branches = None
                vcs_type = vcs_info['vcs_type']
            except asyncio.TimeoutError:
                logging.debug('Timeout opening branch %s', vcs_info['branch_url'])
                resume_branch = None
                additional_colocated_branches = None
                vcs_type = vcs_info['vcs_type']
            else:
                # We try the public branch first, since perhaps a maintainer
                # has made changes to the branch there.
                active_run.vcs_info["branch_url"] = full_branch_url(main_branch).rstrip('/')
                additional_colocated_branches = await asyncio.to_thread(
                    builder.additional_colocated_branches, main_branch)
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
                            raise QueueRateLimiting(e.retry_after) from e
                        except asyncio.TimeoutError:
                            logging.debug('Timeout opening resume branch')
                            resume_branch = None
                else:
                    resume_branch = None
        else:
            active_run.vcs_info = None
            vcs_type = None
            resume_branch = None
            additional_colocated_branches = None
            vcs_info = {}

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
                            item.package, '%s/%s' % (campaign_config.name, 'main'),
                            trace_context=span.context)
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
                    config,
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

    env: Dict[str, str] = {}
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
            "default-empty": campaign_config.default_empty,
            "url": vcs_info.get('branch_url'),
            "subpath": vcs_info.get('subpath'),
            "vcs_type": vcs_info.get('vcs_type'),
            "cached_url": cached_branch_url,
            "additional_colocated_branches": additional_colocated_branches,
        },
        "resume": resume.json() if resume else None,
        "build": {
            "target": builder.kind,
            "environment": build_env,
            "config": build_config,
        },
        "command": command,
        "codebase": item.codebase,
        "codemod": {"command": command, "environment": {}},
        "env": env,
        "campaign": item.campaign,
        "force-build": campaign_config.force_build,
        "skip-setup-validation": campaign_config.skip_setup_validation,
        "target_repository": {
            "url": target_repository_url,
            "vcs_type": vcs_info.get('vcs_type'),
        }
    }

    if mode == 'assign':
        pass
    else:
        await queue_processor.unclaim_run(active_run.log_id)
    return assignment


@routes.get("/health", name="health")
async def handle_health(request):
    return web.Response(text="ok")


@routes.get("/ready", name="ready")
async def handle_ready(request):
    return web.Response(text="ok")


async def finish(
        active_run: ActiveRun, queue_processor: QueueProcessor,
        request: web.Request) -> Tuple[
            List[str], List[str], List[str], JanitorResult]:
    span = aiozipkin.request_span(request)
    worker_name = active_run.worker_name
    main_branch_url = active_run.main_branch_url
    vcs_type = active_run.vcs_type
    subpath = active_run.subpath
    resume_from = active_run.resume_from

    reader = await request.multipart()
    worker_result = None

    filenames = []
    with tempfile.TemporaryDirectory(prefix='janitor-run') as output_directory:
        with span.new_child('read-files'):
            while True:
                part = await reader.next()
                if part is None:
                    break
                if isinstance(part, MultipartReader):
                    raise web.HTTPBadRequest(text='nested multi-part')
                if part.filename == "result.json":
                    worker_result = WorkerResult.from_json(await part.json())
                elif part.filename is None:
                    raise web.HTTPBadRequest(text="Part without filename")
                else:
                    filenames.append(part.filename)
                    output_path = os.path.join(output_directory, part.filename)
                    with open(output_path, "wb") as f:
                        try:
                            f.write(await part.read())
                        except ConnectionResetError as e:
                            raise web.HTTPBadRequest(text=str(e)) from e

        if worker_result is None:
            raise web.HTTPBadRequest(text="Missing result JSON")

        logging.debug('worker result: %r', worker_result)

        if worker_name is None:
            worker_name = worker_result.worker_name

        with span.new_child('gather-logs'):
            logfiles = list(gather_logs(output_directory))

        logfilenames = [entry.name for entry in logfiles]

        result = JanitorResult(
            pkg=active_run.package,
            campaign=active_run.campaign,
            log_id=active_run.log_id,
            code='success',
            worker_name=worker_name,
            branch_url=main_branch_url,
            vcs_type=vcs_type,
            subpath=subpath,
            worker_result=worker_result,
            logfilenames=logfilenames,
            resume_from=resume_from,
            change_set=active_run.change_set,
            codebase=active_run.codebase,
        )

        with span.new_child('import-logs'):
            await import_logs(
                logfiles,
                queue_processor.logfile_manager,
                active_run.package,
                active_run.log_id,
                mtime=result.finish_time.timestamp(),
                backup_logfile_manager=queue_processor.backup_logfile_manager,
            )

        if result.builder_result is not None:
            result.builder_result.from_directory(output_directory)

            artifact_names = result.builder_result.artifact_filenames()
            with span.new_child('upload-artifacts-with-backup'):
                try:
                    await store_artifacts_with_backup(
                        queue_processor.artifact_manager,
                        queue_processor.backup_artifact_manager,
                        output_directory,
                        active_run.log_id,
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

    with span.new_child('finish-run'):
        await queue_processor.finish_run(active_run, result)

    return (filenames, logfilenames, artifact_names, result)


@routes.post("/active-runs/{run_id}/finish", name="finish")
async def handle_finish(request):
    queue_processor = request.app['queue_processor']
    run_id = request.match_info["run_id"]
    active_run = await queue_processor.get_run(run_id)
    if not active_run:
        raise web.HTTPNotFound(text='no such run %s' % run_id)
    try:
        (filenames, logfilenames, artifact_names, result) = await finish(
            active_run, queue_processor, request)
    except RunExists as e:
        return web.json_response(
            {"id": run_id, "filenames": filenames, "artifacts": artifact_names,
             "logs": logfilenames,
             "result": result.json(), 'reason': str(e)},
            status=409,
        )

    # TODO(jelmer): Set Location header to something; /runs/{run_id}= ?
    return web.json_response(
        {"id": run_id, "filenames": filenames,
         "logs": logfilenames,
         "artifacts": artifact_names, "result": result.json()},
        status=201,
    )


async def handle_public_get_active_run(request):
    queue_processor = request.app['queue_processor']
    run_id = request.match_info['run_id']
    active_run = await queue_processor.get_run(run_id)
    if not active_run:
        raise web.HTTPNotFound(text='no such run %s' % run_id)
    return web.json_response(active_run.json())


async def handle_public_finish(request):
    span = aiozipkin.request_span(request)
    queue_processor = request.app['queue_processor']
    run_id = request.match_info["run_id"]
    active_run = await queue_processor.get_run(run_id)
    if not active_run:
        raise web.HTTPNotFound(text='no such run %s' % run_id)

    with span.new_child('check-worker-creds'):
        await check_worker_creds(request.app['database'], request)

    try:
        (filenames, logfilenames, artifact_names, result) = await finish(
            active_run, queue_processor, request)
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


async def handle_public_root(request):
    return web.Response(text='')


async def create_public_app(queue_processor, config, db, tracer=None):
    app = web.Application(middlewares=[
        state.asyncpg_error_middleware])
    app['config'] = config
    app['database'] = db
    app['queue_processor'] = queue_processor
    app.middlewares.insert(0, metrics_middleware)
    app.router.add_get('/', handle_public_root)
    app.router.add_post('/runner/active-runs', handle_public_assign)
    app.router.add_post(
        '/runner/active-runs/{run_id}/finish', handle_public_finish)
    app.router.add_get(
        '/runner/active-runs/{run_id}',
        handle_public_get_active_run,
        name='get-active-run')
    aiozipkin.setup(app, tracer)
    return app


async def create_app(queue_processor, config, db, tracer=None):
    app = web.Application(middlewares=[
        state.asyncpg_error_middleware])
    app.router.add_routes(routes)
    app['config'] = config
    app['database'] = db
    app['queue_processor'] = queue_processor
    app.middlewares.insert(0, metrics_middleware)
    metrics_route = app.router.add_get("/metrics", metrics, name="metrics")
    aiozipkin.setup(app, tracer, skip_routes=[metrics_route])
    return app


async def main(argv=None):
    import argparse

    parser = argparse.ArgumentParser(prog="janitor.runner")
    parser.add_argument(
        "--listen-address", type=str, help="Listen address", default="localhost"
    )
    parser.add_argument("--port", type=int, help="Listen port", default=9911)
    parser.add_argument("--public-port", type=int, help="Listen port", default=9919)
    parser.add_argument(
        "--pre-check",
        help="Command to run to check whether to process package.",
        type=str,
    )
    parser.add_argument(
        "--post-check", help="Command to run to check package before pushing.", type=str
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
        "--public-vcs-location", type=str, default=None,
        help="Public vcs location (used for URLs handed to worker)"
    )
    parser.add_argument(
        "--public-apt-archive-location", 
        type=str,
        default=None,
        help="Base location for our own APT archive")
    parser.add_argument("--public-dep-server-url", type=str, default=None)
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

    set_user_agent(config.user_agent)

    try:
        public_vcs_managers = get_vcs_managers(args.public_vcs_location)
    except UnsupportedProtocol as e:
        parser.error(
            'Unsupported protocol in --public-vcs-location: %s' % e.path)

    endpoint = aiozipkin.create_endpoint("janitor.runner", ipv4=args.listen_address, port=args.port)
    if config.zipkin_address:
        tracer = await aiozipkin.create(config.zipkin_address, endpoint, sample_rate=0.1)
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
        await stack.enter_async_context(logfile_manager)
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
            await stack.enter_async_context(backup_logfile_manager)
            loop.create_task(
                upload_backup_artifacts(
                    backup_artifact_manager, artifact_manager, timeout=60 * 15
                )
            )
        else:
            backup_artifact_manager = None
            backup_logfile_manager = None
        db = await state.create_pool(config.database_location)
        redis = Redis.from_url(config.redis_location)
        stack.push_async_callback(redis.close)
        queue_processor = QueueProcessor(
            db, redis,
            run_timeout=args.run_timeout,
            logfile_manager=logfile_manager,
            artifact_manager=artifact_manager,
            public_vcs_managers=public_vcs_managers,
            use_cached_only=args.use_cached_only,
            committer=config.committer,
            backup_artifact_manager=backup_artifact_manager,
            backup_logfile_manager=backup_logfile_manager,
            avoid_hosts=set(args.avoid_host),
            dep_server_url=args.public_dep_server_url,
            apt_archive_url=args.public_apt_archive_location,
        )

        queue_processor.start_watchdog()

        if args.public_port:
            public_app = await create_public_app(
                queue_processor, config, db, tracer=tracer)
            public_runner = web.AppRunner(public_app)
            await public_runner.setup()
            public_site = web.TCPSite(
                public_runner, args.listen_address, port=args.public_port)
            await public_site.start()

        app = await create_app(queue_processor, config, db, tracer=tracer)
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, args.listen_address, port=args.port)
        await site.start()
        while True:
            await asyncio.sleep(3600)


if __name__ == "__main__":
    sys.exit(asyncio.run(main(sys.argv)))
