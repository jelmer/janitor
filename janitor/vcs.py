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
import logging
import os
import ssl
import sys
import urllib.parse
from collections.abc import Iterable
from contextlib import suppress
from io import BytesIO
from typing import Optional

import breezy.bzr  # noqa: F401
import breezy.git  # noqa: F401
from aiohttp import ClientSession, ClientTimeout
from aiozipkin.helpers import TraceContext
from breezy import urlutils
from breezy.branch import Branch
from breezy.controldir import BranchReferenceLoop
from breezy.diff import show_diff_trees
from breezy.errors import (
    InvalidHttpResponse,
    NoSuchRevision,
    NotBranchError,
)

try:
    from breezy.errors import ConnectionError  # type: ignore
except ImportError:  # breezy >= 4
    pass
from breezy.git.remote import RemoteGitError
from breezy.repository import Repository
from breezy.revision import NULL_REVISION
from breezy.transport import Transport, get_transport_from_url
from dulwich.objects import ZERO_SHA
from silver_platter.utils import (
    BranchMissing,
    BranchRateLimited,
    BranchTemporarilyUnavailable,
    BranchUnavailable,
    BranchUnsupported,
    open_branch,
)
from yarl import URL

EMPTY_GIT_TREE = b'4b825dc642cb6eb9a060e54bf8d69288fbee4904'


class BranchOpenFailure(Exception):
    """Failure to open a branch."""

    def __init__(self, code: str, description: str, retry_after: Optional[int] = None) -> None:
        self.code = code
        self.description = description
        self.retry_after = retry_after


def get_vcs_abbreviation(repository: Repository) -> str:
    vcs = getattr(repository, "vcs", None)
    if vcs:
        return vcs.abbreviation
    return "bzr"


def is_alioth_url(url: str) -> bool:
    return urllib.parse.urlparse(url).netloc in (
        "svn.debian.org",
        "bzr.debian.org",
        "anonscm.debian.org",
        "hg.debian.org",
        "git.debian.org",
        "alioth.debian.org",
    )


def _convert_branch_exception(vcs_url: str, e: Exception) -> Exception:
    if isinstance(e, BranchRateLimited):
        code = "too-many-requests"
        return BranchOpenFailure(code, str(e), retry_after=e.retry_after)
    elif isinstance(e, BranchUnavailable):
        if "http code 429: Too Many Requests" in str(e):
            code = "too-many-requests"
        elif is_alioth_url(vcs_url):
            code = "hosted-on-alioth"
        elif "Unable to handle http code 401: Unauthorized" in str(
            e
        ) or "Unexpected HTTP status 401 for " in str(e):
            code = "401-unauthorized"
        elif "Unable to handle http code 502: Bad Gateway" in str(
            e
        ) or "Unexpected HTTP status 502 for " in str(e):
            code = "502-bad-gateway"
        elif str(e).startswith("Subversion branches are not yet"):
            code = "unsupported-vcs-svn"
        elif str(e).startswith("Mercurial branches are not yet"):
            code = "unsupported-vcs-hg"
        elif str(e).startswith("Darcs branches are not yet"):
            code = "unsupported-vcs-darcs"
        elif str(e).startswith("Fossil branches are not yet"):
            code = "unsupported-vcs-fossil"
        elif isinstance(e, BranchTemporarilyUnavailable):
            code = "branch-temporarily-unavailable"
        else:
            code = "branch-unavailable"
        msg = str(e)
        if e.url not in msg:
            msg = f"{msg} ({e.url})"
        return BranchOpenFailure(code, msg)
    if isinstance(e, BranchMissing):
        if str(e).startswith(
            "Branch does not exist: Not a branch: " '"https://anonscm.debian.org'
        ):
            code = "hosted-on-alioth"
        else:
            code = "branch-missing"
        msg = str(e)
        if e.url not in msg:
            msg = f"{msg} ({e.url})"
        return BranchOpenFailure(code, msg)
    if isinstance(e, BranchUnsupported):
        if getattr(e, 'vcs', None):
            code = "unsupported-vcs-%s" % e.vcs
        elif str(e).startswith("Unsupported protocol for url "):
            if "anonscm.debian.org" in str(e) or "svn.debian.org" in str(e):
                code = "hosted-on-alioth"
            else:
                if "svn://" in str(e):
                    code = "unsupported-vcs-svn"
                elif "cvs+pserver://" in str(e):
                    code = "unsupported-vcs-cvs"
                else:
                    code = "unsupported-vcs-protocol"
        else:
            if str(e).startswith("Subversion branches are not yet"):
                code = "unsupported-vcs-svn"
            elif str(e).startswith("Mercurial branches are not yet"):
                code = "unsupported-vcs-hg"
            elif str(e).startswith("Darcs branches are not yet"):
                code = "unsupported-vcs-darcs"
            elif str(e).startswith("Fossil branches are not yet"):
                code = "unsupported-vcs-fossil"
            else:
                code = "unsupported-vcs"
        msg = str(e)
        if e.url not in msg:
            msg = f"{msg} ({e.url})"
        return BranchOpenFailure(code, msg)

    return e


