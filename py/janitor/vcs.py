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

__all__ = [
    "is_alioth_url",
    "is_authenticated_url",
    "BranchOpenFailure",
    "open_branch_ext",
    "MirrorFailure",
    "UnsupportedVcs",
    "open_cached_branch",
    "VcsManager",
    "LocalGitVcsManager",
    "LocalBzrVcsManager",
    "RemoteGitVcsManager",
    "RemoteBzrVcsManager",
    "get_run_diff",
    "get_vcs_managers",
    "get_vcs_managers_from_config",
    "get_branch_vcs_type",
]

import logging
import ssl
import sys
from io import BytesIO
from typing import Optional

import breezy.bzr  # noqa: F401
import breezy.git  # noqa: F401
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
from breezy.transport import Transport, get_transport_from_url
from silver_platter import (
    BranchMissing,
    BranchRateLimited,
    BranchTemporarilyUnavailable,
    BranchUnavailable,
    BranchUnsupported,
)
from silver_platter import (
    _open_branch as open_branch,
)
from yarl import URL

from ._common import (
    get_branch_vcs_type,
    is_alioth_url,
    is_authenticated_url,
)
from ._common import (
    vcs as _vcs_rs,
)

VcsManager = _vcs_rs.VcsManager
RemoteBzrVcsManager = _vcs_rs.RemoteBzrVcsManager
RemoteGitVcsManager = _vcs_rs.RemoteGitVcsManager
LocalBzrVcsManager = _vcs_rs.LocalBzrVcsManager
LocalGitVcsManager = _vcs_rs.LocalGitVcsManager


class BranchOpenFailure(Exception):
    """Failure to open a branch."""

    def __init__(
        self, code: str, description: str, retry_after: Optional[int] = None
    ) -> None:
        self.code = code
        self.description = description
        self.retry_after = retry_after


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
            'Branch does not exist: Not a branch: "https://anonscm.debian.org'
        ):
            code = "hosted-on-alioth"
        else:
            code = "branch-missing"
        msg = str(e)
        if e.url not in msg:
            msg = f"{msg} ({e.url})"
        return BranchOpenFailure(code, msg)
    if isinstance(e, BranchUnsupported):
        if getattr(e, "vcs", None):
            code = f"unsupported-vcs-{e.vcs}"
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
        try:
            return open_branch(vcs_url, possible_transports, probers=probers)
        except TypeError:
            return open_branch(vcs_url)
    except (
        BranchUnavailable,
        BranchMissing,
        BranchUnsupported,
        BranchRateLimited,
    ) as e:
        raise _convert_branch_exception(vcs_url, e) from e


class MirrorFailure(Exception):
    """Branch failed to mirror."""

    def __init__(self, branch_name: str, reason: str) -> None:
        self.branch_name = branch_name
        self.reason = reason


class UnsupportedVcs(Exception):
    """Specified vcs type is not supported."""


def open_cached_branch(
    url, trace_context: Optional[TraceContext] = None
) -> Optional[Branch]:
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
        logging.warning("Unable to access cache branch at %s: %r", url, e)
        raise


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


def get_vcs_managers(location: str) -> dict[str, VcsManager]:
    if "=" not in location:
        return {
            "git": RemoteGitVcsManager(
                str(URL(location) / "git"),
            ),
            "bzr": RemoteBzrVcsManager(
                str(URL(location) / "bzr"),
            ),
        }
    ret: dict[str, VcsManager] = {}
    for p in location.split(","):
        (k, v) = p.split("=", 1)
        if k == "git":
            ret[k] = RemoteGitVcsManager(str(URL(v)))
        elif k == "bzr":
            ret[k] = RemoteBzrVcsManager(str(URL(v)))
        else:
            raise ValueError(f"unsupported vcs {k}")
    return ret


def get_vcs_managers_from_config(config) -> dict[str, VcsManager]:
    ret: dict[str, VcsManager] = {}
    if config.git_location:
        parsed = urlutils.URL.from_string(config.git_location)
        if parsed.scheme in ("", "file"):
            ret["git"] = LocalGitVcsManager(parsed.path)
        else:
            ret["git"] = RemoteGitVcsManager(
                config.git_location,
            )
    if config.bzr_location:
        parsed = urlutils.URL.from_string(config.git_location)
        if parsed.scheme in ("", "file"):
            ret["bzr"] = LocalBzrVcsManager(parsed.path)
        else:
            ret["bzr"] = RemoteBzrVcsManager(
                config.git_location,
            )
    return ret


def main(argv=None):
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("url", type=str)
    args = parser.parse_args(argv)
    open_branch_ext(args.url)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
