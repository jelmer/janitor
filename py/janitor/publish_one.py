#!/usr/bin/python3
# Copyright (C) 2019 Jelmer Vernooij <jelmer@jelmer.uk>
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

"""Publisher for a single branch.

This is the worker module for the publish service. For each branch that needs
to be published, this module gets invoked. It accepts some JSON on stdin with a
request, and writes results to standard out as JSON.
"""

import logging
import os
import shlex
import traceback
import urllib.error
import urllib.parse
import urllib.request
from contextlib import ExitStack
from email.utils import parseaddr
from typing import Any, Optional

from breezy.branch import Branch
from breezy.errors import DivergedBranches, NoSuchRevision, UnexpectedHttpStatus
from breezy.forge import (
    Forge,
    ForgeLoginRequired,
    UnsupportedForge,
    determine_title,
    get_forge,
)
from breezy.git.remote import RemoteGitBranch, RemoteGitError
from breezy.plugins.gitlab.forge import (
    ForkingDisabled,
    GitLabConflict,
    ProjectCreationTimeout,
)
from breezy.transport import Transport
from jinja2 import (
    Environment,
    FileSystemLoader,
    Template,
    TemplateNotFound,
    TemplateSyntaxError,
    select_autoescape,
)
from silver_platter.publish import (
    EmptyMergeProposal,
    InsufficientChangesForNewProposal,
    MergeProposal,
    MergeProposalExists,
    NoSuchProject,
    PermissionDenied,
    PublishResult,
    SourceNotDerivedFromTarget,
    find_existing_proposed,
    merge_conflicts,
    publish_changes,
)
from silver_platter.utils import (
    BranchMissing,
    BranchRateLimited,
    BranchTemporarilyUnavailable,
    BranchUnavailable,
    create_temp_sprout,
    full_branch_url,
    open_branch,
)

from ._launchpad import override_launchpad_consumer_name
from .debian.debdiff import debdiff_is_empty, markdownify_debdiff

override_launchpad_consumer_name()


MODE_SKIP = "skip"
MODE_BUILD_ONLY = "build-only"
MODE_PUSH = "push"
MODE_PUSH_DERIVED = "push-derived"
MODE_PROPOSE = "propose"
MODE_ATTEMPT_PUSH = "attempt-push"
MODE_BTS = "bts"
SUPPORTED_MODES = [
    MODE_PUSH_DERIVED,
    MODE_PROPOSE,
    MODE_PUSH,
    MODE_BUILD_ONLY,
    MODE_SKIP,
]


class PublishFailure(Exception):
    def __init__(self, code, description) -> None:
        self.code = code
        self.description = description


class PublishNothingToDo(Exception):
    def __init__(self, description) -> None:
        self.description = description


class MergeConflict(Exception):
    def __init__(self, target_branch, source_branch) -> None:
        self.target_branch = target_branch
        self.source_branch = source_branch


class DebdiffRetrievalError(Exception):
    def __init__(self, reason) -> None:
        self.reason = reason


