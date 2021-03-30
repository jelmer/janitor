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

"""Publisher for individual changes.

This is the worker module for the publish service. For each branch that needs
to be published, this module gets invoked. It accepts some JSON on stdin with a
request, and writes results to standard out as JSON.
"""

import os
from typing import Optional, List, Any, Dict

import logging

import urllib.error
import urllib.parse
import urllib.request

from silver_platter.utils import (
    open_branch,
    BranchMissing,
    BranchUnavailable,
    full_branch_url,
)
from silver_platter.publish import (
    EmptyMergeProposal,
    MergeProposal,
    get_hoster,
    merge_conflicts,
    find_existing_proposed,
    Hoster,
    NoSuchProject,
    PermissionDenied,
    UnsupportedHoster,
    SourceNotDerivedFromTarget,
    publish_changes,
    InsufficientChangesForNewProposal,
)

from breezy.branch import Branch
from breezy.errors import DivergedBranches
from breezy.plugins.gitlab.hoster import (
    ForkingDisabled,
    GitLabConflict,
    ProjectCreationTimeout,
)
from breezy.propose import (
    MergeProposalExists,
    HosterLoginRequired,
)

from jinja2 import Environment, FileSystemLoader, select_autoescape

from .debian.debdiff import (
    debdiff_is_empty,
    markdownify_debdiff,
)


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

template_env = Environment(
    loader=FileSystemLoader(os.path.join(os.path.dirname(__file__), '..', "proposal-templates")),
    autoescape=select_autoescape(disabled_extensions=('txt', 'md'), default=False),
)


class PublishFailure(Exception):
    def __init__(self, code, description):
        self.code = code
        self.description = description


class PublishNothingToDo(Exception):
    def __init__(self, description):
        self.description = description


class MergeConflict(Exception):
    def __init__(self, main_branch, local_branch):
        self.main_branch = main_branch
        self.local_branch = local_branch


class DebdiffRetrievalError(Exception):
    def __init__(self, reason):
        self.reason = reason


def publish(
    suite: str,
    pkg: str,
    subrunner: "Publisher",
    subworker_result: Any,
    mode: str,
    role: str,
    hoster: Hoster,
    main_branch: Branch,
    local_branch: Branch,
    external_url: str,
    derived_branch_name: str,
    resume_branch: Optional[Branch] = None,
    dry_run: bool = False,
    log_id: Optional[str] = None,
    existing_proposal: Optional[MergeProposal] = None,
    allow_create_proposal: bool = False,
    derived_owner: Optional[str] = None,
    debdiff: Optional[bytes] = None,
    reviewers: Optional[List[str]] = None,
    result_tags: Optional[Dict[str, bytes]] = None,
    stop_revision: Optional[bytes] = None,
):
    def get_proposal_description(description_format, existing_proposal):
        vs = {
            'package': pkg,
            'log_id': log_id,
            'suite': suite,
            'external_url': external_url,
            'debdiff_is_empty': debdiff_is_empty,
            'markdownify_debdiff': markdownify_debdiff,
            'role': role,
            }
        if subworker_result:
            vs.update(subworker_result)
        if debdiff:
            vs['debdiff'] = debdiff.decode("utf-8", "replace")
        if description_format == 'markdown':
            template = template_env.get_template(suite + '.md')
        else:
            template = template_env.get_template(suite + '.txt')
        return template.render(vs)

    def get_proposal_commit_message(existing_proposal):
        if existing_proposal:
            existing_commit_message = getattr(
                existing_proposal, "get_commit_message", lambda: None
            )()
        else:
            existing_commit_message = None
        return subrunner.get_proposal_commit_message(role, existing_commit_message)

    with main_branch.lock_read(), local_branch.lock_read():
        if merge_conflicts(main_branch, local_branch, stop_revision):
            raise MergeConflict(main_branch, local_branch)

    labels: Optional[List[str]]

    if hoster and hoster.supports_merge_proposal_labels:
        labels = [suite]
    else:
        labels = None
    try:
        return publish_changes(
            local_branch,
            main_branch,
            resume_branch,
            mode,
            derived_branch_name,
            get_proposal_description=get_proposal_description,
            get_proposal_commit_message=(get_proposal_commit_message),
            dry_run=dry_run,
            hoster=hoster,
            allow_create_proposal=allow_create_proposal,
            overwrite_existing=True,
            derived_owner=derived_owner,
            existing_proposal=existing_proposal,
            labels=labels,
            tags=result_tags,
            allow_collaboration=True,
            reviewers=reviewers,
            stop_revision=stop_revision,
        )
    except DivergedBranches:
        raise PublishFailure(
            description="Upstream branch has diverged from local changes.",
            code="diverged-branches",
        )
    except UnsupportedHoster:
        raise PublishFailure(
            description="Hoster unsupported: %s." % (main_branch.repository.user_url),
            code="hoster-unsupported",
        )
    except NoSuchProject as e:
        raise PublishFailure(
            description="project %s was not found" % e.project, code="project-not-found"
        )
    except ForkingDisabled:
        raise PublishFailure(
            description="Forking disabled: %s" % (main_branch.repository.user_url),
            code="forking-disabled",
        )
    except PermissionDenied as e:
        raise PublishFailure(description=str(e), code="permission-denied")
    except MergeProposalExists as e:
        raise PublishFailure(description=str(e), code="merge-proposal-exists")
    except GitLabConflict:
        raise PublishFailure(
            code="gitlab-conflict",
            description=(
                "Conflict during GitLab operation. " "Reached repository limit?"
            ),
        )
    except SourceNotDerivedFromTarget:
        raise PublishFailure(
            code="source-not-derived-from-target",
            description=(
                "The source repository is not a fork of the " "target repository."
            ),
        )
    except ProjectCreationTimeout as e:
        raise PublishFailure(
            code="project-creation-timeout",
            description="Forking the project (to %s) timed out (%ds)"
            % (e.project, e.timeout),
        )
    except InsufficientChangesForNewProposal:
        raise PublishNothingToDo('not enough changes for a new merge proposal')


