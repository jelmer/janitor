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
)

from .debdiff import (
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

# Maximum number of lines of debdiff to inline in the merge request
# description. If this threshold is reached, we'll just include a link to the
# debdiff.
DEBDIFF_INLINE_THRESHOLD = 40


JANITOR_BLURB = """
This merge proposal was created automatically by the Janitor bot.
For more information, including instructions on how to disable
these merge proposals, see %(external_url)s/%(suite)s.

You can follow up to this merge proposal as you normally would.

The bot will automatically update the merge proposal to resolve merge conflicts
or close the merge proposal when all changes are applied through other means
(e.g. cherry-picks). Updates may take several hours to propagate.
"""

JANITOR_BLURB_MD = """
This merge proposal was created automatically by the \
[Janitor bot](%(external_url)s/%(suite)s).
For more information, including instructions on how to disable
these merge proposals, see %(external_url)s/%(suite)s.

You can follow up to this merge proposal as you normally would.

The bot will automatically update the merge proposal to resolve merge conflicts
or close the merge proposal when all changes are applied through other means
(e.g. cherry-picks). Updates may take several hours to propagate.
"""

LOG_BLURB = """
Build and test logs for this branch can be found at
%(external_url)s/%(suite)s/pkg/%(package)s/%(log_id)s.
"""

LOG_BLURB_MD = """
Build and test logs for this branch can be found at
%(external_url)s/%(suite)s/pkg/%(package)s/%(log_id)s.
"""

DEBDIFF_LINK_BLURB = """
These changes affect the binary packages. See the build logs page
or download the full debdiff from
%(external_url)s/api/run/%(log_id)s/debdiff?filter_boring=1
"""

DEBDIFF_BLURB_MD = """
## Debdiff

These changes affect the binary packages:

%(debdiff_md)s
"""

DEBDIFF_BLURB = """
These changes affect the binary packages:

%(debdiff)s
"""

DEBDIFF_LINK_BLURB_MD = """
These changes affect the binary packages; see the
[debdiff](%(external_url)s/api/run/\
%(log_id)s/debdiff?filter_boring=1)
"""

NO_DEBDIFF_BLURB = """
These changes have no impact on the binary debdiff. See
%(external_url)s/api/run/%(log_id)s/debdiff?filter_boring=1 to
download the raw debdiff.
"""

NO_DEBDIFF_BLURB_MD = """
These changes have no impact on the [binary debdiff](
%(external_url)s/api/run/%(log_id)s/debdiff?filter_boring=1).
"""

DIFFOSCOPE_LINK_BLURB_MD = """
You can also view the [diffoscope diff](\
%(external_url)s/api/run/%(log_id)s/diffoscope?filter_boring=1) \
([unfiltered](%(external_url)s/api/run/%(log_id)s/diffoscope)).
"""

DIFFOSCOPE_LINK_BLURB = """
You can also view the diffoscope diff at
%(external_url)s/api/run/%(log_id)s/diffoscope?filter_boring=1,
or unfiltered at %(external_url)s/api/run/%(log_id)s/diffoscope.
"""


class PublishFailure(Exception):
    def __init__(self, code, description):
        self.code = code
        self.description = description


class MergeConflict(Exception):
    def __init__(self, main_branch, local_branch):
        self.main_branch = main_branch
        self.local_branch = local_branch


class DebdiffRetrievalError(Exception):
    def __init__(self, reason):
        self.reason = reason


def strip_janitor_blurb(text, suite, external_url):
    for blurb in [JANITOR_BLURB, JANITOR_BLURB_MD]:
        try:
            i = text.index(blurb % {"suite": suite, "external_url": external_url})
        except ValueError:
            pass
        else:
            return text[:i].strip()
    raise ValueError


def add_janitor_blurb(format, text, pkg, log_id, suite, external_url):
    text += "\n" + (
        (JANITOR_BLURB_MD if format == "markdown" else JANITOR_BLURB)
        % {"suite": suite, "external_url": external_url}
    )
    text += (LOG_BLURB_MD if format == "markdown" else LOG_BLURB) % {
        "package": pkg,
        "log_id": log_id,
        "suite": suite,
        "external_url": external_url,
    }
    return text


def add_debdiff_blurb(format, text, pkg, log_id, suite, debdiff, external_url):
    if not debdiff_is_empty(debdiff):
        blurb = NO_DEBDIFF_BLURB_MD if format == "markdown" else NO_DEBDIFF_BLURB
    elif len(debdiff.splitlines(False)) < DEBDIFF_INLINE_THRESHOLD:
        blurb = DEBDIFF_BLURB_MD if format == "markdown" else DEBDIFF_BLURB
    else:
        blurb = DEBDIFF_LINK_BLURB_MD if format == "markdown" else DEBDIFF_LINK_BLURB
    text += "\n" + (
        blurb
        % {
            "package": pkg,
            "log_id": log_id,
            "suite": suite,
            "debdiff": debdiff,
            "debdiff_md": markdownify_debdiff(debdiff),
            "external_url": external_url,
        }
    )
    return text


def add_diffoscope_blurb(format, text, pkg, log_id, suite, external_url):
    blurb = DIFFOSCOPE_LINK_BLURB_MD if format == "markdown" else DIFFOSCOPE_LINK_BLURB
    text += "\n" + (
        blurb
        % {
            "package": pkg,
            "log_id": log_id,
            "suite": suite,
            "external_url": external_url,
        }
    )
    return text


def publish(
    suite: str,
    pkg: str,
    subrunner: "Publisher",
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
        if existing_proposal:
            existing_description = existing_proposal.get_description()
            try:
                existing_description = strip_janitor_blurb(
                    existing_description, suite, external_url
                )
            except ValueError:
                # Oh, well...
                existing_description = None
        else:
            existing_description = None
        description = subrunner.get_proposal_description(
            role, description_format, existing_description
        )
        description = add_janitor_blurb(
            description_format, description, pkg, log_id, suite, external_url
        )
        if debdiff is not None and role == "main":
            description = add_debdiff_blurb(
                description_format,
                description,
                pkg,
                log_id,
                suite,
                debdiff.decode("utf-8", "replace"),
                external_url,
            )
            description = add_diffoscope_blurb(
                description_format, description, pkg, log_id, suite, external_url
            )
        return description

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


class Publisher(object):
    def get_proposal_description(
        self, role: str, description_format: str, existing_description: Optional[str]
    ) -> str:
        raise NotImplementedError(self.get_proposal_description)

    def read_worker_result(self, result: Any) -> None:
        raise NotImplementedError(self.read_worker_result)

    def allow_create_proposal(self) -> bool:
        raise NotImplementedError(self.allow_create_proposal)


class LintianBrushPublisher(Publisher):
    def get_proposal_description(self, role, description_format, existing_description):
        from silver_platter.debian.lintian import (
            create_mp_description,
            applied_entry_as_line,
        )

        return create_mp_description(
            description_format,
            [
                applied_entry_as_line(
                    description_format,
                    line.get("fixed_lintian_tags", []),
                    line["summary"],
                )
                for line in self.applied
            ],
        )

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
    def get_proposal_description(self, role, format, existing_description):
        text = "Apply hints suggested by the multi-arch hinter.\n\n"
        for entry in self.applied:
            kind = entry["link"].split("#")[-1]
            if format == "markdown":
                text += "* %s: " % entry["binary"]
                if "action" in entry:
                    text += entry["action"] + " "
                    text += "This fixes: %s. ([%s](%s))" % (
                        entry["description"],
                        kind,
                        entry["link"],
                    )
                else:
                    text += "Fix: %s. ([%s](%s))" % (
                        entry["description"],
                        kind,
                        entry["link"],
                    )

                text += "\n"
            else:
                text += "* %s: " % entry["binary"]
                if "action" in entry:
                    text += "%s. This fixes: %s (%s).\n" % (
                        entry["action"],
                        entry["description"],
                        kind,
                    )
                else:
                    text += "Fix: %s (%s)\n" % (entry["description"], kind)

        text += """
These changes were suggested on https://wiki.debian.org/MultiArch/Hints.
"""

        return text

    def get_proposal_commit_message(self, role, existing_commit_message):
        return "Apply multi-arch hints."

    def read_worker_result(self, result):
        self.applied = result["applied-hints"]

    def allow_create_proposal(self):
        return True


class OrphanPublisher(Publisher):

    # TODO(jelmer): Check that the wnpp bug is still open.

    def get_proposal_description(self, role, format, existing_description):
        from silver_platter.debian.orphan import move_instructions

        text = "Move orphaned package to the QA team.\n\n"
        if self.wnpp_bug:
            if format == "markdown":
                text += (
                    "For details, see the [orphan bug](https://bugs.debian.org/%d).\n\n"
                    % self.wnpp_bug
                )
            else:
                text += (
                    "For details, see the orphan bug at https://bugs.debian.org/%d.\n\n"
                    % self.wnpp_bug
                )
        if not self.pushed and self.new_vcs_url:
            text += "\n".join(
                move_instructions(
                    self.package_name,
                    self.salsa_user,
                    self.old_vcs_url,
                    self.new_vcs_url,
                )
            )
        return text

    def get_proposal_commit_message(self, role, existing_commit_message):
        return "Move package to the QA team."

    def read_worker_result(self, result):
        self.wnpp_bug = result.get("wnpp_bug")
        self.pushed = result["pushed"]
        self.old_vcs_url = result["old_vcs_url"]
        self.new_vcs_url = result["new_vcs_url"]
        try:
            self.package_name = result["package_name"]
            self.salsa_user = result["salsa_user"]
        except KeyError:
            if self.new_vcs_url is not None:
                self.salsa_user, self.package_name = (
                    urllib.parse.urlparse(self.new_vcs_url).path.strip("/").split("/")
                )
            else:
                self.salsa_user = None
                self.package_name = None

    def allow_create_proposal(self):
        return True


class MIAPublisher(Publisher):

    def get_proposal_description(self, role, format, existing_description):
        text = "Remove MIA uploaders:\n\n"
        for uploader in self.uploaders:
            text += " * %s\n" % uploader
        return text

    def get_proposal_commit_message(self, role, existing_commit_message):
        return "Remove MIA uploaders."

    def read_worker_result(self, result):
        self.uploaders = result.get("removed_uploaders")

    def allow_create_proposal(self):
        return True


class UncommittedPublisher(Publisher):
    def get_proposal_description(self, role, format, existing_description):
        return "Import archive changes missing from the VCS."

    def get_proposal_commit_message(self, role, existing_commit_message):
        return "Import archive changes missing from the VCS."

    def read_worker_result(self, result):
        self.tags = result["tags"]

    def allow_create_proposal(self):
        return True


class ScrubObsoletePublisher(Publisher):
    def get_proposal_description(self, role, format, existing_description):
        return "Remove unnecessary constraints."

    def get_proposal_commit_message(self, role, existing_commit_message):
        return "Remove unnecessary constraints."

    def read_worker_result(self, result):
        pass

    def allow_create_proposal(self):
        return True


class NewUpstreamPublisher(Publisher):
    def read_worker_result(self, result):
        self._upstream_version = result["upstream_version"]

    def get_proposal_description(self, role, format, existing_description):
        if role == "pristine-tar":
            return "pristine-tar data for new upstream version %s.\n" % (
                self._upstream_version
            )
        elif role == "upstream":
            return "Import of new upstream version %s.\n" % (self._upstream_version)
        elif role == "main":
            return "Merge new upstream version %s.\n" % self._upstream_version
        else:
            raise KeyError(role)

    def get_proposal_commit_message(self, role, existing_commit_message):
        return self.get_proposal_description(role, "text", None)

    def allow_create_proposal(self):
        # No upstream release too small...
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
        elif e.code in (400, 502, 503, 504):
            raise DebdiffRetrievalError(e.file.read().decode("utf-8", "replace"))
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
    import sys

    parser = argparse.ArgumentParser()
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO)

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

    result = {}
    if publish_result.proposal:
        result["proposal_url"] = publish_result.proposal.url
        result["is_new"] = publish_result.is_new
    result["branch_name"] = branch_name

    json.dump(result, sys.stdout)

    sys.exit(0)