def publish(
    *,
    template_env,
    campaign: str,
    commit_message_template: Optional[str],
    title_template: Optional[str],
    codemod_result: Any,
    mode: str,
    role: str,
    forge: Forge,
    target_branch: Branch,
    source_branch: Branch,
    derived_branch_name: str,
    resume_branch: Optional[Branch] = None,
    log_id: Optional[str] = None,
    existing_proposal: Optional[MergeProposal] = None,
    allow_create_proposal: bool = False,
    debdiff: Optional[bytes] = None,
    reviewers: Optional[list[str]] = None,
    result_tags: Optional[dict[str, bytes]] = None,
    stop_revision: Optional[bytes] = None,
    extra_context: Optional[dict[str, Any]] = None,
):
    def get_proposal_description(description_format, existing_proposal):
        vs = {
            "log_id": log_id,
            "campaign": campaign,
            "role": role,
        }
        if extra_context:
            vs.update(extra_context)
        if codemod_result:
            vs.update(codemod_result)
            vs["codemod"] = codemod_result
        if debdiff:
            vs["debdiff"] = debdiff.decode("utf-8", "replace")
        if description_format == "markdown":
            template = template_env.get_template(campaign + ".md")
        else:
            template = template_env.get_template(campaign + ".txt")
        return template.render(vs)

    def get_proposal_commit_message(existing_proposal):
        if commit_message_template:
            template = Template(commit_message_template)
            return template.render(codemod_result or {})
        else:
            return None

    def get_proposal_title(existing_proposal):
        if title_template:
            template = Template(title_template)
            return template.render(codemod_result or {})
        else:
            try:
                description = get_proposal_description("text", existing_proposal)
            except TemplateNotFound:
                description = get_proposal_description("markdown", existing_proposal)
            return determine_title(description)

    with target_branch.lock_read(), source_branch.lock_read():
        try:
            if merge_conflicts(target_branch, source_branch, stop_revision):
                raise MergeConflict(target_branch, source_branch)
        except NoSuchRevision as e:
            raise PublishFailure(
                description=f"Revision missing: {e.revision}",  # type: ignore
                code="revision-missing",
            ) from e

    labels: Optional[list[str]]

    if forge and forge.supports_merge_proposal_labels:
        labels = [campaign]
    else:
        labels = None
    try:
        return publish_changes(
            source_branch,
            target_branch,
            resume_branch,
            mode,
            derived_branch_name,
            get_proposal_description=get_proposal_description,
            get_proposal_commit_message=get_proposal_commit_message,
            get_proposal_title=get_proposal_title,
            forge=forge,
            allow_create_proposal=allow_create_proposal,
            overwrite_existing=True,
            existing_proposal=existing_proposal,
            labels=labels,
            tags=result_tags,
            allow_collaboration=True,
            reviewers=reviewers,
            stop_revision=stop_revision,
        )
    except DivergedBranches as e:
        raise PublishFailure(
            description="Upstream branch has diverged from local changes.",
            code="diverged-branches",
        ) from e
    except UnsupportedForge as e:
        raise PublishFailure(
            description=f"Forge unsupported: {target_branch.repository.user_url}.",
            code="hoster-unsupported",
        ) from e
    except NoSuchProject as e:
        raise PublishFailure(
            description=f"project {e.project} was not found", code="project-not-found"
        ) from e
    except ForkingDisabled as e:
        raise PublishFailure(
            description=f"Forking disabled: {target_branch.repository.user_url}",
            code="forking-disabled",
        ) from e
    except PermissionDenied as e:
        raise PublishFailure(description=str(e), code="permission-denied") from e
    except TemplateNotFound as e:
        raise PublishFailure(description=str(e), code="template-not-found") from e
    except TemplateSyntaxError as e:
        raise PublishFailure(description=str(e), code="template-syntax-error") from e
    except MergeProposalExists as e:
        raise PublishFailure(description=str(e), code="merge-proposal-exists") from e
    except GitLabConflict as e:
        raise PublishFailure(
            code="gitlab-conflict",
            description=(
                "Conflict during GitLab operation. " "Reached repository limit?"
            ),
        ) from e
    except SourceNotDerivedFromTarget as e:
        raise PublishFailure(
            code="source-not-derived-from-target",
            description=(
                "The source repository is not a fork of the " "target repository."
            ),
        ) from e
    except ProjectCreationTimeout as e:
        raise PublishFailure(
            code="project-creation-timeout",
            description="Forking the project (to %s) timed out (%ds)"
            % (e.project, e.timeout),
        ) from e
    except RemoteGitError as exc:
        raise PublishFailure(
            code="remote-git-error", description=f"remote git error: {exc}"
        ) from exc
    except InsufficientChangesForNewProposal as e:
        raise PublishNothingToDo("not enough changes for a new merge proposal") from e
    except BranchTemporarilyUnavailable as e:
        raise PublishFailure("branch-temporarily-unavailable", str(e)) from e
    except BranchUnavailable as e:
        raise PublishFailure("branch-unavailable", str(e)) from e


class DebdiffMissingRun(Exception):
    """Raised when the debdiff was missing a run."""

    def __init__(self, missing_run_id) -> None:
        self.missing_run_id = missing_run_id