def open_branch_ext(
    vcs_url: str, possible_transports: Optional[list[Transport]] = None, probers=None
) -> Branch:
    try:
        return open_branch(vcs_url, possible_transports, probers=probers)
    except (BranchUnavailable, BranchMissing, BranchUnsupported, BranchRateLimited) as e:
        raise _convert_branch_exception(vcs_url, e) from e


class MirrorFailure(Exception):
    """Branch failed to mirror."""

    def __init__(self, branch_name: str, reason: str) -> None:
        self.branch_name = branch_name
        self.reason = reason


class UnsupportedVcs(Exception):
    """Specified vcs type is not supported."""


def open_cached_branch(
        url, trace_context: Optional[TraceContext] = None) -> Optional[Branch]:
    # TODO(jelmer): Somehow pass in trace context headers
    try:
        transport = get_transport_from_url(url)
        return Branch.open_from_transport(transport)
    except NotBranchError:
        return None
    except RemoteGitError:
        return None
    except InvalidHttpResponse:
        return None
    except ConnectionError as e:
        logging.info("Unable to reach cache server: %s", e)
        return None
    except BranchReferenceLoop:
        return None
    except ssl.SSLCertVerificationError as e:
        logging.warning('Unable to access cache branch at %s: %r', url, e)
        raise


class VcsManager:
    def get_branch(
            self, codebase: str, branch_name: str,
            *, trace_context: Optional[TraceContext] = None
    ) -> Branch:
        raise NotImplementedError(self.get_branch)

    def get_branch_url(
        self, codebase: str, branch_name: str
    ) -> Optional[str]:
        raise NotImplementedError(self.get_branch_url)

    def get_repository(
        self, codebase: str
    ) -> Repository:
        raise NotImplementedError(self.get_repository)

    def get_repository_url(self, codebase: str) -> str:
        raise NotImplementedError(self.get_repository_url)

    def list_repositories(self) -> Iterable[str]:
        raise NotImplementedError(self.list_repositories)

    async def get_diff(
            self, codebase: str, old_revid: bytes,
            new_revid: bytes) -> bytes:
        raise NotImplementedError(self.get_diff)

    async def get_revision_info(
            self, codebase: str, old_revid: bytes, new_revid: bytes):
        raise NotImplementedError(self.get_revision_info)


