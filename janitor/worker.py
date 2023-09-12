#!/usr/bin/python3
# Copyright (C) 2018-2022 Jelmer Vernooij <jelmer@jelmer.uk>
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
import errno
import json
import logging
import os
import shlex
import signal
import socket
import ssl
import subprocess
import sys
import traceback
import warnings
from contextlib import ExitStack, contextmanager
from datetime import datetime
from functools import partial
from http.client import IncompleteRead
from tempfile import TemporaryDirectory
from typing import Any, Optional, cast

import backoff
import yarl
from aiohttp import (
    BasicAuth,
    web,
)
from aiohttp_openmetrics import REGISTRY, Counter, push_to_gateway, setup_metrics
from breezy import urlutils
from breezy.branch import Branch
from breezy.config import (
    GlobalStack,
    PlainTextCredentialStore,
    credential_store_registry,
)
from breezy.controldir import ControlDir, format_registry
from breezy.errors import (
    AlreadyControlDirError,
    InvalidHttpResponse,
    NoRepositoryPresent,
    NotBranchError,
    TransportError,
    TransportNotPossible,
    UnexpectedHttpStatus,
)

try:
    from breezy.errors import ConnectionError  # type: ignore
except ImportError:  # breezy >= 4
    pass
from breezy.git.remote import RemoteGitError
from breezy.revision import NULL_REVISION
from breezy.transform import ImmortalLimbo, MalformedTransform, TransformRenameFailed
from breezy.transport import NoSuchFile, Transport, get_transport
from breezy.tree import MissingNestedTree
from breezy.workingtree import WorkingTree
from jinja2 import Template
from silver_platter.apply import CommandResult as GenericCommandResult
from silver_platter.apply import DetailedFailure as GenericDetailedFailure
from silver_platter.apply import (
    ResultFileFormatError,
    ScriptFailed,
    ScriptMadeNoChanges,
    ScriptNotFound,
)
from silver_platter.apply import script_runner as generic_script_runner
from silver_platter.probers import select_probers
from silver_platter.utils import (
    BranchMissing,
    BranchTemporarilyUnavailable,
    BranchUnavailable,
    full_branch_url,
    get_branch_vcs_type,
    open_branch,
)
from silver_platter.workspace import Workspace

from ._worker import (
    AssignmentFailure,
    Client,
    EmptyQueue,
    Metadata,
    ResultUploadFailure,
    abort_run,
    gce_external_ip,
    is_gce_instance,
)
from ._worker import (
    WorkerFailure as _WorkerFailure,
)
from .vcs import BranchOpenFailure, open_branch_ext


def WorkerFailure(code: str, description: str, details: Any | None = None, stage: tuple[str, ...] | None = None, transient: bool | None = None):
    return _WorkerFailure(code, description, details, stage, transient)


push_branch_retries = Counter(
    "push_branch_retries", "Number of branch push retries.")
upload_result_retries = Counter(
    "upload_result_retries", "Number of result upload retries.")
assignment_failed_count = Counter(
    "assignment_failed_count", "Failed to obtain assignment")


routes = web.RouteTableDef()
USER_AGENT = "janitor/worker (0.1)"


logger = logging.getLogger(__name__)


def _convert_codemod_script_failed(e: ScriptFailed) -> _WorkerFailure:
    if e.args[1] == 127:
        return WorkerFailure(
            'command-not-found',
            f'Command {e.args[0]} not found',
            stage=("codemod", ))
    elif e.args[1] == 137:
        return WorkerFailure(
            'killed',
            'Process was killed (by OOM killer?)',
            stage=("codemod", ))
    return WorkerFailure(
        'command-failed',
        'Script {} failed to run with code {}'.format(*e.args),
        stage=("codemod", ))


class Target:
    """A build target."""

    name: str

    def build(self, local_tree: WorkingTree, subpath, output_directory, config):
        raise NotImplementedError(self.build)

    def validate(self, local_tree: WorkingTree, subpath: str, config):
        pass

    def make_changes(self, local_tree: WorkingTree, subpath: str, argv, *, log_directory,
                     resume_metadata=None):
        raise NotImplementedError(self.make_changes)