class DifferUnavailable(Exception):
    """The differ was unavailable."""

    def __init__(self, reason) -> None:
        self.reason = reason


def get_debdiff(differ_url: str, unchanged_id: str, log_id: str) -> bytes:
    debdiff_url = urllib.parse.urljoin(
        differ_url, f"/debdiff/{unchanged_id}/{log_id}?filter_boring=1"
    )
    headers = {"Accept": "text/plain"}

    request = urllib.request.Request(debdiff_url, headers=headers)
    try:
        with urllib.request.urlopen(request) as f:
            return f.read()
    except urllib.error.HTTPError as e:
        if e.code == 404:
            if "unavailable_run_id" in e.headers:
                raise DebdiffMissingRun(e.headers["unavailable_run_id"]) from e
            raise
        elif e.code in (400, 500, 502, 503, 504):
            raise DebdiffRetrievalError(
                "Error %d: %s" % (e.code, e.file.read().decode("utf-8", "replace"))  # type: ignore
            ) from e
        else:
            raise
    except ConnectionResetError as e:
        raise DifferUnavailable(str(e)) from e
    except urllib.error.URLError as e:
        raise DebdiffRetrievalError(str(e)) from e


def _drop_env(args):
    while args and "=" in args[0]:
        args.pop(0)


def publish_one(
    template_env,
    campaign: str,
    command,
    codemod_result,
    target_branch_url: str,
    mode: str,
    role: str,
    revision: bytes,
    log_id: str,
    unchanged_id: str,
    source_branch_url: str,
    differ_url: str,
    derived_branch_name: str,
    require_binary_diff: bool = False,
    possible_forges: Optional[list[Forge]] = None,
    possible_transports: Optional[list[Transport]] = None,
    allow_create_proposal: bool = False,
    reviewers: Optional[list[str]] = None,
    result_tags: Optional[dict[str, bytes]] = None,
    commit_message_template: Optional[str] = None,
    title_template: Optional[str] = None,
    existing_mp_url: Optional[str] = None,
    extra_context: Optional[dict[str, Any]] = None,
) -> tuple[PublishResult, str]:
    args = shlex.split(command)
    _drop_env(args)

    with ExitStack() as es:
        try:
            source_branch = open_branch(
                source_branch_url, possible_transports=possible_transports
            )
        except BranchTemporarilyUnavailable as e:
            raise PublishFailure("local-branch-temporarily-unavailable", str(e)) from e
        except BranchUnavailable as e:
            raise PublishFailure("local-branch-unavailable", str(e)) from e
        except BranchMissing as e:
            raise PublishFailure("local-branch-missing", str(e)) from e

        if isinstance(source_branch, RemoteGitBranch):
            local_tree, destroy = create_temp_sprout(source_branch)
            es.callback(destroy)
            source_branch = local_tree.branch

        try:
            target_branch = open_branch(
                target_branch_url, possible_transports=possible_transports
            )
        except BranchRateLimited as e:
            raise PublishFailure("branch-rate-limited", str(e)) from e
        except BranchTemporarilyUnavailable as e:
            raise PublishFailure("branch-temporarily-unavailable", str(e)) from e
        except BranchUnavailable as e:
            raise PublishFailure("branch-unavailable", str(e)) from e
        except BranchMissing as e:
            raise PublishFailure("branch-missing", str(e)) from e

        try:
            if mode == MODE_BTS:
                raise NotImplementedError
            else:
                forge = get_forge(target_branch, possible_forges=possible_forges)
        except UnsupportedForge as e:
            if mode not in (MODE_PUSH, MODE_BUILD_ONLY):
                netloc = urllib.parse.urlparse(target_branch.user_url).netloc
                raise PublishFailure(
                    description=f"Forge unsupported: {netloc}.",
                    code="hoster-unsupported",
                ) from e
            # We can't figure out what branch to resume from when there's no forge
            # that can tell us.
            resume_branch = None
            existing_proposal = None
            if mode == MODE_PUSH:
                logging.warning(
                    "Unsupported forge (%s), will attempt to push to %s",
                    e,
                    full_branch_url(target_branch),
                )
            forge = None
        except ForgeLoginRequired as e:
            if mode not in (MODE_PUSH, MODE_BUILD_ONLY):
                netloc = urllib.parse.urlparse(target_branch.user_url).netloc
                raise PublishFailure(
                    description=f"Forge {netloc} supported but no login known.",
                    code="hoster-no-login",
                ) from e
            # We can't figure out what branch to resume from when there's no forge
            # that can tell us.
            resume_branch = None
            existing_proposal = None
            if mode == MODE_PUSH:
                logging.warning(
                    "No login for forge (%s), will attempt to push to %s",
                    e,
                    full_branch_url(target_branch),
                )
            forge = None
        except UnexpectedHttpStatus as e:
            if e.code == 502:
                raise PublishFailure("bad-gateway", str(e)) from e
            elif e.code == 429:
                raise PublishFailure("too-many-requests", str(e)) from e
            else:
                traceback.print_exc()
                raise PublishFailure(f"http-{e.code}", str(e)) from e
        else:
            if existing_mp_url is not None:
                try:
                    existing_proposal = forge.get_proposal_by_url(existing_mp_url)
                except UnsupportedForge as e:
                    raise PublishFailure("forge-mp-url-mismatch", str(e)) from e
                overwrite: Optional[bool] = True
                try:
                    resume_branch = open_branch(
                        existing_proposal.get_source_branch_url(),
                        possible_transports=possible_transports,
                    )
                except BranchRateLimited as e:
                    raise PublishFailure("resume-branch-rate-limited", str(e)) from e
                except BranchTemporarilyUnavailable as e:
                    raise PublishFailure(
                        "resume-branch-temporarily-unavailable", str(e)
                    ) from e
                except BranchUnavailable as e:
                    raise PublishFailure("resume-branch-unavailable", str(e)) from e
                except BranchMissing as e:
                    raise PublishFailure("resume-branch-missing", str(e)) from e
            else:
                try:
                    (
                        resume_branch,
                        overwrite,
                        existing_proposals,
                    ) = find_existing_proposed(
                        target_branch, forge, derived_branch_name
                    )
                except NoSuchProject as e:
                    if mode not in (MODE_PUSH, MODE_BUILD_ONLY):
                        raise PublishFailure(
                            description=f"Project {e.project} not found.",
                            code="project-not-found",
                        ) from e
                    resume_branch = None
                    existing_proposal = None
                except ForgeLoginRequired as e:
                    raise PublishFailure(
                        description=f"Forge {forge} supported but no login known.",
                        code="hoster-no-login",
                    ) from e
                except PermissionDenied as e:
                    raise PublishFailure(
                        description=(
                            f"Permission denied while finding existing proposal: {e.extra}"
                        ),
                        code="permission-denied",
                    ) from e
                else:
                    if existing_proposals and len(existing_proposals) > 1:
                        existing_proposal = existing_proposals[0]
                        logging.warning(
                            "Multiple existing proposals: %r. Using %r",
                            existing_proposals,
                            existing_proposal,
                        )
                    elif existing_proposals and len(existing_proposals) > 0:
                        existing_proposal = existing_proposals[0]
                    else:
                        existing_proposal = None

        debdiff: Optional[bytes]
        try:
            debdiff = get_debdiff(differ_url, unchanged_id, log_id)
        except DebdiffRetrievalError as e:
            raise PublishFailure(
                description=f"Error from differ for build diff: {e.reason}",
                code="differ-error",
            ) from e
        except DifferUnavailable as e:
            raise PublishFailure(
                description=f"Unable to contact differ for build diff: {e.reason}",
                code="differ-unreachable",
            ) from e
        except DebdiffMissingRun as e:
            if mode in (MODE_PROPOSE, MODE_ATTEMPT_PUSH) and require_binary_diff:
                if e.missing_run_id == log_id:
                    raise PublishFailure(
                        description=(
                            "Build diff is not available. "
                            f"Run ({log_id}) not yet published?"
                        ),
                        code="missing-build-diff-self",
                    ) from e
                else:
                    raise PublishFailure(
                        description=(
                            "Binary debdiff is not available. "
                            f"Control run ({e.missing_run_id}) not published?"
                        ),
                        code="missing-build-diff-control",
                    ) from e
            debdiff = None

        try:
            publish_result = publish(
                template_env=template_env,
                campaign=campaign,
                commit_message_template=commit_message_template,
                title_template=title_template,
                codemod_result=codemod_result,
                mode=mode,
                role=role,
                forge=forge,
                target_branch=target_branch,
                source_branch=source_branch,
                derived_branch_name=derived_branch_name,
                resume_branch=resume_branch,
                log_id=log_id,
                existing_proposal=existing_proposal,
                allow_create_proposal=allow_create_proposal,
                debdiff=debdiff,
                reviewers=reviewers,
                result_tags=result_tags,
                stop_revision=revision,
                extra_context=extra_context,
            )
        except EmptyMergeProposal as e:
            raise PublishFailure(
                code="empty-merge-proposal",
                description=(
                    "No changes to propose; " "changes made independently upstream?"
                ),
            ) from e
        except MergeConflict as e:
            raise PublishFailure(
                code="merge-conflict",
                description="merge would conflict (upstream changes?)",
            ) from e

    return publish_result, derived_branch_name