class LocalGitVcsManager(VcsManager):
    def __init__(self, base_path: str) -> None:
        self.base_path = base_path

    def __repr__(self) -> str:
        return f"{type(self).__name__}({self.base_path!r})"

    def get_branch(self, codebase, branch_name, *, trace_context=None):
        url = self.get_branch_url(codebase, branch_name)
        try:
            return open_branch(url)
        except (BranchUnavailable, BranchMissing):
            return None

    def get_branch_url(self, codebase, branch_name):
        return urlutils.join_segment_parameters(
            "file:%s" % (
                os.path.join(self.base_path, codebase)), {
                    "branch": urlutils.escape(branch_name, safe='')})

    def get_repository(self, codebase):
        try:
            return Repository.open(os.path.join(self.base_path, codebase))
        except NotBranchError:
            return None

    def get_repository_url(self, codebase):
        return os.path.join(self.base_path, codebase)

    def list_repositories(self):
        for entry in os.scandir(os.path.join(self.base_path)):
            yield entry.name

    async def get_diff(self, codebase, old_revid, new_revid):
        if old_revid == new_revid:
            return b""
        repo = self.get_repository(codebase)
        if repo is None:
            raise KeyError

        if old_revid == NULL_REVISION:
            old_sha = EMPTY_GIT_TREE
        else:
            old_sha = repo.lookup_bzr_revision_id(old_revid)[0]
        if new_revid == NULL_REVISION:
            new_sha = EMPTY_GIT_TREE
        else:
            new_sha = repo.lookup_bzr_revision_id(new_revid)[0]

        args: list[str] = [
            "git",
            "diff",
            old_sha.decode('utf-8'), new_sha.decode('utf-8')
        ]

        p = await asyncio.create_subprocess_exec(
            *args,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            stdin=asyncio.subprocess.PIPE,
            cwd=repo.user_transport.local_abspath('.'),
        )

        try:
            (stdout, stderr) = await asyncio.wait_for(p.communicate(b""), 30.0)
        except asyncio.TimeoutError:
            with suppress(ProcessLookupError):
                p.kill()
            raise

        if p.returncode != 0:
            raise RuntimeError('git diff failed: %s', stderr.decode())
        return stdout

    async def get_revision_info(self, codebase, old_revid, new_revid):
        from dulwich.errors import MissingCommitError
        repo = self.get_repository(codebase)
        if repo is None:
            raise KeyError
        ret = []
        old_sha = repo.lookup_bzr_revision_id(old_revid)[0]
        new_sha = repo.lookup_bzr_revision_id(new_revid)[0]
        try:
            walker = repo._git.get_walker(include=[new_sha], exclude=[old_sha])
        except MissingCommitError as e:
            raise KeyError from e
        for entry in walker:
            ret.append({
                'commit-id': entry.commit.id.decode('ascii'),
                'revision-id': repo.lookup_foreign_revision_id(entry.commit.id).decode('utf-8'),
                'message': entry.commit.message.decode('utf-8', 'replace')})
            await asyncio.sleep(0)
        return ret


class LocalBzrVcsManager(VcsManager):
    def __init__(self, base_path: str) -> None:
        self.base_path = base_path

    def __repr__(self) -> str:
        return f"{type(self).__name__}({self.base_path!r})"

    def get_branch(self, codebase, branch_name, *, trace_context=None):
        url = self.get_branch_url(codebase, branch_name)
        try:
            return open_branch(url)
        except (BranchUnavailable, BranchMissing):
            return None

    def get_branch_url(self, codebase, branch_name):
        return os.path.join(self.base_path, codebase, branch_name)

    def get_repository(self, codebase):
        try:
            return Repository.open(os.path.join(self.base_path, codebase))
        except NotBranchError:
            return None

    def get_repository_url(self, codebase):
        return os.path.join(self.base_path, codebase)

    def list_repositories(self):
        for entry in os.scandir(os.path.join(self.base_path)):
            yield entry.name

    async def get_diff(self, codebase, old_revid, new_revid):
        if old_revid == new_revid:
            return b""
        repo = self.get_repository(codebase)
        if repo is None:
            raise KeyError
        args = [
            sys.executable,
            '-m',
            'breezy',
            "diff",
            '-rrevid:{}..revid:{}'.format(
                old_revid.decode(),
                new_revid.decode(),
            ),
            urlutils.join(repo.user_url)
        ]

        p = await asyncio.create_subprocess_exec(
            *args,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            stdin=asyncio.subprocess.PIPE,
        )

        try:
            (stdout, stderr) = await asyncio.wait_for(p.communicate(b""), 30.0)
        except asyncio.TimeoutError:
            with suppress(ProcessLookupError):
                p.kill()
            raise

        if p.returncode != 3:
            return stdout

        raise RuntimeError('diff returned %d' % p.returncode)

    async def get_revision_info(self, codebase, old_revid, new_revid):
        repo = self.get_repository(codebase)
        if repo is None:
            raise KeyError
        ret = []
        with repo.lock_read():
            graph = repo.get_graph()
            for rev in repo.iter_revisions(graph.iter_lefthand_ancestry(new_revid, [old_revid])):
                ret.append({
                    'revision-id': rev.revision_id.decode('utf-8'),
                    'link': None,
                    'message': rev.description})
                await asyncio.sleep(0)
        return ret