class Publisher(object):
    def get_proposal_commit_message(
        self, role: str, format: str
    ) -> str:
        raise NotImplementedError(self.get_proposal_commit_message)

    def read_worker_result(self, result: Any) -> None:
        pass

    def allow_create_proposal(self) -> bool:
        raise NotImplementedError(self.allow_create_proposal)


class LintianBrushPublisher(Publisher):

    def get_proposal_commit_message(self, role, existing_commit_message):
        applied = []
        for result in self.applied:
            applied.append((result["fixed_lintian_tags"], result["summary"]))
        if existing_commit_message and not existing_commit_message.startswith(
            "Fix lintian issues: "
        ):
            # The commit message is something we haven't set - let's leave it
            # alone.
            return
        return "Fix lintian issues: " + (", ".join(sorted([l for r, l in applied])))

    def read_worker_result(self, result):
        self.applied = result["applied"]
        self.failed = result["failed"]
        self.add_on_only = result["add_on_only"]

    def allow_create_proposal(self):
        return self.applied and not self.add_on_only


class MultiArchHintsPublisher(Publisher):

    def get_proposal_commit_message(self, role, existing_commit_message):
        return "Apply multi-arch hints."

    def allow_create_proposal(self):
        return True


class OrphanPublisher(Publisher):

    # TODO(jelmer): Check that the wnpp bug is still open.

    def get_proposal_commit_message(self, role, existing_commit_message):
        return "Move package to the QA team."

    def allow_create_proposal(self):
        return True


class MIAPublisher(Publisher):

    def get_proposal_commit_message(self, role, existing_commit_message):
        return "Remove MIA uploaders."

    def allow_create_proposal(self):
        return True


class UncommittedPublisher(Publisher):

    def get_proposal_commit_message(self, role, existing_commit_message):
        return "Import archive changes missing from the VCS."

    def allow_create_proposal(self):
        return True


class ScrubObsoletePublisher(Publisher):
    def get_proposal_commit_message(self, role, existing_commit_message):
        return "Remove unnecessary constraints."

    def allow_create_proposal(self):
        return True


class NewUpstreamPublisher(Publisher):
    def read_worker_result(self, result):
        self._upstream_version = result["upstream_version"]

    def get_proposal_commit_message(self, role, existing_commit_message):
        if role == "pristine-tar":
            return "pristine-tar data for new upstream version %s." % (
                self._upstream_version
            )
        elif role == "upstream":
            return "Import of new upstream version %s." % (self._upstream_version)
        elif role == "main":
            return "Merge new upstream version %s." % self._upstream_version
        else:
            raise KeyError(role)

    def allow_create_proposal(self):
        # No upstream release too small...
        return True