class DebianTarget(Target):
    """Debian target."""

    name = "debian"

    def __init__(self, env) -> None:
        self.env = env
        self.committer = env.get("COMMITTER")
        uc = env.get("DEB_UPDATE_CHANGELOG", "auto")
        if uc == "auto":
            self.update_changelog = None
        elif uc == "update":
            self.update_changelog = True
        elif uc == "leave":
            self.update_changelog = True
        else:
            logging.warning(
                'Invalid value for DEB_UPDATE_CHANGELOG: %s, '
                'defaulting to auto.', uc)
            self.update_changelog = None

    def make_changes(self, local_tree, subpath, argv, *, log_directory,
                     resume_metadata=None):
        from silver_platter.debian.apply import DetailedFailure as DebianDetailedFailure
        from silver_platter.debian.apply import MissingChangelog
        from silver_platter.debian.apply import script_runner as debian_script_runner

        if not argv:
            return GenericCommandResult(
                description='No change build', context={}, tags=[], value=0)

        logging.info('Running %r', argv)
        # TODO(jelmer): This is only necessary for deb-new-upstream
        dist_command = 'PYTHONPATH={} {} -m janitor.debian.dist --log-directory={} '.format(
            ':'.join(sys.path), sys.executable, log_directory)

        try:
            dist_command = "SCHROOT={} {}".format(self.env["CHROOT"], dist_command)
        except KeyError:
            pass

        if local_tree.has_filename(os.path.join(subpath, 'debian')):
            dist_command += ' --packaging=%s' % local_tree.abspath(
                os.path.join(subpath, 'debian'))

        # Prevent 404s because files have gone away:
        dist_command += ' --apt-update --apt-dist-upgrade'

        extra_env = {'DIST': dist_command}
        extra_env.update(self.env)
        try:
            with open(os.path.join(log_directory, "codemod.log"), 'wb') as f:
                return debian_script_runner(
                    local_tree, script=argv, commit_pending=None,
                    resume_metadata=resume_metadata, subpath=subpath,
                    update_changelog=self.update_changelog,
                    extra_env=extra_env, committer=self.committer,
                    stderr=f)
        except ResultFileFormatError as e:
            raise WorkerFailure(
                'result-file-format', 'Result file was invalid: %s' % e,
                transient=False,
                stage=("codemod", )) from e
        except ScriptMadeNoChanges as e:
            raise WorkerFailure(
                'nothing-to-do', 'No changes made',
                transient=False,
                stage=("codemod", )) from e
        except MissingChangelog as e:
            raise WorkerFailure(
                'missing-changelog', 'No changelog present: %s' % e.args[0],
                transient=False,
                stage=("codemod", )) from e
        except DebianDetailedFailure as e:
            stage = ("codemod", ) + (e.stage if e.stage else ())
            raise WorkerFailure(
                e.result_code, e.description, e.details, stage=stage) from e
        except ScriptNotFound as e:
            raise WorkerFailure(
                "codemod-not-found",
                "Codemod script %r not found" % argv) from e
        except ScriptFailed as e:
            raise _convert_codemod_script_failed(e) from e
        except MemoryError as e:
            raise WorkerFailure(
                'out-of-memory', str(e), stage=("codemod", )) from e

    def build(self, local_tree, subpath, output_directory, config):
        from janitor.debian.build import BuildFailure, build_from_config
        try:
            return build_from_config(
                local_tree, subpath, output_directory, config, self.env)
        except BuildFailure as e:
            raise WorkerFailure(
                e.code, e.description,
                stage=((("build", ) + (e.stage, )) if e.stage else ()),
                details=e.details) from e

    def validate(self, local_tree, subpath, config):
        from .debian.validate import ValidateError, validate_from_config
        try:
            return validate_from_config(local_tree, subpath, config)
        except ValidateError as e:
            raise WorkerFailure(
                e.code, e.description,
                transient=False,
                stage=("validate", )) from e


class GenericTarget(Target):
    """Generic build target."""

    name = "generic"

    def __init__(self, env) -> None:
        self.env = env

    def make_changes(self, local_tree, subpath, argv, *, log_directory,
                     resume_metadata=None):
        if not argv:
            return GenericCommandResult(
                description='No change build', context={}, tags=[], value=0)

        logging.info('Running %r', argv)
        try:
            with open(os.path.join(log_directory, "codemod.log"), 'wb') as f:
                return generic_script_runner(
                    local_tree, script=argv, commit_pending=None,
                    resume_metadata=resume_metadata, subpath=subpath,
                    committer=self.env.get('COMMITTER'), extra_env=self.env,
                    stderr=f)
        except ResultFileFormatError as e:
            raise WorkerFailure(
                'result-file-format', 'Result file was invalid: %s' % e,
                transient=False,
                stage=("codemod", )) from e
        except ScriptMadeNoChanges as e:
            raise WorkerFailure(
                'nothing-to-do', 'No changes made', stage=("codemod", ),
                transient=False) from e
        except GenericDetailedFailure as e:
            stage = ("codemod", ) + (e.stage if e.stage else ())
            raise WorkerFailure(
                e.result_code, e.description, e.details, stage=stage) from e
        except ScriptNotFound as e:
            raise WorkerFailure(
                "codemod-not-found",
                "Codemod script %r not found" % argv) from e
        except ScriptFailed as e:
            raise _convert_codemod_script_failed(e) from e

    def build(self, local_tree, subpath, output_directory, config):
        from janitor.generic.build import BuildFailure, build_from_config
        try:
            return build_from_config(
                local_tree, subpath, output_directory, config, self.env)
        except BuildFailure as e:
            raise WorkerFailure(
                e.code, e.description,
                stage=((("build", ) + (e.stage, )) if e.stage else ()),
                details=e.details) from e


@backoff.on_exception(
    backoff.expo,
    (InvalidHttpResponse, IncompleteRead, ConnectionError),
    max_tries=10,
    on_backoff=lambda m: push_branch_retries.inc())
def import_branches_git(
        repo_url, local_branch: Branch, campaign: str, log_id: str,
        branches: Optional[list[tuple[str, Optional[str], Optional[bytes], Optional[bytes]]]],
        tags: Optional[list[tuple[str, bytes]]],
        update_current: bool = True):
    from breezy.git.dir import BareLocalGitControlDirFormat
    from breezy.git.repository import GitRepository
    from breezy.repository import InterRepository
    from dulwich.objects import ZERO_SHA

    try:
        vcs_result_controldir = ControlDir.open(repo_url)
    except NotBranchError:
        transport = get_transport(repo_url)
        if not transport.has('.'):
            try:
                transport.ensure_base()
            except NoSuchFile:
                transport.create_prefix()
        # The server is expected to have repositories ready for us, unless
        # we're working locally.
        format = BareLocalGitControlDirFormat()
        vcs_result_controldir = format.initialize(repo_url)

    repo = cast("GitRepository", vcs_result_controldir.open_repository())

    def get_changed_refs(refs):
        changed_refs: dict[bytes, tuple[bytes, Optional[bytes]]] = {}
        for (fn, _n, _br, r) in (branches or []):
            tagname = f"refs/tags/run/{log_id}/{fn}".encode()
            if r is None:
                changed_refs[tagname] = (ZERO_SHA, r)
            else:
                changed_refs[tagname] = (repo.lookup_bzr_revision_id(r)[0], r)
            if update_current:
                branchname = f"refs/heads/{campaign}/{fn}".encode()
                # TODO(jelmer): Ideally this would be a symref:
                changed_refs[branchname] = changed_refs[tagname]
        for n, r in (tags or []):
            tagname = f"refs/tags/{log_id}/{n}".encode()
            changed_refs[tagname] = (repo.lookup_bzr_revision_id(r)[0], r)
            if update_current:
                tagname = f"refs/tags/{n}".encode()
                changed_refs[tagname] = (repo.lookup_bzr_revision_id(r)[0], r)
        return changed_refs

    inter = InterRepository.get(local_branch.repository, repo)
    inter.fetch_refs(get_changed_refs, lossy=False, overwrite=True)