class RemoteGitVcsManager(VcsManager):
    def __init__(self, base_url: str, trace_configs=None) -> None:
        self.base_url = base_url
        self.trace_configs = trace_configs

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.base_url == other.base_url

    async def get_diff(self, codebase, old_revid, new_revid):
        if old_revid == new_revid:
            return b""

        url = self.get_diff_url(codebase, old_revid, new_revid)
        async with ClientSession(trace_configs=self.trace_configs) as client, client.get(url, timeout=ClientTimeout(30), raise_for_status=True) as resp:
            return await resp.read()

    def _lookup_revid(self, revid, default):
        if revid == NULL_REVISION:
            return default
        else:
            return revid[len(b'git-v1:'):]

    async def get_revision_info(self, codebase, old_revid, new_revid):
        url = urllib.parse.urljoin(self.base_url, "{}/revision-info?old={}&new={}".format(
            codebase,
            self._lookup_revid(old_revid, ZERO_SHA).decode('utf-8'),
            self._lookup_revid(new_revid, ZERO_SHA).decode('utf-8')))
        async with ClientSession(trace_configs=self.trace_configs) as client, client.get(url, timeout=ClientTimeout(30), raise_for_status=True) as resp:
            return await resp.json()

    def __repr__(self) -> str:
        return f"{type(self).__name__}({self.base_url!r})"

    def get_diff_url(self, codebase, old_revid, new_revid):
        return urllib.parse.urljoin(self.base_url, "{}/diff?old={}&new={}".format(
            codebase,
            self._lookup_revid(old_revid, EMPTY_GIT_TREE).decode('utf-8'),
            self._lookup_revid(new_revid, EMPTY_GIT_TREE).decode('utf-8')))

    def get_branch(self, codebase, branch_name, *, trace_context=None):
        url = self.get_branch_url(codebase, branch_name)
        return open_cached_branch(url, trace_context=trace_context)

    def get_branch_url(self, codebase, branch_name) -> str:
        return urlutils.join_segment_parameters("{}/{}".format(
            self.base_url.rstrip("/"), codebase), {
                "branch": urlutils.escape(branch_name, safe='')})

    def get_repository_url(self, codebase: str) -> str:
        return "{}/{}".format(self.base_url.rstrip("/"), codebase)


