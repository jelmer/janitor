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
import asyncpg
from datetime import datetime, timedelta
from email.utils import parseaddr
import functools
import json
from io import BytesIO
import logging
import os
import signal
import socket
import sys
import tempfile
from typing import List, Any, Optional, Iterable, BinaryIO, Dict, Tuple, Set
import uuid

from aiohttp import (
    web,
    WSMsgType,
)

from yarl import URL

from breezy import debug
from breezy.branch import Branch
from breezy.errors import PermissionDenied
from breezy.propose import Hoster, HosterLoginRequired
from breezy.transport import Transport

from prometheus_client import Counter, Gauge, Histogram

from silver_platter.debian import (
    pick_additional_colocated_branches,
    select_preferred_probers,
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
    full_branch_url,
)

from . import (
    state,
)
from .artifacts import (
    get_artifact_manager,
    ArtifactManager,
    LocalArtifactManager,
    store_artifacts_with_backup,
    upload_backup_artifacts,
)
from .config import read_config, get_suite_config, Config
from .debian import (
    changes_filenames,
    open_guessed_salsa_branch,
    find_changes,
    NoChangesFile,
)
from .debian import state as debian_state
from .logs import (
    get_log_manager,
    ServiceUnavailable,
    LogFileManager,
    FileSystemLogFileManager,
)
from .prometheus import setup_metrics
from .pubsub import Topic, pubsub_handler
from .schedule import do_schedule
from .vcs import (
    get_vcs_abbreviation,
    is_authenticated_url,
    open_branch_ext,
    BranchOpenFailure,
    LocalVcsManager,
    RemoteVcsManager,
    UnsupportedVcs,
    VcsManager,
    legacy_import_branches,
    import_branches,
)

apt_package_count = Gauge(
    "apt_package_count", "Number of packages with a version published", ["suite"]
)
packages_processed_count = Counter("package_count", "Number of packages processed.")
queue_length = Gauge(
    "queue_length", "Number of items in the queue.", labelnames=("bucket",)
)
queue_duration = Gauge(
    "queue_duration", "Time to process all items in the queue sequentially"
)
last_success_gauge = Gauge(
    "job_last_success_unixtime", "Last time a batch job successfully finished"
)
build_duration = Histogram("build_duration", "Build duration", ["package", "suite"])
current_tick = Gauge(
    "current_tick",
    "The current tick in the queue that's being processed",
    labelnames=("bucket",),
)
run_count = Gauge("run_count", "Number of total runs.", labelnames=("suite",))
run_result_count = Gauge(
    "run_result_count", "Number of runs by code.", labelnames=("suite", "result_code")
)
never_processed_count = Gauge(
    "never_processed_count", "Number of items never processed.", labelnames=("suite",)
)
review_status_count = Gauge(
    "review_status_count", "Last runs by review status.", labelnames=("review_status",)
)


class BuilderResult(object):

    kind: str

    def from_directory(self, path, codebase):
        raise NotImplementedError(self.from_directory)

    async def store(self, conn, run_id, package):
        raise NotImplementedError(self.store)

    def json(self):
        raise NotImplementedError(self.json)

    def artifact_filenames(self):
        raise NotImplementedError(self.artifact_filenames)


class Builder(object):
    """Abstract builder class."""

    result_cls = BuilderResult


class DebianResult(BuilderResult):

    kind = "debian"

    def __init__(
        self, build_version=None, build_distribution=None, changes_filename=None
    ):
        self.build_version = build_version
        self.build_distribution = build_distribution
        self.changes_filename = changes_filename

    def from_directory(self, path, package):
        self.output_directory = path
        (
            self.changes_filename,
            self.build_version,
            self.build_distribution,
        ) = find_changes(path, package)

    def artifact_filenames(self):
        if not self.changes_filename:
            return []
        changes_path = os.path.join(self.output_directory, self.changes_filename)
        return list(changes_filenames(changes_path)) + [
            os.path.basename(self.changes_filename)
        ]

    @classmethod
    def from_worker_result(cls, worker_result):
        build_version = worker_result.build_version if worker_result else None
        build_distribution = worker_result.build_distribution if worker_result else None
        changes_filename = worker_result.changes_filename if worker_result else None
        return cls(
            build_version=build_version,
            build_distribution=build_distribution,
            changes_filename=changes_filename,
        )

    async def store(self, conn, run_id, package):
        if self.build_version:
            await debian_state.store_debian_build(
                conn,
                run_id,
                package,
                self.build_version,
                self.build_distribution,
            )

    def json(self):
        return {
            "build_distribution": self.build_distribution,
            "build_version": self.build_version,
            "changes_filename": self.changes_filename,
        }

    def __bool__(self):
        return self.changes_filename is not None


class DebianBuilder(Builder):

    result_cls = DebianResult

    def __init__(self, distro_config, apt_location):
        self.distro_config = distro_config
        self.apt_location = apt_location

    async def build_env(self, conn, suite_config, queue_item):
        if self.apt_location.startswith("gs://"):
            bucket_name = URL(self.apt_location).host
            apt_location = "https://storage.googleapis.com/%s/" % bucket_name
        env = {
            "EXTRA_REPOSITORIES": ":".join(
                [
                    "deb %s %s/ main" % (apt_location, suite)
                    for suite in suite_config.debian_build.extra_build_suite
                ]
            )
        }

        if suite_config.debian_build.chroot:
            env["CHROOT"] = suite_config.debian_build.chroot
        elif self.distro_config.chroot:
            env["CHROOT"] = self.distro_config.chroot

        if self.distro_config.name:
            env["DISTRIBUTION"] = self.distro_config.name

        env["REPOSITORIES"] = "%s %s/ %s" % (
            self.distro_config.archive_mirror_uri,
            self.distro_config.name,
            " ".join(self.distro_config.component),
        )

        env["BUILD_DISTRIBUTION"] = suite_config.debian_build.build_distribution or ""
        env["BUILD_SUFFIX"] = suite_config.debian_build.build_suffix or ""

        last_build_version = await debian_state.get_last_build_version(
            conn, queue_item.package, queue_item.suite
        )

        if last_build_version:
            env["LAST_BUILD_VERSION"] = str(last_build_version)

        env.update([(env.key, env.value) for env in suite_config.debian_build.sbuild_env])
        return env