@backoff.on_exception(
    backoff.expo,
    (InvalidHttpResponse, IncompleteRead, ConnectionError),
    max_tries=10,
    on_backoff=lambda m: push_branch_retries.inc())
def import_branches_bzr(
        repo_url: str, local_branch, campaign: str, log_id: str, branches,
        tags: Optional[list[tuple[str, bytes]]], update_current: bool = True
):
    format = format_registry.make_controldir('bzr')
    for fn, _n, _br, r in branches:
        try:
            rootcd = ControlDir.open(repo_url)
        except NotBranchError:
            rootcd = ControlDir.create(repo_url)
        try:
            rootcd.find_repository()
        except NoRepositoryPresent:
            rootcd.create_repository(shared=True)
        transport = rootcd.user_transport.clone(campaign)
        name = (fn if fn != 'main' else '')
        if not transport.has('.'):
            try:
                transport.ensure_base()
            except NoSuchFile:
                transport.create_prefix()
        try:
            branchcd = format.initialize_on_transport(transport)
        except AlreadyControlDirError:
            branchcd = ControlDir.open_from_transport(transport)

        try:
            target_branch = branchcd.open_branch(name=name)
        except NotBranchError:
            target_branch = branchcd.create_branch(name=name)
        if update_current:
            local_branch.push(target_branch, overwrite=True, stop_revision=r)
        else:
            target_branch.repository.fetch(local_branch.repository, revision_id=r)

        target_branch.tags.set_tag(log_id, r)

        graph = target_branch.repository.get_graph()
        for name, revision in (tags or []):
            # Only set tags on those branches where the revisions exist
            if graph.is_ancestor(revision, target_branch.last_revision()):
                target_branch.tags.set_tag(f'{log_id}/{name}', revision)
                if update_current:
                    target_branch.tags.set_tag(name, revision)


def handle_sigterm(client: Client, workitem, signum):
    logging.warning('Received signal %d, aborting and exiting...', signum)

    async def shutdown():
        if workitem:
            await abort_run(
                client, workitem['assignment']['id'], workitem['metadata'], "Killed by signal")
        sys.exit(1)
    loop = asyncio.get_event_loop()
    loop.create_task(shutdown())


@contextmanager
def copy_output(output_log: str, tee: bool = False):
    old_stdout = os.dup(sys.stdout.fileno())
    old_stderr = os.dup(sys.stderr.fileno())
    if tee:
        p = subprocess.Popen(["tee", output_log], stdin=subprocess.PIPE)
        newfd = p.stdin
    else:
        newfd = open(output_log, 'wb')
    os.dup2(newfd.fileno(), sys.stdout.fileno())  # type: ignore
    os.dup2(newfd.fileno(), sys.stderr.fileno())  # type: ignore
    try:
        yield
    finally:
        sys.stdout.flush()
        sys.stderr.flush()
        os.dup2(old_stdout, sys.stdout.fileno())
        os.dup2(old_stderr, sys.stderr.fileno())
        if newfd is not None:
            newfd.close()


@backoff.on_exception(
    backoff.expo,
    (IncompleteRead, UnexpectedHttpStatus, InvalidHttpResponse,
     ConnectionError, ssl.SSLEOFError),
    max_tries=10,
    on_backoff=lambda m: push_branch_retries.inc())
def push_branch(
    source_branch: Branch,
    url: str,
    vcs_type: Optional[str],
    overwrite=False,
    stop_revision=None,
    tag_selector=None,
    possible_transports: Optional[list[Transport]] = None,
) -> None:
    url, params = urlutils.split_segment_parameters(url)
    branch_name = params.get("branch")
    if branch_name is not None:
        branch_name = urlutils.unquote(branch_name)
    if vcs_type is None:
        vcs_type = source_branch.controldir.cloning_metadir()
    try:
        target = ControlDir.open(url, possible_transports=possible_transports)
    except NotBranchError:
        target = ControlDir.create(
            url, format=vcs_type, possible_transports=possible_transports
        )

    target.push_branch(
        source_branch, revision_id=stop_revision, overwrite=overwrite, name=branch_name,
        tag_selector=tag_selector
    )


def _push_error_to_worker_failure(e, stage):
    if isinstance(e, UnexpectedHttpStatus):
        if e.code == 502:
            return WorkerFailure(
                "bad-gateway",
                "Failed to push result branch: %s" % e,
                stage=stage,
                transient=True
            )
        return WorkerFailure(
            "push-failed", "Failed to push result branch: %s" % e,
            stage=stage)
    if isinstance(e, ConnectionError):
        if "Temporary failure in name resolution" in e.msg:
            return WorkerFailure(
                "failed-temporarily",
                "Failed to push result branch: %s" % e,
                stage=stage, transient=True)
        return WorkerFailure(
            "push-failed", "Failed to push result branch: %s" % e,
            stage=stage)

    if isinstance(
            e, (InvalidHttpResponse, IncompleteRead,
                ConnectionError, ssl.SSLEOFError,
                ssl.SSLError)):
        return WorkerFailure(
            "push-failed", "Failed to push result branch: %s" % e,
            stage=stage)
    if isinstance(e, RemoteGitError):
        if str(e) == 'missing necessary objects':
            return WorkerFailure(
                'git-missing-necessary-objects', str(e),
                stage=stage)
        elif str(e) == 'failed to updated ref':
            return WorkerFailure(
                'git-ref-update-failed',
                str(e), stage=stage)
        else:
            return WorkerFailure(
                "git-error", str(e), stage=stage)
    return e