class RemoteBzrVcsManager(VcsManager):
    def __init__(self, base_url: str, *, trace_configs=None) -> None:
        self.base_url = base_url
        self.trace_configs = trace_configs

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.base_url == other.base_url

    async def get_diff(self, codebase, old_revid, new_revid):
        if old_revid == new_revid:
            return b""
        url = self.get_diff_url(codebase, old_revid, new_revid)
        async with ClientSession(trace_configs=self.trace_configs) as client, client.get(url, timeout=ClientTimeout(30), raise_for_status=True) as resp:
            return await resp.read()

    async def get_revision_info(self, codebase, old_revid, new_revid):
        url = urllib.parse.urljoin(self.base_url, "{}/revision-info?old={}&new={}".format(
            codebase, old_revid.decode('utf-8'),
            new_revid.decode('utf-8')))
        async with ClientSession(trace_configs=self.trace_configs) as client, client.get(url, timeout=ClientTimeout(30), raise_for_status=True) as resp:
            return await resp.json()

    def __repr__(self) -> str:
        return f"{type(self).__name__}({self.base_url!r})"

    def get_diff_url(self, codebase, old_revid, new_revid):
        return urllib.parse.urljoin(self.base_url, "{}/diff?old={}&new={}".format(
            codebase, old_revid.decode('utf-8'),
            new_revid.decode('utf-8')))

    def get_branch(self, codebase, branch_name, *, trace_context=None):
        url = self.get_branch_url(codebase, branch_name)
        return open_cached_branch(url, trace_context=trace_context)

    def get_branch_url(self, codebase, branch_name) -> str:
        return "{}/{}/{}".format(self.base_url.rstrip("/"), codebase, branch_name)

    def get_repository_url(self, codebase: str) -> str:
        return "{}/{}".format(self.base_url.rstrip("/"), codebase)


def get_run_diff(vcs_manager: VcsManager, run, role) -> bytes:
    f = BytesIO()
    try:
        repo = vcs_manager.get_repository(run.codebase)  # type: Optional[Repository]
    except NotBranchError:
        repo = None
    if repo is None:
        return b"Local VCS repository for %s temporarily inaccessible" % (
            run.codebase.encode("ascii")
        )
    for actual_role, _, base_revision, revision in run.result_branches:
        if role == actual_role:
            old_revid = base_revision
            new_revid = revision
            break
    else:
        return b"No branch with role %s" % role.encode()

    try:
        old_tree = repo.revision_tree(old_revid)
    except NoSuchRevision:
        return b"Old revision %s temporarily missing" % (old_revid,)
    try:
        new_tree = repo.revision_tree(new_revid)
    except NoSuchRevision:
        return b"New revision %s temporarily missing" % (new_revid,)
    show_diff_trees(old_tree, new_tree, to_file=f)
    return f.getvalue()


def get_vcs_managers(location, *, trace_configs=None):
    if '=' not in location:
        return {
            'git': RemoteGitVcsManager(
                str(URL(location) / "git"), trace_configs=trace_configs),
            'bzr': RemoteBzrVcsManager(
                str(URL(location) / "bzr"), trace_configs=trace_configs),
        }
    ret: dict[str, VcsManager] = {}
    for p in location.split(','):
        (k, v) = p.split('=', 1)
        if k == 'git':
            ret[k] = RemoteGitVcsManager(
                str(URL(v)), trace_configs=trace_configs)
        elif k == 'bzr':
            ret[k] = RemoteBzrVcsManager(
                str(URL(v)), trace_configs=trace_configs)
        else:
            raise ValueError('unsupported vcs %s' % k)
    return ret


def get_vcs_managers_from_config(
        config, *, trace_configs=None) -> dict[str, VcsManager]:
    ret: dict[str, VcsManager] = {}
    if config.git_location:
        parsed = urlutils.URL.from_string(config.git_location)
        if parsed.scheme in ("", "file"):
            ret['git'] = LocalGitVcsManager(parsed.path)
        else:
            ret['git'] = RemoteGitVcsManager(
                config.git_location, trace_configs=trace_configs)
    if config.bzr_location:
        parsed = urlutils.URL.from_string(config.git_location)
        if parsed.scheme in ("", "file"):
            ret['bzr'] = LocalBzrVcsManager(parsed.path)
        else:
            ret['bzr'] = RemoteBzrVcsManager(
                config.git_location, trace_configs=trace_configs)
    return ret


def is_authenticated_url(url: str):
    return (url.startswith('git+ssh://') or url.startswith('bzr+ssh://'))


def main(argv=None):
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("url", type=str)
    args = parser.parse_args(argv)
    open_branch_ext(args.url)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