class CMEPublisher(Publisher):
    def get_proposal_commit_message(self, role, existing_commit_message):
        return "Run CME fix."

    def allow_create_proposal(self):
        # CME doesn't provide enough information
        return True


class DebdiffMissingRun(Exception):
    """Raised when the debdiff was missing a run."""

    def __init__(self, missing_run_id):
        self.missing_run_id = missing_run_id


class DifferUnavailable(Exception):
    """The differ was unavailable."""

    def __init__(self, reason):
        self.reason = reason


def get_debdiff(differ_url: str, log_id: str) -> bytes:
    debdiff_url = urllib.parse.urljoin(
        differ_url, "/debdiff/BASE/%s?filter_boring=1" % log_id
    )
    headers = {"Accept": "text/plain"}

    request = urllib.request.Request(debdiff_url, headers=headers)
    try:
        with urllib.request.urlopen(request) as f:
            return f.read()
    except urllib.error.HTTPError as e:
        if e.code == 404:
            if "unavailable_run_id" in e.headers:
                raise DebdiffMissingRun(e.headers["unavailable_run_id"])
            raise
        elif e.code in (400, 500, 502, 503, 504):
            raise DebdiffRetrievalError(
                'Error %d: %s' % (e.code, e.file.read().decode("utf-8", "replace")))
        else:
            raise
    except ConnectionResetError as e:
        raise DifferUnavailable(str(e))
    except urllib.error.URLError as e:
        raise DebdiffRetrievalError(str(e))