def run_worker(
    *,
    codebase: str,
    campaign: str,
    main_branch_url: Optional[str],
    run_id: str,
    build_config: Any,
    env: dict[str, str],
    command: list[str],
    output_directory: str,
    metadata: Metadata,
    target_repo_url: str,
    vendor: str,
    target: str,
    vcs_type: Optional[str] = None,
    subpath: str = '',
    resume_branch_url: Optional[str] = None,
    cached_branch_url: Optional[str] = None,
    resume_codemod_result=None,
    resume_branches: Optional[
        list[tuple[str, str, Optional[bytes], Optional[bytes]]]] = None,
    possible_transports: Optional[list[Transport]] = None,
    force_build: bool = False,
    tee: bool = False,
    additional_colocated_branches: Optional[dict[str, str]] = None,
    skip_setup_validation: bool = False,
    default_empty: bool = False,
):

    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    with ExitStack() as es:
        es.enter_context(copy_output(os.path.join(output_directory, "worker.log"), tee=tee))
        try:
            metadata.command = command
            metadata.codebase = codebase

            build_target: Target
            if target == "debian":
                build_target = DebianTarget(env)
            elif target == "generic":
                build_target = GenericTarget(env)
            else:
                raise WorkerFailure(
                    'target-unsupported', 'The target %r is not supported' % target,
                    transient=False, stage=("setup", ))

            logger.info("Opening branch at %s", main_branch_url)
            if main_branch_url:
                try:
                    main_branch = open_branch_ext(main_branch_url, possible_transports=possible_transports)
                except BranchOpenFailure as e:
                    raise WorkerFailure(e.code, e.description, stage=("setup", ), details={
                        'url': main_branch_url,
                        'retry_after': e.retry_after,
                    }, transient=('temporarily' in e.code)) from e
                metadata.branch_url = main_branch.user_url
                metadata.vcs_type = get_branch_vcs_type(main_branch)
                metadata.subpath = subpath
                empty_format = None
            else:
                assert vcs_type is not None
                main_branch = None
                metadata.branch_url = None
                metadata.vcs_type = vcs_type
                metadata.subpath = ""
                try:
                    empty_format = format_registry.make_controldir(vcs_type)
                except KeyError as e:
                    raise WorkerFailure(
                        "vcs-type-unsupported", f"Unable to find format for vcs type {vcs_type}",
                        stage=("setup", ),
                        transient=False,
                        details={'vcs_type': vcs_type}) from e

            if cached_branch_url:
                try:
                    cached_branch = open_branch(
                        cached_branch_url, possible_transports=possible_transports,
                        probers=select_probers(vcs_type))
                except BranchMissing as e:
                    logger.info("Cached branch URL %s missing: %s", cached_branch_url, e)
                    cached_branch = None
                except BranchUnavailable as e:
                    logger.warning(
                        "Cached branch URL %s unavailable: %s", cached_branch_url, e
                    )
                    cached_branch = None
                else:
                    if cached_branch is not None:
                        logger.info("Using cached branch %s", full_branch_url(cached_branch))
            else:
                cached_branch = None

            if resume_branch_url:
                logger.info('Using resume branch: %s', resume_branch_url)
                try:
                    resume_branch = open_branch(
                        resume_branch_url, possible_transports=possible_transports,
                        probers=select_probers(vcs_type)
                    )
                except BranchTemporarilyUnavailable as e:
                    logger.info('Resume branch URL %s temporarily unavailable: %s', e.url, e)
                    traceback.print_exc()
                    raise WorkerFailure(
                        "worker-resume-branch-temporarily-unavailable", str(e),
                        stage=("setup", ),
                        transient=True,
                        details={'url': e.url}) from e
                except BranchUnavailable as e:
                    logger.info('Resume branch URL %s unavailable: %s', e.url, e)
                    traceback.print_exc()
                    raise WorkerFailure(
                        "worker-resume-branch-unavailable", str(e),
                        stage=("setup", ),
                        transient=False,
                        details={'url': e.url}) from e
                except BranchMissing as e:
                    raise WorkerFailure(
                        "worker-resume-branch-missing", str(e),
                        stage=("setup", ),
                        transient=False,
                        details={'url': e.url}) from e
            else:
                resume_branch = None

            roles: dict[Optional[str], str] = {b: r for (r, b) in (additional_colocated_branches or {}).items()}

            if main_branch:
                roles[main_branch.name] = 'main'
                directory_name = urlutils.split_segment_parameters(main_branch.user_url)[0].rstrip('/').rsplit('/')[-1]
            else:
                roles[''] = 'main'
                directory_name = codebase

            ws = Workspace(
                main_branch,
                resume_branch=resume_branch,
                cached_branch=cached_branch,
                path=os.path.join(output_directory, directory_name),
                additional_colocated_branches=[b for (r, b) in (additional_colocated_branches or {}).items()],
                resume_branch_additional_colocated_branches=(
                    [name for (role, name, base_revision, revision) in resume_branches if role != 'main'] if resume_branches else None
                ),
                format=empty_format,
            )

            try:
                es.enter_context(ws)
            except IncompleteRead as e:
                traceback.print_exc()
                raise WorkerFailure(
                    "incomplete-read", str(e), stage=("setup", "clone"), transient=True) from e
            except MalformedTransform as e:
                traceback.print_exc()
                raise WorkerFailure("malformed-transform", str(e), stage=("setup", "clone"), transient=False) from e
            except TransformRenameFailed as e:
                traceback.print_exc()
                raise WorkerFailure("transform-rename-failed", str(e), stage=("setup", "clone"), transient=False) from e
            except ImmortalLimbo as e:
                traceback.print_exc()
                raise WorkerFailure("transform-immortal-limbo", str(e), stage=("setup", "clone"), transient=False) from e
            except UnexpectedHttpStatus as e:
                traceback.print_exc()
                if e.code == 502:
                    raise WorkerFailure("bad-gateway", str(e), stage=("setup", "clone"), transient=True) from e
                else:
                    raise WorkerFailure(
                        "http-%s" % e.code, str(e),
                        stage=("setup", "clone"), details={'status-code': e.code}) from e
            except TransportError as e:
                if "No space left on device" in str(e):
                    raise WorkerFailure("no-space-on-device", e.msg, stage=("setup", "clone"), transient=True) from e
                if "Too many open files" in str(e):
                    raise WorkerFailure("too-many-open-files", e.msg, stage=("setup", "clone"), transient=True) from e
                if "Temporary failure in name resolution" in str(e):
                    raise WorkerFailure(
                        "temporary-transport-error", str(e), stage=("setup", "clone"),
                        transient=True) from e
                traceback.print_exc()
                raise WorkerFailure("transport-error", str(e), stage=("setup", "clone")) from e
            except RemoteGitError as e:
                raise WorkerFailure("git-error", str(e), stage=("setup", "clone")) from e
            except TimeoutError as e:
                raise WorkerFailure("timeout", str(e), stage=("setup", "clone")) from e
            except MissingNestedTree as e:
                raise WorkerFailure("requires-nested-tree-support", str(e), stage=("setup", "clone")) from e

            logger.info('Workspace ready - starting.')

            if ws.local_tree.has_changes():
                raise WorkerFailure(
                    "unexpected-changes-in-tree",
                    description="The working tree has unexpected changes after initial clone",
                    stage=("setup", "clone"))

            if not skip_setup_validation:
                build_target.validate(ws.local_tree, subpath, build_config)

            if ws.main_branch:
                metadata.revision = metadata.main_branch_revision = ws.main_branch.last_revision()
            else:
                metadata.revision = metadata.main_branch_revision = NULL_REVISION

            metadata.codemod = {}

            if ws.resume_branch is None:
                # If the resume branch was discarded for whatever reason, then we
                # don't need to pass in the codemod result.
                resume_codemod_result = None

            if main_branch:
                metadata.add_remote("origin", main_branch.user_url)

            try:
                changer_result = build_target.make_changes(
                    ws.local_tree, subpath, command,
                    log_directory=output_directory,
                    resume_metadata=resume_codemod_result)
                if not ws.any_branch_changes():
                    raise WorkerFailure("nothing-to-do", "Nothing to do.", stage=("codemod", ), transient=False)
            except _WorkerFailure as e:
                if e.code == "nothing-to-do":
                    if ws.changes_since_main():
                        # This should only ever happen if we were resuming
                        assert ws.resume_branch is not None, \
                            ("Found existing changes despite not having resumed. "
                             f"Mainline: {ws.main_branch_revid!r}, local: {ws.local_tree.branch.last_revision()!r}")
                        raise WorkerFailure(
                            "nothing-new-to-do", e.description, stage=("codemod", ), transient=False) from e
                    elif force_build:
                        changer_result = GenericCommandResult(
                            description='No change build',
                            context={},
                            tags=[],
                            value=0)
                    else:
                        raise
                else:
                    raise
            finally:
                metadata.revision = ws.local_tree.branch.last_revision()

            metadata.refreshed = ws.refreshed
            metadata.value = changer_result.value
            metadata.codemod = changer_result.context
            metadata.target_branch_url = changer_result.target_branch_url
            metadata.description = changer_result.description

            result_branches: list[tuple[str, Optional[str], Optional[bytes], Optional[bytes]]] = []
            for (name, base_revision, revision) in ws.result_branches():
                try:
                    role = roles[name]
                except KeyError:
                    logging.warning('Unable to find role for branch %s', name)
                    continue
                if base_revision == revision:
                    continue
                result_branches.append((role, name, base_revision, revision))

            result_branch_roles = [role for (role, remote_name, br, r) in result_branches]
            assert len(result_branch_roles) == len(set(result_branch_roles)), \
                "Duplicate result branches: %r" % result_branches

            for (f, n, br, r) in (result_branches or []):
                metadata.add_branch(f, n, br, r)
            for (n, r) in (changer_result.tags or []):
                metadata.add_tag(n, r)  # type: ignore

            actual_vcs_type = get_branch_vcs_type(ws.local_tree.branch)

            if vcs_type is None:
                vcs_type = actual_vcs_type
            elif actual_vcs_type != vcs_type:
                raise WorkerFailure(
                    'vcs-type-mismatch',
                    f'Expected VCS {vcs_type}, got {actual_vcs_type}',
                    stage=("result-push", ),
                    transient=False)

            try:
                if vcs_type.lower() == "git":
                    import_branches_git(
                        target_repo_url, ws.local_tree.branch,
                        campaign, run_id, result_branches,
                        changer_result.tags,
                        update_current=False
                    )
                elif vcs_type.lower() == "bzr":
                    import_branches_bzr(
                        target_repo_url, ws.local_tree.branch,
                        campaign, run_id, result_branches,
                        changer_result.tags,
                        update_current=False
                    )
                else:
                    raise NotImplementedError
            except Exception as e:
                raise _push_error_to_worker_failure(e, ("result-push", )) from e

            if force_build:
                should_build = True
            else:
                should_build = (
                    any([(role is None or role == 'main')
                         for (role, name, br, r) in result_branches]))

            if should_build:
                metadata.target_name = build_target.name

                build_target_details = build_target.build(
                    ws.local_tree, subpath, output_directory, build_config)

                metadata.target_details = build_target_details
            else:
                build_target_details = None

            logging.info("Pushing result branch to %r", target_repo_url)

            try:
                if vcs_type.lower() == "git":
                    import_branches_git(
                        target_repo_url, ws.local_tree.branch,
                        campaign, run_id, result_branches, changer_result.tags,
                        update_current=True
                    )
                elif vcs_type.lower() == "bzr":
                    import_branches_bzr(
                        target_repo_url, ws.local_tree.branch,
                        campaign, run_id, result_branches, changer_result.tags,
                        update_current=True
                    )
                else:
                    raise NotImplementedError
            except Exception as e:
                raise _push_error_to_worker_failure(e, ("result-sym", )) from e

            if cached_branch_url:
                # TODO(jelmer): integrate into import_branches_git / import_branches_bzr
                logging.info(
                    "Pushing packaging branch cache to %s",
                    cached_branch_url)

                def tag_selector(tag_name):
                    return (tag_name.startswith(vendor + '/')
                            or tag_name.startswith('upstream/'))

                if ws.main_branch:
                    try:
                        push_branch(
                            ws.local_tree.branch,
                            cached_branch_url,
                            vcs_type=vcs_type.lower() if vcs_type is not None else None,
                            possible_transports=possible_transports,
                            stop_revision=ws.main_branch.last_revision(),
                            tag_selector=tag_selector,
                            overwrite=True,
                        )
                    except (InvalidHttpResponse, IncompleteRead,
                            ConnectionError, UnexpectedHttpStatus, RemoteGitError,
                            TransportNotPossible,
                            ssl.SSLEOFError, ssl.SSLError, TransportError) as e:
                        logging.warning(
                            "unable to push to cache URL %s: %s",
                            cached_branch_url, e)

            logging.info("All done.")
        except _WorkerFailure:
            raise
        except BaseException:
            traceback.print_exc()
            raise