class JanitorResult(object):
    def __init__(
        self,
        pkg,
        log_id,
        branch_url,
        description=None,
        code=None,
        worker_result=None,
        worker_cls=None,
        logfilenames=None,
        legacy_branch_name=None,
    ):
        self.package = pkg
        self.log_id = log_id
        self.description = description
        self.branch_url = branch_url
        self.code = code
        self.legacy_branch_name = legacy_branch_name
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
            # TODO(jelmer): use Builder.worker_cls here rather than DebianResult
            self.builder_result = DebianResult.from_worker_result(worker_result)
            self.branches = worker_result.branches
            self.tags = worker_result.tags
            self.remotes = worker_result.remotes
            self.failure_details = worker_result.details
        else:
            self.context = None
            self.main_branch_revision = None
            self.revision = None
            self.subworker_result = None
            self.value = None
            self.builder_result = DebianResult()
            self.branches = None
            self.tags = None
            self.failure_details = None
            self.remotes = {}

    def json(self):
        return {
            "package": self.package,
            "log_id": self.log_id,
            "description": self.description,
            "code": self.code,
            "failure_details": self.failure_details,
            "target": self.builder_result.kind,
            "target-details": self.builder_result.json(),
            "legacy_branch_name": self.legacy_branch_name,
            "logfilenames": self.logfilenames,
            "subworker": self.subworker_result,
            "value": self.value,
            "remotes": self.remotes,
            "branches": (
                [
                    (fn, n, br.decode("utf-8"), r.decode("utf-8"))
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
    return env


class WorkerResult(object):
    """The result from a worker."""

    def __init__(
        self,
        code,
        description,
        context=None,
        subworker=None,
        main_branch_revision=None,
        revision=None,
        value=None,
        changes_filename=None,
        build_distribution=None,
        build_version=None,
        branches=None,
        tags=None,
        remotes=None,
        details=None,
    ):
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
        self.branches = branches
        self.tags = tags
        self.remotes = remotes
        self.details = details

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
                (fn, n, br.encode("utf-8"), r.encode("utf-8"))
                for (fn, n, br, r) in branches
            ]
        if tags:
            tags = [(n, r.encode("utf-8")) for (fn, n, r) in tags]
        return cls(
            worker_result.get("code"),
            worker_result.get("description"),
            worker_result.get("context"),
            worker_result.get("subworker"),
            main_branch_revision,
            revision,
            worker_result.get("value"),
            worker_result.get("changes_filename"),
            worker_result.get("build_distribution"),
            worker_result.get("build_version"),
            branches,
            tags,
            worker_result.get("remotes"),
            worker_result.get("details"),
        )


async def run_subprocess(args, env, log_path=None):
    if log_path:
        read, write = os.pipe()
        p = await asyncio.create_subprocess_exec(
            *args, env=env, stdout=write, stderr=write, stdin=asyncio.subprocess.PIPE
        )
        p.stdin.close()
        os.close(write)
        tee = await asyncio.create_subprocess_exec("tee", log_path, stdin=read)
        os.close(read)
        await tee.wait()
        return await p.wait()
    else:
        p = await asyncio.create_subprocess_exec(
            *args, env=env, stdin=asyncio.subprocess.PIPE
        )
        p.stdin.close()
        return await p.wait()


async def invoke_subprocess_worker(
    worker_kind: str,
    main_branch_url: str,
    env: Dict[str, str],
    command: List[str],
    output_directory: str,
    resume: Optional["ResumeInfo"] = None,
    cached_branch_url: Optional[str] = None,
    pre_check: Optional[str] = None,
    post_check: Optional[str] = None,
    build_command: Optional[str] = None,
    log_path: Optional[str] = None,
    subpath: Optional[str] = None,
) -> int:
    subprocess_env = dict(os.environ.items())
    for k, v in env.items():
        if v is not None:
            subprocess_env[k] = v
    worker_module = {
        "local": "janitor.worker",
        "gcb": "janitor.gcb_worker",
    }[worker_kind.split(":")[0]]
    args = [
        sys.executable,
        "-m",
        worker_module,
        "--branch-url=%s" % main_branch_url,
        "--output-directory=%s" % output_directory,
    ]
    if ":" in worker_kind:
        args.append("--host=%s" % worker_kind.split(":")[1])
    if resume:
        args.append("--resume-branch-url=%s" % resume.resume_branch_url)
        resume_result_path = os.path.join(output_directory, "previous_result.json")
        with open(resume_result_path, "w") as f:
            json.dump(resume.result, f)
        args.append("--resume-result-path=%s" % resume_result_path)
        for (role, name, base, revision) in resume.resume_result_branches or []:
            if name is not None:
                args.append("--extra-resume-branch=%s:%s" % (role, name))
    if cached_branch_url:
        args.append("--cached-branch-url=%s" % cached_branch_url)
    if pre_check:
        args.append("--pre-check=%s" % pre_check)
    if post_check:
        args.append("--post-check=%s" % post_check)
    if build_command:
        args.append("--build-command=%s" % build_command)
    if subpath:
        args.append("--subpath=%s" % subpath)

    args.extend(command)
    return await run_subprocess(args, env=subprocess_env, log_path=log_path)


async def open_branch_with_fallback(
    conn, pkg, vcs_type, vcs_url, possible_transports=None
):
    probers = select_preferred_probers(vcs_type)
    try:
        return open_branch_ext(
            vcs_url, possible_transports=possible_transports, probers=probers
        )
    except BranchOpenFailure as e:
        if e.code == "hosted-on-alioth":
            logging.info(
                "Branch %s is hosted on alioth. Trying some other options..", vcs_url
            )
            try:
                branch = await open_guessed_salsa_branch(
                    conn,
                    pkg,
                    vcs_type,
                    vcs_url,
                    possible_transports=possible_transports,
                )
            except BranchOpenFailure:
                raise e
            else:
                if branch:
                    await state.update_branch_url(
                        conn, pkg, "Git", full_branch_url(branch).rstrip("/")
                    )
                    return branch
        raise


async def import_logs(
    output_directory: str,
    logfile_manager: LogFileManager,
    backup_logfile_manager: Optional[LogFileManager],
    pkg: str,
    log_id: str,
) -> List[str]:
    logfilenames = []
    for entry in os.scandir(output_directory):
        if entry.is_dir():
            continue
        parts = entry.name.split(".")
        if parts[-1] == "log" or (
            len(parts) == 3 and parts[-2] == "log" and parts[-1].isdigit()
        ):
            try:
                await logfile_manager.import_log(pkg, log_id, entry.path)
            except ServiceUnavailable as e:
                logging.warning("Unable to upload logfile %s: %s", entry.name, e)
                if backup_logfile_manager:
                    await backup_logfile_manager.import_log(pkg, log_id, entry.path)
            logfilenames.append(entry.name)
    return logfilenames


class ActiveRun(object):
    """Tracks state of an active run."""

    queue_item: state.QueueItem
    log_id: str
    start_time: datetime
    worker_name: str
    worker_link: Optional[str]

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

    def _extra_json(self):
        return {}

    def create_result(self, **kwargs):
        return JanitorResult(
            pkg=self.queue_item.package, log_id=self.log_id, **kwargs)

    def json(self) -> Any:
        """Return a JSON representation."""
        ret = {
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
            "logfilenames": list(self.list_log_files()),
        }
        ret.update(self._extra_json())
        return ret


class ActiveRemoteRun(ActiveRun):

    KEEPALIVE_INTERVAL = 60 * 10

    log_files: Dict[str, BinaryIO]
    websockets: Set[web.WebSocketResponse]

    def __init__(
        self,
        queue_item: state.QueueItem,
        worker_name: str,
        legacy_branch_name: str,
        jenkins_metadata: Optional[Dict[str, str]] = None,
    ):
        super(ActiveRemoteRun, self).__init__(queue_item)
        self.worker_name = worker_name
        self.log_files = {}
        self.main_branch_url = self.queue_item.branch_url
        self.resume_branch_name = None
        self.reset_keepalive()
        self.legacy_branch_name = legacy_branch_name
        self._watch_dog = None
        self._jenkins_metadata = jenkins_metadata

    def _extra_json(self):
        return {
            "jenkins": self._jenkins_metadata,
            "last-keepalive": self.last_keepalive.isoformat(
                timespec='seconds'),
            }

    @property
    def worker_link(self):
        if self._jenkins_metadata is not None:
            return self._jenkins_metadata["build_url"]
        return None

    def start_watchdog(self, queue_processor):
        if self._watch_dog is not None:
            raise Exception("Watchdog already started")
        self._watch_dog = asyncio.create_task(self.watchdog(queue_processor))

    def stop_watchdog(self):
        if self._watch_dog is None:
            return
        try:
            self._watch_dog.cancel()
        except asyncio.CancelledError:
            pass
        self._watch_dog = None

    def reset_keepalive(self):
        self.last_keepalive = datetime.now()

    def append_log(self, name, data):
        try:
            f = self.log_files[name]
        except KeyError:
            f = self.log_files[name] = BytesIO()
            ret = True
        else:
            ret = False
        f.write(data)
        return ret

    async def watchdog(self, queue_processor):
        while True:
            await asyncio.sleep(self.KEEPALIVE_INTERVAL)
            duration = datetime.now() - self.last_keepalive
            if duration > timedelta(seconds=(self.KEEPALIVE_INTERVAL * 2)):
                logging.warning(
                    "No keepalives received from %s for %s in %d, aborting.",
                    self.worker_name,
                    self.log_id,
                    duration.total_seconds(),
                )
                result = self.create_result(
                    branch_url=self.queue_item.branch_url,
                    description=("No keepalives received in %s." % duration),
                    code="worker-timeout",
                    logfilenames=[],
                )
                await queue_processor.finish_run(self, result)
                return

    def kill(self) -> None:
        raise NotImplementedError(self.kill)

    def list_log_files(self):
        return list(self.log_files.keys())

    def get_log_file(self, name):
        try:
            return BytesIO(self.log_files[name].getvalue())
        except KeyError:
            raise FileNotFoundError


async def open_canonical_main_branch(conn, queue_item, possible_transports=None):
    try:
        main_branch = await open_branch_with_fallback(
            conn,
            queue_item.package,
            queue_item.vcs_type,
            queue_item.branch_url,
            possible_transports=possible_transports,
        )
    except BranchOpenFailure as e:
        await state.update_branch_status(
            conn,
            queue_item.branch_url,
            None,
            status=e.code,
            description=e.description,
            revision=None,
        )
        raise
    else:
        branch_url = full_branch_url(main_branch)
        await state.update_branch_status(
            conn,
            queue_item.branch_url,
            branch_url,
            status="success",
            revision=main_branch.last_revision(),
        )
        return main_branch


async def open_resume_branch(main_branch, branch_name, possible_hosters=None):
    try:
        hoster = get_hoster(main_branch, possible_hosters=possible_hosters)
    except UnsupportedHoster as e:
        # We can't figure out what branch to resume from when there's
        # no hoster that can tell us.
        logging.warning("Unsupported hoster (%s)", e)
        return None
    else:
        try:
            (
                resume_branch,
                unused_overwrite,
                unused_existing_proposal,
            ) = find_existing_proposed(
                    main_branch, hoster, branch_name,
                    preferred_schemes=['https', 'git', 'bzr'])
        except NoSuchProject as e:
            logging.warning("Project %s not found", e.project)
            return None
        except PermissionDenied as e:
            logging.warning("Unable to list existing proposals: %s", e)
            return None
        else:
            return resume_branch


async def check_resume_result(conn: asyncpg.Connection, suite: str, resume_branch: Branch) -> Optional["ResumeInfo"]:
    if resume_branch is not None:
        (
            resume_branch_result,
            resume_branch_name,
            resume_review_status,
            resume_result_branches,
        ) = await state.get_run_result_by_revision(
            conn, suite, revision=resume_branch.last_revision()
        )
        if resume_review_status == "rejected":
            logging.info("Unsetting resume branch, since last run was " "rejected.")
            return None
        return ResumeInfo(
            resume_branch,
            resume_branch_result,
            resume_result_branches or [],
            legacy_branch_name=resume_branch_name,
        )
    else:
        return None


class ResumeInfo(object):
    def __init__(self, branch, result, resume_result_branches, legacy_branch_name):
        self.branch = branch
        self.result = result
        self.resume_result_branches = resume_result_branches
        self.legacy_branch_name = legacy_branch_name

    @property
    def resume_branch_url(self):
        return full_branch_url(self.branch)

    def json(self):
        return {
            "result": self.result,
            "branch_url": self.resume_branch_url,
            "branch_name": self.legacy_branch_name,
            "branches": [
                (fn, n, br.decode("utf-8"), r.decode("utf-8"))
                for (fn, n, br, r) in self.resume_result_branches
            ],
        }


def queue_item_env(queue_item):
    env = {}
    env["PACKAGE"] = queue_item.package
    if queue_item.upstream_branch_url:
        env["UPSTREAM_BRANCH_URL"] = queue_item.upstream_branch_url
    return env


class ActiveLocalRun(ActiveRun):
    def __init__(self, queue_item: state.QueueItem, output_directory: str):
        super(ActiveLocalRun, self).__init__(queue_item)
        self.worker_name = socket.gethostname()
        self.output_directory = output_directory

    worker_link = None

    def kill(self) -> None:
        self._task.cancel()

    def list_log_files(self):
        return [
            n
            for n in os.listdir(self.output_directory)
            if os.path.isfile(os.path.join(self.output_directory, n))
            and n.endswith(".log")
        ]

    def get_log_file(self, name):
        full_path = os.path.join(self.output_directory, name)
        return open(full_path, "rb")

    async def process(
        self,
        db: state.Database,
        config: Config,
        vcs_manager: VcsManager,
        logfile_manager: LogFileManager,
        backup_logfile_manager: Optional[LogFileManager],
        artifact_manager: Optional[ArtifactManager],
        worker_kind: str,
        build_command: Optional[str],
        apt_location: str,
        pre_check=None,
        post_check=None,
        dry_run: bool = False,
        possible_transports: Optional[List[Transport]] = None,
        possible_hosters: Optional[List[Hoster]] = None,
        use_cached_only: bool = False,
        overall_timeout: Optional[int] = None,
        committer: Optional[str] = None,
        backup_artifact_manager: Optional[ArtifactManager] = None,
    ) -> JanitorResult:
        logging.info(
            "Running %r on %s", self.queue_item.command, self.queue_item.package
        )

        if self.queue_item.branch_url is None:
            # TODO(jelmer): Try URLs in possible_salsa_urls_from_package_name
            return self.create_result(
                branch_url=self.queue_item.branch_url,
                description="No VCS URL known for package.",
                code="not-in-vcs",
                logfilenames=[],
            )

        env = {}
        env.update(queue_item_env(self.queue_item))
        if committer:
            env.update(committer_env(committer))

        try:
            suite_config = get_suite_config(config, self.queue_item.suite)
        except KeyError:
            return self.create_result(
                code="unknown-suite",
                description="Suite %s not in configuration" % self.queue_item.suite,
                logfilenames=[],
                branch_url=self.queue_item.branch_url,
            )

        # This is simple for now, since we only support one distribution..
        builder = DebianBuilder(config.distribution, apt_location)

        if not use_cached_only:
            async with db.acquire() as conn:
                try:
                    main_branch = await open_canonical_main_branch(
                        conn, self.queue_item, possible_transports=possible_transports
                    )
                except BranchOpenFailure as e:
                    return self.create_result(
                        branch_url=self.queue_item.branch_url,
                        description=e.description,
                        code=e.code,
                        logfilenames=[],
                    )

            try:
                resume_branch = await open_resume_branch(
                    main_branch,
                    suite_config.branch_name,
                    possible_hosters=possible_hosters,
                )
            except HosterLoginRequired as e:
                return self.create_result(
                    branch_url=self.queue_item.branch_url,
                    description=str(e),
                    code="hoster-login-required",
                    logfilenames=[],
                )

            if resume_branch is None:
                resume_branch = vcs_manager.get_branch(
                    self.queue_item.package,
                    suite_config.branch_name,
                    get_vcs_abbreviation(main_branch.repository),
                )

            if resume_branch is not None:
                logging.info(
                    "Resuming from %s", full_branch_url(resume_branch))

            cached_branch_url = vcs_manager.get_branch_url(
                self.queue_item.package,
                "master",
                get_vcs_abbreviation(main_branch.repository),
            )
        else:
            main_branch = vcs_manager.get_branch(self.queue_item.package, "master")
            if main_branch is None:
                return self.create_result(
                    branch_url=self.queue_item.branch_url,
                    code="cached-branch-missing",
                    description="Missing cache branch for %s" % self.queue_item.package,
                    logfilenames=[],
                )
            logging.info("Using cached branch %s", full_branch_url(main_branch))
            resume_branch = vcs_manager.get_branch(
                self.queue_item.package, suite_config.branch_name
            )
            cached_branch_url = None

        if self.queue_item.refresh and resume_branch:
            logging.info("Since refresh was requested, ignoring resume branch.")
            resume_branch = None

        async with db.acquire() as conn:
            resume = await check_resume_result(
                conn, self.queue_item.suite, resume_branch
            )

            env.update(
                await builder.build_env(conn, suite_config, self.queue_item)
            )

        log_path = os.path.join(self.output_directory, "worker.log")
        try:
            self._task = asyncio.create_task(
                asyncio.wait_for(
                    invoke_subprocess_worker(
                        worker_kind,
                        full_branch_url(main_branch).rstrip("/"),
                        env,
                        self.queue_item.command,
                        self.output_directory,
                        resume=resume,
                        cached_branch_url=cached_branch_url,
                        pre_check=pre_check,
                        post_check=post_check,
                        build_command=build_command,
                        log_path=log_path,
                        subpath=self.queue_item.subpath,
                    ),
                    timeout=overall_timeout,
                )
            )
            # set_name is only available on Python 3.8
            if getattr(self._task, "set_name", None):
                self._task.set_name(self.log_id)
            retcode = await self._task
        except asyncio.CancelledError:
            return self.create_result(
                branch_url=full_branch_url(main_branch),
                code="cancelled",
                description="Job cancelled",
                logfilenames=[],
            )
        except asyncio.TimeoutError:
            return self.create_result(
                branch_url=full_branch_url(main_branch),
                code="timeout",
                description="Run timed out after %d seconds"
                % overall_timeout,  # type: ignore
                logfilenames=[],
            )

        logfilenames = await import_logs(
            self.output_directory,
            logfile_manager,
            backup_logfile_manager,
            self.queue_item.package,
            self.log_id,
        )

        if retcode != 0:
            if retcode < 0:
                description = "Worker killed with signal %d" % abs(retcode)
                code = "worker-killed"
            else:
                code = "worker-failure"
                try:
                    with open(log_path, "r") as f:
                        description = list(f.readlines())[-1]
                except FileNotFoundError:
                    description = "Worker exited with return code %d" % retcode

            return self.create_result(
                branch_url=full_branch_url(main_branch),
                code=code,
                description=description,
                logfilenames=logfilenames,
            )

        json_result_path = os.path.join(self.output_directory, "result.json")
        if os.path.exists(json_result_path):
            worker_result = WorkerResult.from_file(json_result_path)
        else:
            worker_result = WorkerResult(
                "worker-missing-result",
                "Worker failed and did not write a result file.",
            )

        if worker_result.code is not None:
            return self.create_result(
                branch_url=full_branch_url(main_branch),
                worker_result=worker_result,
                logfilenames=logfilenames,
                legacy_branch_name=(
                    resume.legacy_branch_name
                    if resume and worker_result.code == "nothing-to-do"
                    else None
                ),
            )

        result = self.create_result(
            branch_url=full_branch_url(main_branch),
            code="success",
            worker_result=worker_result,
            logfilenames=logfilenames,
        )

        try:
            result.builder_result.from_directory(
                self.output_directory, self.queue_item.package
            )
        except NoChangesFile as e:
            # Oh, well.
            logging.info("No changes file found: %s", e)

        try:
            local_branch = open_branch(
                os.path.join(self.output_directory, self.queue_item.package)
            )
        except (BranchMissing, BranchUnavailable) as e:
            return self.create_result(
                branch_url=full_branch_url(main_branch),
                description="result branch unavailable: %s" % e,
                code="result-branch-unavailable",
                worker_result=worker_result,
                logfilenames=logfilenames,
            )

        enable_tag_pushing(local_branch)

        legacy_import_branches(
            vcs_manager,
            (main_branch, main_branch.last_revision()),
            (local_branch, local_branch.last_revision()),
            self.queue_item.package,
            suite_config.branch_name,
            additional_colocated_branches=(
                pick_additional_colocated_branches(main_branch)
            ),
        )
        import_branches(
            vcs_manager,
            local_branch,
            self.queue_item.package,
            suite_config.name,
            self.log_id,
            result.branches,
            result.tags,
        )
        result.legacy_branch_name = suite_config.branch_name

        if result.builder_result and artifact_manager:
            artifact_names = result.builder_result.artifact_filenames()
            await store_artifacts_with_backup(
                artifact_manager,
                backup_artifact_manager,
                self.output_directory,
                self.log_id,
                artifact_names,
            )

        return result


async def export_queue_length(db: state.Database) -> None:
    # TODO(jelmer): Move to a different process?
    while True:
        async with db.acquire() as conn:
            queue_duration.set((await state.queue_duration(conn)).total_seconds())
            async for bucket, tick, length in state.queue_stats(conn):
                current_tick.labels(bucket=bucket).set(tick)
                queue_length.labels(bucket=bucket).set(length)
        await asyncio.sleep(60)


async def export_stats(db: state.Database) -> None:
    # TODO(jelmer): Move to a different process?
    while True:
        async with db.acquire() as conn:
            for suite, count in await conn.fetch(
                """
select suite, count(distinct package) from run where result_code = 'success'
group by 1"""
            ):
                apt_package_count.labels(suite=suite).set(count)

            by_suite: Dict[str, int] = {}
            by_suite_result: Dict[Tuple[str, str], int] = {}
            async for package_name, suite, run_duration, result_code in (
                state.iter_by_suite_result_code(conn)
            ):
                by_suite.setdefault(suite, 0)
                by_suite[suite] += 1
                by_suite_result.setdefault((suite, result_code), 0)
                by_suite_result[(suite, result_code)] += 1
            for suite, count in by_suite.items():
                run_count.labels(suite=suite).set(count)
            for (suite, result_code), count in by_suite_result.items():
                run_result_count.labels(suite=suite, result_code=result_code).set(count)
            for suite, count in await state.get_never_processed_count(conn):
                never_processed_count.labels(suite).set(count)
            for review_status, count in await state.iter_review_status(conn):
                review_status_count.labels(review_status).set(count)

        # Every 30 minutes
        await asyncio.sleep(60 * 30)


class QueueProcessor(object):
    def __init__(
        self,
        database,
        config,
        worker_kind,
        build_command,
        pre_check=None,
        post_check=None,
        dry_run=False,
        logfile_manager=None,
        artifact_manager=None,
        vcs_manager=None,
        public_vcs_manager=None,
        concurrency=1,
        use_cached_only=False,
        overall_timeout=None,
        committer=None,
        apt_location=None,
        backup_artifact_manager=None,
        backup_logfile_manager=None,
    ):
        """Create a queue processor.

        Args:
          worker_kind: The kind of worker to run ('local', 'gcb')
          build_command: The command used to build packages
          pre_check: Function to run prior to modifying a package
          post_check: Function to run after modifying a package
        """
        self.database = database
        self.config = config
        self.worker_kind = worker_kind
        self.build_command = build_command
        self.pre_check = pre_check
        self.post_check = post_check
        self.dry_run = dry_run
        self.logfile_manager = logfile_manager
        self.artifact_manager = artifact_manager
        self.vcs_manager = vcs_manager
        self.public_vcs_manager = public_vcs_manager
        self.concurrency = concurrency
        self.use_cached_only = use_cached_only
        self.topic_queue = Topic("queue", repeat_last=True)
        self.topic_result = Topic("result")
        self.overall_timeout = overall_timeout
        self.committer = committer
        self.active_runs: Dict[str, ActiveRun] = {}
        self.apt_location = apt_location
        self.backup_artifact_manager = backup_artifact_manager
        self.backup_logfile_manager = backup_logfile_manager

    def status_json(self) -> Any:
        return {
            "processing": [
                active_run.json() for active_run in self.active_runs.values()
            ],
            "concurrency": self.concurrency,
        }

    async def process_queue_item(self, item: state.QueueItem) -> None:
        with tempfile.TemporaryDirectory() as output_directory:
            active_run = ActiveLocalRun(item, output_directory)
            self.register_run(active_run)
            result = await active_run.process(
                self.database,
                config=self.config,
                vcs_manager=self.vcs_manager,
                artifact_manager=self.artifact_manager,
                apt_location=self.apt_location,
                worker_kind=self.worker_kind,
                pre_check=self.pre_check,
                build_command=self.build_command,
                post_check=self.post_check,
                dry_run=self.dry_run,
                logfile_manager=self.logfile_manager,
                backup_logfile_manager=self.backup_logfile_manager,
                use_cached_only=self.use_cached_only,
                overall_timeout=self.overall_timeout,
                committer=self.committer,
                backup_artifact_manager=self.backup_artifact_manager,
            )
            await self.finish_run(active_run, result)

    def register_run(self, active_run: ActiveRun) -> None:
        self.active_runs[active_run.log_id] = active_run
        self.topic_queue.publish(self.status_json())
        packages_processed_count.inc()

    async def finish_run(self, active_run: ActiveRun, result: JanitorResult) -> None:
        finish_time = datetime.now()
        item = active_run.queue_item
        duration = finish_time - active_run.start_time
        build_duration.labels(package=item.package, suite=item.suite).observe(
            duration.total_seconds()
        )
        if result.code == "success" and item.suite != "unchanged":
            async with self.database.acquire() as conn:
                run = await state.get_unchanged_run(
                    conn, result.package, result.main_branch_revision
                )
                if run is None:
                    logging.info("Scheduling control run for %s.", item.package)
                    await do_schedule(
                        conn,
                        item.package,
                        "unchanged",
                        command=[
                            "just-build",
                            (
                                "--revision=%s"
                                % result.main_branch_revision.decode("utf-8")
                            ),
                        ],
                        bucket="control",
                        estimated_duration=duration,
                        requestor="control",
                    )
        if not self.dry_run:
            async with self.database.acquire() as conn:
                await state.store_run(
                    conn,
                    result.log_id,
                    item.package,
                    result.branch_url,
                    active_run.start_time,
                    finish_time,
                    item.command,
                    result.description,
                    item.context,
                    result.context,
                    result.main_branch_revision,
                    result.code,
                    build_version=result.builder_result.build_version,
                    build_distribution=result.builder_result.build_distribution,
                    branch_name=result.legacy_branch_name,
                    revision=result.revision,
                    subworker_result=result.subworker_result,
                    suite=item.suite,
                    logfilenames=result.logfilenames,
                    value=result.value,
                    worker_name=active_run.worker_name,
                    worker_link=active_run.worker_link,
                    result_branches=result.branches,
                    result_tags=result.tags,
                    failure_details=result.failure_details,
                )
                await result.builder_result.store(conn, result.log_id, item.package)
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
        todo = set(
            [
                self.process_queue_item(item)
                for item in await self.next_queue_item(self.concurrency)
            ]
        )

        def handle_sigterm():
            self.concurrency = None
            logging.info("Received SIGTERM; not starting new jobs.")

        loop = asyncio.get_event_loop()
        loop.add_signal_handler(signal.SIGTERM, handle_sigterm)
        try:
            while True:
                if not todo:
                    if self.concurrency is None:
                        break
                    logging.info("Nothing to do. Sleeping for 60s.")
                    await asyncio.sleep(60)
                    done: Set[asyncio.Future[None]] = set()
                else:
                    done, pending = await asyncio.wait(
                        todo, return_when="FIRST_COMPLETED"
                    )
                    for task in done:
                        task.result()
                    todo = pending  # type: ignore
                if self.concurrency:
                    todo.update(
                        [
                            self.process_queue_item(item)
                            for item in await self.next_queue_item(len(done))
                        ]
                    )
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
    run_id = request.match_info["run_id"]
    try:
        active_run = queue_processor.active_runs[run_id]
    except KeyError:
        return web.Response(text="No such current run: %s" % run_id, status=404)
    log_filenames = active_run.list_log_files()
    return web.json_response(log_filenames)


async def handle_kill(request):
    queue_processor = request.app.queue_processor
    run_id = request.match_info["run_id"]
    try:
        ret = queue_processor.active_runs[run_id].json()
        queue_processor.active_runs[run_id].kill()
    except KeyError:
        return web.Response(text="No such current run: %s" % run_id, status=404)
    return web.json_response(ret)


async def handle_progress_ws(request):
    queue_processor = request.app.queue_processor
    ws = web.WebSocketResponse()
    await ws.prepare(request)

    # Messages on the progress bus:
    # b'log\0<run-id>\0<logfilename>\0<logbytes>'
    # b'keepalive\0<run-id>'

    async for msg in ws:
        if msg.type == WSMsgType.BINARY:
            (run_id_bytes, rest) = msg.data.split(b"\0", 1)
            run_id = run_id_bytes.decode("utf-8")
            try:
                active_run = queue_processor.active_runs[run_id]
            except KeyError:
                logging.warning("No such current run: %s" % run_id)
                continue
            if rest.startswith(b"log\0"):
                (unused_kind, logname, data) = rest.split(b"\0", 2)
                if active_run.append_log(logname.decode("utf-8"), data):
                    # Make sure everybody is aware of the new log file.
                    queue_processor.topic_queue.publish(queue_processor.status_json())
                active_run.reset_keepalive()
            elif rest == b"keepalive":
                active_run.reset_keepalive()
            else:
                logging.warning("Unknown progress message %r for %s", rest, run_id)

    return ws


async def handle_log(request):
    queue_processor = request.app.queue_processor
    run_id = request.match_info["run_id"]
    filename = request.match_info["filename"]
    if "/" in filename:
        return web.Response(text="Invalid filename %s" % filename, status=400)
    try:
        active_run = queue_processor.active_runs[run_id]
    except KeyError:
        return web.Response(text="No such current run: %s" % run_id, status=404)
    try:
        f = active_run.get_log_file(filename)
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


async def handle_assign(request):
    json = await request.json()
    worker = json["worker"]

    possible_transports = []
    possible_hosters = []

    async def abort(active_run, code, description):
        result = active_run.create_result(
            branch_url=active_run.main_branch_url,
            code=code,
            description=description,
        )
        async with queue_processor.database.acquire() as conn:
            await queue_processor.finish_run(active_run, result)
            await state.drop_queue_item(conn, active_run.queue_item.id)

    queue_processor = request.app.queue_processor
    [item] = await queue_processor.next_queue_item(1)

    suite_config = get_suite_config(queue_processor.config, item.suite)

    active_run = ActiveRemoteRun(
        worker_name=worker,
        queue_item=item,
        legacy_branch_name=suite_config.branch_name,
        jenkins_metadata=json.get("jenkins"),
    )

    queue_processor.register_run(active_run)

    # This is simple for now, since we only support one distribution.
    builder = DebianBuilder(
        queue_processor.config.distribution,
        queue_processor.apt_location
        )

    async with queue_processor.database.acquire() as conn:
        build_env = await builder.build_env(conn, suite_config, item)

        try:
            main_branch = await open_canonical_main_branch(
                conn, item, possible_transports=possible_transports
            )
        except BranchOpenFailure:
            resume_branch = None
            vcs_type = item.vcs_type
        else:
            active_run.main_branch_url = full_branch_url(main_branch).rstrip('/')
            vcs_type = get_vcs_abbreviation(main_branch.repository)
            if not item.refresh:
                resume_branch = await open_resume_branch(
                    main_branch,
                    suite_config.branch_name,
                    possible_hosters=possible_hosters,
                )
            else:
                resume_branch = None

        if vcs_type is not None:
            vcs_type = vcs_type.lower()

        if resume_branch is None and not item.refresh:
            resume_branch = queue_processor.public_vcs_manager.get_branch(
                item.package, suite_config.branch_name, vcs_type
            )

        resume = await check_resume_result(conn, item.suite, resume_branch)
        if resume is not None:
            if is_authenticated_url(resume.branch.user_url):
                raise AssertionError('invalid resume branch %r' % (
                    resume.branch))

    try:
        cached_branch_url = queue_processor.public_vcs_manager.get_branch_url(
            item.package, "master", vcs_type
        )
    except UnsupportedVcs:
        cached_branch_url = None

    env = {}
    env.update(queue_item_env(item))
    if queue_processor.committer:
        env.update(committer_env(queue_processor.committer))

    assignment = {
        "id": active_run.log_id,
        "description": "%s on %s" % (item.suite, item.package),
        "queue_id": item.id,
        "branch": {
            "url": active_run.main_branch_url,
            "subpath": item.subpath,
            "vcs_type": item.vcs_type,
            "cached_url": cached_branch_url,
        },
        "resume": resume.json() if resume else None,
        "build": {"environment": build_env},
        "env": env,
        "command": item.command,
        "suite": item.suite,
        "legacy_branch_name": active_run.legacy_branch_name,
        "vcs_manager": queue_processor.public_vcs_manager.base_url,
    }

    active_run.start_watchdog(queue_processor)
    return web.json_response(assignment, status=201)


async def handle_finish(request):
    queue_processor = request.app.queue_processor
    run_id = request.match_info["run_id"]
    try:
        active_run = queue_processor.active_runs[run_id]
    except KeyError:
        return web.json_response(
            {"reason": "No such current run: %s" % run_id}, status=404
        )

    active_run.stop_watchdog()

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

        logfilenames = await import_logs(
            output_directory,
            queue_processor.logfile_manager,
            queue_processor.backup_logfile_manager,
            active_run.queue_item.package,
            run_id,
        )

        if worker_result.code is not None:
            result = active_run.create_result(
                branch_url=active_run.main_branch_url,
                worker_result=worker_result,
                logfilenames=logfilenames,
                legacy_branch_name=(
                    active_run.resume_branch_name
                    if worker_result.code == "nothing-to-do"
                    else None
                ),
            )
        else:
            result = active_run.create_result(
                branch_url=active_run.main_branch_url,
                code="success",
                worker_result=worker_result,
                logfilenames=logfilenames,
                legacy_branch_name=active_run.legacy_branch_name,
            )

            try:
                result.builder_result.from_directory(
                    output_directory, active_run.queue_item.package
                )
            except NoChangesFile as e:
                # Oh, well.
                logging.info("No changes file found: %s", e)

            artifact_names = result.builder_result.artifact_filenames()
            await store_artifacts_with_backup(
                queue_processor.artifact_manager,
                queue_processor.backup_artifact_manager,
                output_directory,
                run_id,
                artifact_names,
            )

    await queue_processor.finish_run(active_run, result)
    return web.json_response(
        {"id": active_run.log_id, "filenames": filenames, "result": result.json()},
        status=201,
    )


async def run_web_server(listen_addr, port, queue_processor):
    app = web.Application()
    app.queue_processor = queue_processor
    setup_metrics(app)
    app.router.add_get("/status", handle_status)
    app.router.add_get("/log/{run_id}", handle_log_index)
    app.router.add_get("/log/{run_id}/{filename}", handle_log)
    app.router.add_post("/kill/{run_id}", handle_kill)
    app.router.add_get("/ws/progress", handle_progress_ws)
    app.router.add_get(
        "/ws/queue", functools.partial(pubsub_handler, queue_processor.topic_queue)
    )
    app.router.add_get(
        "/ws/result", functools.partial(pubsub_handler, queue_processor.topic_result)
    )
    app.router.add_post("/assign", handle_assign)
    app.router.add_post("/finish/{run_id}", handle_finish)
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()


def main(argv=None):
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
        "--build-command", help="Build package to verify it.", type=str, default=None
    )
    parser.add_argument(
        "--dry-run",
        help="Create branches but don't push or propose anything.",
        action="store_true",
        default=False,
    )
    parser.add_argument(
        "--worker",
        type=str,
        default="local",
        choices=["local", "gcb"],
        help="Worker to use.",
    )
    parser.add_argument(
        "--concurrency",
        type=int,
        default=1,
        help="Number of workers to run in parallel.",
    )
    parser.add_argument(
        "--use-cached-only", action="store_true", help="Use cached branches only."
    )
    parser.add_argument(
        "--config", type=str, default="janitor.conf", help="Path to configuration."
    )
    parser.add_argument(
        "--overall-timeout",
        type=int,
        default=None,
        help="Overall timeout per run (in seconds).",
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
        "--public-vcs-location", type=str, default="https://janitor.debian.net/"
    )

    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO)

    debug.set_debug_flags_from_config()

    with open(args.config, "r") as f:
        config = read_config(f)

    state.DEFAULT_URL = config.database_location
    public_vcs_manager = RemoteVcsManager(args.public_vcs_location)
    if config.vcs_location:
        vcs_manager = LocalVcsManager(config.vcs_location)
    else:
        vcs_manager = public_vcs_manager
    logfile_manager = get_log_manager(config.logs_location)
    artifact_manager = get_artifact_manager(config.artifact_location)

    loop = asyncio.get_event_loop()
    if args.backup_directory:
        backup_logfile_directory = os.path.join(args.backup_directory, "logs")
        backup_artifact_directory = os.path.join(args.backup_directory, "artifacts")
        if not os.path.isdir(backup_logfile_directory):
            os.mkdir(backup_logfile_directory)
        if not os.path.isdir(backup_artifact_directory):
            os.mkdir(backup_artifact_directory)
        backup_artifact_manager = LocalArtifactManager(backup_artifact_directory)
        backup_logfile_manager = FileSystemLogFileManager(backup_logfile_directory)
        loop.run_until_complete(
            upload_backup_artifacts(
                backup_artifact_manager, artifact_manager, timeout=60 * 15
            )
        )
    else:
        backup_artifact_manager = None
        backup_logfile_manager = None
    db = state.Database(config.database_location)
    queue_processor = QueueProcessor(
        db,
        config,
        args.worker,
        args.build_command,
        args.pre_check,
        args.post_check,
        args.dry_run,
        logfile_manager,
        artifact_manager,
        vcs_manager,
        public_vcs_manager,
        args.concurrency,
        args.use_cached_only,
        overall_timeout=args.overall_timeout,
        committer=config.committer,
        apt_location=config.apt_location,
        backup_artifact_manager=backup_artifact_manager,
        backup_logfile_manager=backup_logfile_manager,
    )

    async def run():
        async with artifact_manager:
            return await asyncio.gather(
                loop.create_task(queue_processor.process()),
                loop.create_task(export_queue_length(db)),
                loop.create_task(export_stats(db)),
                loop.create_task(
                    run_web_server(args.listen_address, args.port, queue_processor)
                ),
            )

    loop.run_until_complete(run())


if __name__ == "__main__":
    sys.exit(main(sys.argv))