def publish_one(
    suite,
    pkg,
    command,
    subworker_result,
    main_branch_url,
    mode,
    role,
    revision: bytes,
    log_id,
    local_branch_url,
    differ_url: str,
    external_url: str,
    derived_branch_name: str,
    dry_run=False,
    require_binary_diff=False,
    derived_owner=None,
    possible_hosters=None,
    possible_transports=None,
    allow_create_proposal=None,
    reviewers=None,
    result_tags=None,
):

    subrunner: Publisher
    if command.startswith("new-upstream"):
        subrunner = NewUpstreamPublisher()
    elif command.startswith("lintian-brush"):
        subrunner = LintianBrushPublisher()
    elif command.startswith("apply-multiarch-hints"):
        subrunner = MultiArchHintsPublisher()
    elif command.startswith("orphan"):
        subrunner = OrphanPublisher()
    elif command.startswith("import-upload"):
        subrunner = UncommittedPublisher()
    elif command.startswith("scrub-obsolete"):
        subrunner = ScrubObsoletePublisher()
    elif command.startswith("mia"):
        subrunner = MIAPublisher()
    elif command.startswith("cme-fix"):
        subrunner = CMEPublisher()
    else:
        raise AssertionError("unknown command %r" % command)

    try:
        local_branch = open_branch(
            local_branch_url, possible_transports=possible_transports
        )
    except BranchUnavailable as e:
        raise PublishFailure("local-branch-unavailable", str(e))
    except BranchMissing as e:
        raise PublishFailure("local-branch-missing", str(e))

    try:
        main_branch = open_branch(
            main_branch_url, possible_transports=possible_transports
        )
    except BranchUnavailable as e:
        raise PublishFailure("branch-unavailable", str(e))
    except BranchMissing as e:
        raise PublishFailure("branch-missing", str(e))

    subrunner.read_worker_result(subworker_result)

    try:
        if mode == MODE_BTS:
            from breezy.plugins.debian.bts import DebianBtsHoster

            hoster = DebianBtsHoster()
            mode = MODE_PROPOSE
        else:
            hoster = get_hoster(main_branch, possible_hosters=possible_hosters)
    except UnsupportedHoster as e:
        if mode not in (MODE_PUSH, MODE_BUILD_ONLY):
            netloc = urllib.parse.urlparse(main_branch.user_url).netloc
            raise PublishFailure(
                description="Hoster unsupported: %s." % netloc,
                code="hoster-unsupported",
            )
        # We can't figure out what branch to resume from when there's no hoster
        # that can tell us.
        resume_branch = None
        existing_proposal = None
        if mode == MODE_PUSH:
            logging.warning(
                "Unsupported hoster (%s), will attempt to push to %s",
                e,
                full_branch_url(main_branch),
            )
        hoster = None
    except HosterLoginRequired as e:
        if mode not in (MODE_PUSH, MODE_BUILD_ONLY):
            netloc = urllib.parse.urlparse(main_branch.user_url).netloc
            raise PublishFailure(
                description="Hoster %s supported but not login known." % netloc,
                code="hoster-no-login")
        # We can't figure out what branch to resume from when there's no hoster
        # that can tell us.
        resume_branch = None
        existing_proposal = None
        if mode == MODE_PUSH:
            logging.warning(
                "No login for hoster (%s), will attempt to push to %s",
                e, full_branch_url(main_branch),
            )
        hoster = None
    else:
        try:
            (resume_branch, overwrite, existing_proposal) = find_existing_proposed(
                main_branch, hoster, derived_branch_name, owner=derived_owner
            )
        except NoSuchProject as e:
            if mode not in (MODE_PUSH, MODE_BUILD_ONLY):
                raise PublishFailure(
                    description="Project %s not found." % e.project,
                    code="project-not-found",
                )
            resume_branch = None
            existing_proposal = None
        except PermissionDenied as e:
            raise PublishFailure(
                description=(
                    "Permission denied while finding existing proposal: %s" % e.extra
                ),
                code="permission-denied",
            )

    if allow_create_proposal is None:
        allow_create_proposal = subrunner.allow_create_proposal()

    debdiff: Optional[bytes]
    try:
        debdiff = get_debdiff(differ_url, log_id)
    except DebdiffRetrievalError as e:
        raise PublishFailure(
            description="Error from differ for build diff: %s" % e.reason,
            code="differ-error",
        )
    except DifferUnavailable as e:
        raise PublishFailure(
            description="Unable to contact differ for build diff: %s" % e.reason,
            code="differ-unreachable",
        )
    except DebdiffMissingRun as e:
        if mode in (MODE_PROPOSE, MODE_ATTEMPT_PUSH) and require_binary_diff:
            if e.missing_run_id == log_id:
                raise PublishFailure(
                    description=(
                        "Build diff is not available. "
                        "Run (%s) not yet published?" % log_id
                    ),
                    code="missing-build-diff-self",
                )
            else:
                raise PublishFailure(
                    description=(
                        "Binary debdiff is not available. "
                        "Control run (%s) not published?" % e.missing_run_id
                    ),
                    code="missing-build-diff-control",
                )
        debdiff = None

    try:
        publish_result = publish(
            suite,
            pkg,
            subrunner,
            subworker_result,
            mode,
            role,
            hoster,
            main_branch,
            local_branch,
            external_url,
            derived_branch_name,
            resume_branch,
            dry_run=dry_run,
            log_id=log_id,
            existing_proposal=existing_proposal,
            allow_create_proposal=allow_create_proposal,
            debdiff=debdiff,
            derived_owner=derived_owner,
            reviewers=reviewers,
            result_tags=result_tags,
            stop_revision=revision,
        )
    except EmptyMergeProposal:
        raise PublishFailure(
            code="empty-merge-proposal",
            description=(
                "No changes to propose; " "changes made independently upstream?"
            ),
        )
    except MergeConflict:
        raise PublishFailure(
            code="merge-conflict",
            description="merge would conflict (upstream changes?)",
        )

    return publish_result, derived_branch_name


if __name__ == "__main__":
    import argparse
    import json
    import os
    import sys

    parser = argparse.ArgumentParser()
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, stream=sys.stderr)

    request = json.load(sys.stdin)

    try:
        publish_result, branch_name = publish_one(
            suite=request["suite"],
            pkg=request["package"],
            derived_branch_name=request["derived_branch_name"],
            command=request["command"],
            subworker_result=request["subworker_result"],
            main_branch_url=request["main_branch_url"],
            mode=request["mode"],
            role=request["role"],
            log_id=request["log_id"],
            external_url=request["external_url"].rstrip("/"),
            local_branch_url=request["local_branch_url"],
            dry_run=request["dry-run"],
            derived_owner=request.get("derived-owner"),
            require_binary_diff=request["require-binary-diff"],
            possible_hosters=None,
            possible_transports=None,
            allow_create_proposal=request["allow_create_proposal"],
            differ_url=request["differ_url"],
            reviewers=request.get("reviewers"),
            revision=request["revision"].encode("utf-8"),
            result_tags=request.get("tags"),
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
        result["is_new"] = publish_result.is_new
    result["branch_name"] = branch_name

    json.dump(result, sys.stdout)

    sys.exit(0)