INDEX_TEMPLATE = Template("""\
<html lang="en">
<head><title>Job{% if assignment %}{{ assignment['id'] }}{% endif %}</title></head>
<body>

{% if assignment %}
<h1>Run Details</h1>

<ul>
<li><a href="/assignment">Raw Assignment</a></li>
<li><b>Campaign</b>: {{ metadata.campaign }}</li>
<li><b>Codebase</b>: {{ metadata.codebase }}</li>
{% if metadata and metadata.start_time %}
<li><b>Start Time</b>: {{ metadata.start_time }}
<li><b>Current duration</b>: {{ datetime.utcnow() - metadata.start_time }}
{% endif %}
<li><b>Environment</b>: <ul>
{% for key, value in assignment['env'].items() %}
<li>{{ key }}: {{ value }}</li>
{% endfor %}
</li>
</ul>

<h2>Codemod</h2>

<ul>
<li><b>Command</b>: {{ assignment['codemod']['command'] }}</li>
<li><b>Environment</b>: <ul>
{% for key, value in assignment['codemod']['environment'].items() %}
<li>{{ key }}: {{ value }}</li>
{% endfor %}
</ul>
</li>
</ul>

<h2>Build</h2>

<ul>
<li><b>Target</b>: {{ assignment['build']['target'] }}</li>
<li><b>Force Build</b>: {{ assignment.get('force-build', False) }}</li>
<li><b>Environment</b>: <ul>
{% for key, value in assignment['build']['environment'].items() %}
<li>{{ key }}: {{ value }}</li>
{% endfor %}
</ul>
</li>
</ul>

{% if lognames %}
<h1>Logs</h1>
<ul>
{% for name in lognames %}
  <li><a href="/logs/{{ name }}">{{ name }}</a></li>
{% endfor %}
</ul>
{% endif %}

{% else %}

<p>No current assignment.</p>

{% endif %}

</body>
</html>
""")