def load_template_env(path):
    env = Environment(
        loader=FileSystemLoader(path),
        trim_blocks=True,
        lstrip_blocks=True,
        autoescape=select_autoescape(disabled_extensions=("txt", "md"), default=False),
    )
    env.globals.update(
        {
            "debdiff_is_empty": debdiff_is_empty,
            "markdownify_debdiff": markdownify_debdiff,
            "parseaddr": parseaddr,
        }
    )
    return env


if __name__ == "__main__":
    import argparse
    import json
    import sys

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--template-env-path",
        type=str,
        default=os.path.join(os.path.dirname(__file__), "..", "proposal-templates"),
        help="Path to templates",
    )
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, stream=sys.stderr)

    request = json.load(sys.stdin)

    template_env = load_template_env(args.template_env_path)
    template_env.globals["external_url"] = (
        request["external_url"].rstrip("/") if request["external_url"] else None
    )

    try:
        publish_result, branch_name = publish_one(
            template_env,
            campaign=request["campaign"],
            derived_branch_name=request["derived_branch_name"],
            command=request["command"],
            codemod_result=request["codemod_result"],
            target_branch_url=request["target_branch_url"],
            mode=request["mode"],
            role=request["role"],
            log_id=request["log_id"],
            unchanged_id=request["unchanged_id"],
            source_branch_url=request["source_branch_url"],
            require_binary_diff=request["require-binary-diff"],
            possible_forges=None,
            possible_transports=None,
            allow_create_proposal=request["allow_create_proposal"],
            differ_url=request["differ_url"],
            reviewers=request.get("reviewers"),
            revision=request["revision"].encode("utf-8"),
            result_tags=request.get("tags"),
            commit_message_template=request.get("commit_message_template"),
            title_template=request.get("title_template"),
            existing_mp_url=request.get("existing_mp_url"),
            extra_context=request.get("extra_context"),
        )
    except PublishFailure as e:
        json.dump({"code": e.code, "description": e.description}, sys.stdout)
        sys.exit(1)
    except PublishNothingToDo as e:
        json.dump({"code": "nothing-to-do", "description": e.description}, sys.stdout)
        sys.exit(1)

    result = {}
    if publish_result.proposal:
        result["proposal_url"] = publish_result.proposal.url
        result["proposal_web_url"] = publish_result.proposal.get_web_url()
        result["is_new"] = publish_result.is_new
    result["branch_name"] = branch_name
    result["target_branch_url"] = publish_result.target_branch.user_url.rstrip("/")
    if publish_result.forge:
        result["target_branch_web_url"] = publish_result.forge.get_web_url(
            publish_result.target_branch
        )

    json.dump(result, sys.stdout)

    sys.exit(0)