@routes.get('/', name='index')
async def handle_index(request):
    if 'directory' in request.app['workitem']:
        lognames = [entry.name for entry in os.scandir(request.app['workitem']['directory'])
                    if entry.name.endswith('.log') and entry.is_file()]
    else:
        lognames = None
    return web.Response(text=INDEX_TEMPLATE.render(
        assignment=request.app['workitem'].get('assignment'),
        metadata=request.app['workitem'].get('metadata'),
        lognames=lognames,
        datetime=datetime),
        content_type='text/html', status=200)


@routes.get('/assignment', name='assignment')
async def handle_assignment(request):
    return web.json_response(request.app['workitem'].get('assignment'))


@routes.get('/intermediate-result', name='intermediate-result')
async def handle_intermediate_result(request):
    return web.json_response(request.app['workitem'].get('metadata'))


ARTIFACT_INDEX_TEMPLATE = Template("""\
<html>
<head><title>Artifact Index</title><head>
<body>
<h1>Artifacts</h1>
<ul>
{% for name in names %}
  <li><a href="/artifacts/{{ name }}">{{ name }}</a></li>
{% endfor %}
</ul>
</body>
</html>
""")


@routes.get('/artifacts/', name='artifact-index')
async def handle_artifact_index(request):
    if 'directory' not in request.app['workitem']:
        raise web.HTTPNotFound(text="Output directory not created yet")
    try:
        names = [entry.name for entry in os.scandir(request.app['workitem']['directory'])
                 if not entry.name.endswith('.log') and entry.is_file()]
    except FileNotFoundError as e:
        raise web.HTTPNotFound(text="Output directory does not exist") from e
    return web.Response(
        text=ARTIFACT_INDEX_TEMPLATE.render(names=names), content_type='text/html',
        status=200)


@routes.get('/artifacts/{filename}', name='artifact')
async def handle_artifact(request):
    if 'directory' not in request.app['workitem']:
        raise web.HTTPNotFound(text="Artifact directory not created yet")
    p = os.path.join(request.app['workitem']['directory'], request.match_info['filename'])
    if not os.path.exists(p):
        raise web.HTTPNotFound(text="No such artifact")
    return web.FileResponse(p)


LOG_INDEX_TEMPLATE = Template("""\
<html>
<head><title>Log Index</title><head>
<body>
<h1>Logs</h1>
<ul>
{% for name in names %}
  <li><a href="/logs/{{ name }}">{{ name }}</a></li>
{% endfor %}
</ul>
</body>
</html>
""")


@routes.get('/logs/', name='log-index')
async def handle_log_index(request):
    if 'directory' not in request.app['workitem']:
        raise web.HTTPNotFound(text="Log directory not created yet")
    names = [entry.name for entry in os.scandir(request.app['workitem']['directory'])
             if entry.name.endswith('.log')]
    if request.headers.get('Accept') == 'application/json':
        return web.json_response(names)
    else:
        return web.Response(
            text=LOG_INDEX_TEMPLATE.render(names=names), content_type='text/html',
            status=200)


@routes.get('/logs/{filename}', name='log')
async def handle_log(request):
    if 'directory' not in request.app['workitem']:
        raise web.HTTPNotFound(text="Log directory not created yet")
    p = os.path.join(request.app['workitem']['directory'], request.match_info['filename'])
    if not os.path.exists(p):
        raise web.HTTPNotFound(text="No such log file")
    return web.FileResponse(p)


@routes.get('/health', name='health')
async def handle_health(request):
    return web.Response(text='ok', status=200)


@routes.get('/log-id', name='log_id')
async def handle_log_id(request):
    assignment = request.app['workitem'].get('assignment')
    if assignment is None:
        return web.Response(text='', status=200)
    return web.Response(text=assignment.get('id', ''), status=200)


async def process_single_item(
        client, my_url: Optional[yarl.URL], node_name, workitem,
        jenkins_build_url=None, prometheus: Optional[str] = None,
        codebase: Optional[str] = None, campaign: Optional[str] = None,
        tee: bool = False, output_directory_base: Optional[str] = None):
    assignment = await client.get_assignment_raw(
        str(my_url), node_name,
        jenkins_build_url=jenkins_build_url,
        codebase=codebase, campaign=campaign,
    )
    workitem['assignment'] = assignment

    logging.debug("Got back assignment: %r", assignment)

    with ExitStack() as es:
        es.callback(workitem.clear)
        campaign = assignment["campaign"]
        codebase = assignment["codebase"]
        branch_url = assignment["branch"]["url"]
        vcs_type = assignment["branch"]["vcs_type"]
        additional_colocated_branches = assignment["branch"]["additional_colocated_branches"]
        force_build = assignment.get('force-build', False)
        subpath = assignment["branch"].get("subpath", "") or ""
        if assignment["resume"]:
            resume_result = assignment["resume"]["result"]
            resume_branch_url = assignment["resume"]["branch_url"].rstrip("/")
            resume_branches = [
                (role, name, base.encode("utf-8") if base else None,
                 revision.encode("utf-8") if revision else None)
                for (role, name, base, revision) in assignment["resume"]["branches"]
            ]
        else:
            resume_result = None
            resume_branch_url = None
            resume_branches = None
        cached_branch_url = assignment["branch"].get("cached_url")
        command = shlex.split(assignment["codemod"]["command"])
        target = assignment["build"]["target"]
        build_environment = assignment["build"].get("environment", {})
        build_config = assignment["build"].get("config", {})

        start_time = datetime.utcnow()
        metadata = Metadata()
        metadata.queue_id = int(assignment["queue_id"])
        metadata.start_time = start_time
        metadata.branch_url = branch_url
        metadata.vcs_type = vcs_type
        workitem['metadata'] = metadata

        target_repo_url = assignment["target_repository"]["url"]

        run_id = assignment["id"]

        possible_transports: list[Transport] = []

        env = assignment["env"]

        skip_setup_validation = assignment.get("skip-setup-validation", False)

        default_empty = assignment["branch"].get('default-empty', False)

        env.update(build_environment)

        logging.debug('Environment: %r', env)

        vendor = build_environment.get('DEB_VENDOR', 'debian')

        tmpdir_prefix = os.path.join(os.environ.get("TMPDIR", "/tmp"), "janitor-worker")

        output_directory = es.enter_context(TemporaryDirectory(prefix=tmpdir_prefix, dir=output_directory_base))
        workitem['directory'] = output_directory
        loop = asyncio.get_running_loop()

        main_task = loop.run_in_executor(
            None,
            partial(
                run_worker,
                codebase=codebase,
                main_branch_url=branch_url,
                run_id=run_id,
                subpath=subpath,
                vcs_type=vcs_type,
                build_config=build_config,
                env=env,
                command=command,
                output_directory=output_directory,
                metadata=metadata,
                target_repo_url=target_repo_url,
                vendor=vendor,
                campaign=campaign,
                target=target,
                resume_branch_url=resume_branch_url,
                resume_branches=resume_branches,
                cached_branch_url=cached_branch_url,
                resume_codemod_result=resume_result,
                possible_transports=possible_transports,
                force_build=force_build,
                tee=tee,
                additional_colocated_branches=additional_colocated_branches,
                skip_setup_validation=skip_setup_validation,
                default_empty=default_empty,
            ),
        )
        try:
            await main_task
        except _WorkerFailure as e:
            metadata.update(e)
            logging.info("Worker failed in %r (%s): %s",
                         e.stage, e.code, e.description)
            # This is a failure for the worker, but returning 0 will cause
            # jenkins to mark the job having failed, which is not really
            # true.  We're happy if we get to successfully POST to /finish
            return
        except OSError as e:
            if e.errno == errno.ENOSPC:
                metadata.code = "no-space-on-device"
                metadata.description = str(e)
            else:
                metadata.code = "worker-exception"
                metadata.description = str(e)
            return
        except BaseException as e:
            metadata.code = "worker-failure"
            metadata.description = ''.join(traceback.format_exception_only(type(e), e)).rstrip('\n')
            return
        else:
            metadata.code = None
            logging.info("%s", metadata.description)
            return
        finally:
            finish_time = datetime.utcnow()
            metadata.finish_time = finish_time
            logging.info("Elapsed time: %s", finish_time - start_time)

            result = await client.upload_results(
                run_id=assignment["id"],
                metadata=metadata,
                output_directory=output_directory,
            )

            logging.info('Results uploaded')

            logging.debug("Result: %r", result)

            if prometheus:
                await push_to_gateway(
                    prometheus, job="janitor.worker",
                    grouping_key={
                        'run_id': assignment['id'],
                        'campaign': campaign,  # type: ignore
                    },
                    registry=REGISTRY)
            workitem.clear()


async def create_app():
    app = web.Application()
    app['workitem'] = {}
    app.router.add_routes(routes)
    setup_metrics(app)
    return app


async def main(debug, listen_address, port, base_url, my_url, codebase,
               campaign, credentials, prometheus, tee, loop, external_address,
               output_directory, gcp_logging):
    if debug:
        loop = asyncio.get_event_loop()
        loop.set_debug(True)
        loop.slow_callback_duration = 0.001
        warnings.simplefilter('always', ResourceWarning)

    if gcp_logging:
        import google.cloud.logging
        log_client = google.cloud.logging.Client()
        log_client.get_default_handler()
        log_client.setup_logging()
    else:
        if debug:
            log_level = logging.DEBUG
        else:
            log_level = logging.INFO

        logging.basicConfig(
            level=log_level,
            format="[%(asctime)s] %(message)s",
            datefmt="%Y-%m-%d %H:%M:%S")

    app = await create_app()

    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_address, port)
    await site.start()
    (_site_addr, site_port) = site._server.sockets[0].getsockname()  # type: ignore

    global_config = GlobalStack()
    global_config.set("branch.fetch_tags", True)

    base_url = yarl.URL(base_url)

    auth: Optional[BasicAuth]
    if credentials:
        with open(credentials) as f:
            creds = json.load(f)
        auth = BasicAuth(login=creds["login"], password=creds["password"])
    elif 'WORKER_NAME' in os.environ and 'WORKER_PASSWORD' in os.environ:
        auth = BasicAuth(
            login=os.environ["WORKER_NAME"],
            password=os.environ["WORKER_PASSWORD"])
        del os.environ["WORKER_PASSWORD"]
    else:
        auth = BasicAuth.from_url(base_url)

    if auth is not None:
        class WorkerCredentialStore(PlainTextCredentialStore):
            def get_credentials(
                self, protocol, host, port=None, user=None, path=None, realm=None
            ):
                if host == base_url.host:
                    return {
                        "user": auth.login,  # type: ignore
                        "password": auth.password,  # type: ignore
                        "protocol": protocol,
                        "port": port,
                        "host": host,
                        "realm": realm,
                        "verify_certificates": True,
                    }
                return None

        credential_store_registry.register(
            "janitor-worker", WorkerCredentialStore, fallback=True
        )

    jenkins_build_url = os.environ.get('BUILD_URL')

    node_name = os.environ.get("NODE_NAME")
    if not node_name:
        node_name = socket.gethostname()

    loop = asyncio.get_event_loop()
    if my_url:
        my_url = yarl.URL(my_url)
    elif external_address:
        my_url = yarl.URL.build(
            scheme='http', host=external_address, port=site_port)
    elif 'MY_IP' in os.environ:
        my_url = yarl.URL.build(
            scheme='http', host=os.environ['MY_IP'], port=site_port)
    elif await is_gce_instance():
        external_ip = await gce_external_ip()
        if external_ip:
            my_url = yarl.URL.build(
                scheme='http', host=external_ip, port=site_port)
        else:
            my_url = None
    # TODO(jelmer): Find out kubernetes IP?
    else:
        my_url = None

    if my_url:
        logging.info('Diagnostics available at %s', my_url)

    if auth:
        client = Client(str(base_url), auth.login, auth.password, USER_AGENT)
    else:
        client = Client(str(base_url), None, None, user_agent=USER_AGENT)

    loop.add_signal_handler(
        signal.SIGINT, handle_sigterm, client,
        app['workitem'], signal.SIGINT)
    loop.add_signal_handler(
        signal.SIGTERM, handle_sigterm, client,
        app['workitem'], signal.SIGTERM)

    while True:
        try:
            await process_single_item(
                client, my_url=my_url,
                node_name=node_name,
                workitem=app['workitem'],
                jenkins_build_url=jenkins_build_url,
                prometheus=prometheus,
                codebase=codebase, campaign=campaign,
                tee=tee, output_directory_base=output_directory)
        except AssignmentFailure as e:
            logging.fatal("failed to get assignment: %s", e)
            return 1
        except EmptyQueue:
            logging.info('queue is empty')
            return 0
        except ResultUploadFailure as e:
            sys.stderr.write(str(e))
            return 1
        if not loop:
            return 0


def main_sync(**kwargs):
    return asyncio.run(main(**kwargs))
