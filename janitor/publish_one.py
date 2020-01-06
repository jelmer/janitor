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

"""Publishing VCS changes."""

import urllib.error
import urllib.parse
import urllib.request

from silver_platter.utils import (
    open_branch,
    BranchMissing,
    BranchUnavailable,
    )
from silver_platter.proposal import (
    EmptyMergeProposal,
    get_hoster,
    merge_conflicts,
    publish_changes as publish_changes_from_workspace,
    propose_changes,
    push_changes,
    push_derived_changes,
    find_existing_proposed,
    NoSuchProject,
    PermissionDenied,
    UnsupportedHoster,
    )
from silver_platter.debian import (
    pick_additional_colocated_branches,
    )

from breezy.errors import DivergedBranches
from breezy.plugins.propose.propose import (
    MergeProposalExists,
    )

from .debdiff import (
    debdiff_is_empty,
    markdownify_debdiff,
    )
from .trace import warning


MODE_SKIP = 'skip'
MODE_BUILD_ONLY = 'build-only'
MODE_PUSH = 'push'
MODE_PUSH_DERIVED = 'push-derived'
MODE_PROPOSE = 'propose'
MODE_ATTEMPT_PUSH = 'attempt-push'

# Maximum number of lines of debdiff to inline in the merge request
# description. If this threshold is reached, we'll just include a link to the
# debdiff.
DEBDIFF_INLINE_THRESHOLD = 40


JANITOR_BLURB = """
This merge proposal was created automatically by the Janitor bot
(https://janitor.debian.net/%(suite)s).

You can follow up to this merge proposal as you normally would.
"""

JANITOR_BLURB_MD = """
This merge proposal was created automatically by the
[Janitor bot](https://janitor.debian.net/%(suite)s).

You can follow up to this merge proposal as you normally would.
"""

OLD_JANITOR_BLURB = """
This merge proposal was created automatically by the Janitor bot
(https://janitor.debian.net/).

You can follow up to this merge proposal as you normally would.
"""


LOG_BLURB = """
Build and test logs for this branch can be found at
https://janitor.debian.net/%(suite)s/pkg/%(package)s/%(log_id)s.
"""

LOG_BLURB_MD = """
Build and test logs for this branch can be found at
https://janitor.debian.net/%(suite)s/pkg/%(package)s/%(log_id)s.
"""

DEBDIFF_LINK_BLURB = """
These changes affect the binary packages. See the build logs page
or download the full debdiff from
https://janitor.debian.net/api/run/%(log_id)s/debdiff?filter_boring=1
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
[debdiff](https://janitor.debian.net/api/run/\
%(log_id)s/debdiff?filter_boring=1)
"""

NO_DEBDIFF_BLURB = """
These changes have no impact on the binary debdiff. See
https://janitor.debian.net/api/run/%(log_id)s/debdiff?filter_boring=1 to
download the raw debdiff.
"""

NO_DEBDIFF_BLURB_MD = """
These changes have no impact on the [binary debdiff](
https://janitor.debian.net/api/run/%(log_id)s/debdiff?filter_boring=1).
"""


class PublishFailure(Exception):

    def __init__(self, code, description):
        self.code = code
        self.description = description


class MergeConflict(Exception):

    def __init__(self, main_branch, local_branch):
        self.main_branch = main_branch
        self.local_branch = local_branch


def strip_janitor_blurb(text, suite):
    for blurb in [JANITOR_BLURB, OLD_JANITOR_BLURB, JANITOR_BLURB_MD]:
        try:
            i = text.index(blurb % {'suite': suite})
        except ValueError:
            pass
        else:
            return text[:i].strip()
    return text


def add_janitor_blurb(format, text, pkg, log_id, suite):
    text += '\n' + (
        (JANITOR_BLURB_MD if format == 'markdown' else JANITOR_BLURB) %
        {'suite': suite})
    text += (
        (LOG_BLURB_MD if format == 'markdown' else LOG_BLURB) %
        {'package': pkg, 'log_id': log_id, 'suite': suite})
    return text


def add_debdiff_blurb(format, text, pkg, log_id, suite, debdiff):
    if not debdiff_is_empty(debdiff):
        blurb = (
            NO_DEBDIFF_BLURB_MD if format == 'markdown' else NO_DEBDIFF_BLURB)
    elif len(debdiff.splitlines(False)) < DEBDIFF_INLINE_THRESHOLD:
        blurb = (
            DEBDIFF_BLURB_MD if format == 'markdown' else DEBDIFF_BLURB)
    else:
        blurb = (
            DEBDIFF_LINK_BLURB_MD
            if format == 'markdown' else DEBDIFF_LINK_BLURB)
    text += '\n' + (blurb % {
        'package': pkg,
        'log_id': log_id,
        'suite': suite,
        'debdiff': debdiff,
        'debdiff_md': markdownify_debdiff(debdiff)
        })
    return text


class BranchWorkspace(object):
    """Workspace-like object that doesn't use working trees.
    """

    def __init__(self, main_branch, local_branch, resume_branch=None,
                 push_colocated=None):
        self.main_branch = main_branch
        self.local_branch = local_branch
        self.resume_branch = resume_branch
        self.orig_revid = (resume_branch or main_branch).last_revision()
        self.additional_colocated_branches = (
            pick_additional_colocated_branches(main_branch))
        self.push_colocated = push_colocated

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        return False

    def changes_since_main(self):
        return self.local_branch.last_revision() \
               != self.main_branch.last_revision()

    def changes_since_resume(self):
        return self.orig_revid != self.local_branch.last_revision()

    def propose(self, name, description, hoster=None, existing_proposal=None,
                overwrite_existing=None, labels=None, dry_run=False,
                commit_message=None, reviewers=None):
        if hoster is None:
            hoster = get_hoster(self.main_branch)
        return propose_changes(
            self.local_branch, self.main_branch,
            hoster=hoster, name=name, mp_description=description,
            resume_branch=self.resume_branch,
            resume_proposal=existing_proposal,
            overwrite_existing=overwrite_existing,
            labels=labels, dry_run=dry_run,
            commit_message=commit_message,
            additional_colocated_branches=self.additional_colocated_branches,
            reviewers=reviewers)

    def push(self, hoster=None, dry_run=False):
        if hoster is None:
            hoster = get_hoster(self.main_branch)

        # Presumably breezy would do this check too, but we want to be *really*
        # sure.
        with self.local_branch.lock_read():
            graph = self.local_branch.repository.get_graph()
            if not graph.is_ancestor(
                    self.main_branch.last_revision(),
                    self.local_branch.last_revision()):
                raise DivergedBranches(self.main_branch, self.local_branch)

        if self.push_colocated:
            additional_colocated_branches = self.additional_colocated_branches
        else:
            additional_colocated_branches = []

        return push_changes(
            self.local_branch, self.main_branch, hoster=hoster,
            additional_colocated_branches=additional_colocated_branches,
            dry_run=dry_run)

    def push_derived(self, name, hoster=None, overwrite_existing=False):
        if hoster is None:
            hoster = get_hoster(self.main_branch)
        return push_derived_changes(
            self.local_branch,
            self.main_branch, hoster, name,
            overwrite_existing=overwrite_existing)


def publish(
        suite, pkg, subrunner, mode, hoster,
        main_branch, local_branch, resume_branch=None,
        dry_run=False, log_id=None, existing_proposal=None,
        allow_create_proposal=False, reviewers=None, debdiff=None):
    def get_proposal_description(description_format, existing_proposal):
        if existing_proposal:
            existing_description = existing_proposal.get_description()
            try:
                existing_description = strip_janitor_blurb(
                    existing_description, suite)
            except ValueError:
                # Oh, well...
                existing_description = None
        else:
            existing_description = None
        description = subrunner.get_proposal_description(
            description_format, existing_description)
        description = add_janitor_blurb(
            description_format, description, pkg, log_id, suite)
        if debdiff is not None:
            description = add_debdiff_blurb(
                description_format, description, pkg, log_id, suite,
                debdiff.decode('utf-8', 'replace'))
        return description

    def get_proposal_commit_message(existing_proposal):
        if existing_proposal:
            existing_commit_message = (
                getattr(existing_proposal, 'get_commit_message',
                        lambda: None)())
        else:
            existing_commit_message = None
        return subrunner.get_proposal_commit_message(
            existing_commit_message)

    with main_branch.lock_read(), local_branch.lock_read():
        if merge_conflicts(main_branch, local_branch):
            raise MergeConflict(main_branch, local_branch)

    with BranchWorkspace(
            main_branch, local_branch, resume_branch=resume_branch,
            push_colocated=subrunner.push_colocated()) as ws:
        if not hoster.supports_merge_proposal_labels:
            labels = None
        else:
            labels = [suite]
        try:
            return publish_changes_from_workspace(
                ws, mode, subrunner.branch_name(),
                get_proposal_description=get_proposal_description,
                get_proposal_commit_message=(
                    get_proposal_commit_message),
                dry_run=dry_run, hoster=hoster,
                allow_create_proposal=allow_create_proposal,
                overwrite_existing=True,
                existing_proposal=existing_proposal,
                labels=labels, reviewers=reviewers)
        except NoSuchProject as e:
            raise PublishFailure(
                description='project %s was not found' % e.project,
                code='project-not-found')
        except PermissionDenied as e:
            raise PublishFailure(
                description=str(e), code='permission-denied')
        except MergeProposalExists as e:
            raise PublishFailure(
                description=str(e), code='merge-proposal-exists')


class LintianBrushPublisher(object):

    def __init__(self, args):
        self.args = args

    def branch_name(self):
        return "lintian-fixes"

    def get_proposal_description(
            self, description_format, existing_description):
        from silver_platter.debian.lintian import (
            create_mp_description,
            applied_entry_as_line,
            )
        return create_mp_description(
            description_format, [
                applied_entry_as_line(
                    description_format, l.get('fixed_lintian_tags', []),
                    l['summary'])
                for l in self.applied])

    def get_proposal_commit_message(self, existing_commit_message):
        applied = []
        for result in self.applied:
            applied.append((result['fixed_lintian_tags'], result['summary']))
        if existing_commit_message and not existing_commit_message.startswith(
                'Fix lintian issues: '):
            # The commit message is something we haven't set - let's leave it
            # alone.
            return
        return "Fix lintian issues: " + (
            ', '.join(sorted([l for r, l in applied])))

    def read_worker_result(self, result):
        self.applied = result['applied']
        self.failed = result['failed']
        self.add_on_only = result['add_on_only']

    def allow_create_proposal(self):
        return self.applied and not self.add_on_only

    def push_colocated(self):
        return False


class MultiArchHintsPublisher(object):

    def __init__(self, args):
        self.args = args

    def branch_name(self):
        return "multiarch-hints"

    def get_proposal_description(self, format, existing_description):
        return 'Apply multi-arch hints.'

    def get_proposal_commit_message(self, existing_commit_message):
        return 'Apply multi-arch hints.'

    def read_worker_result(self, result):
        self.applied = result['applied-hints']

    def allow_create_proposal(self):
        return True

    def push_colocated(self):
        return False


class NewUpstreamPublisher(object):

    def __init__(self, args):
        self.args = args

    def branch_name(self):
        if '--snapshot' in self.args:
            return "new-upstream-snapshot"
        else:
            return "new-upstream"

    def read_worker_result(self, result):
        self._upstream_version = result['upstream_version']

    def get_proposal_description(self, format, existing_description):
        return "New upstream version %s.\n" % self._upstream_version

    def get_proposal_commit_message(self, existing_commit_message):
        return self.get_proposal_description('text', None)

    def allow_create_proposal(self):
        # No upstream release too small...
        return True

    def push_colocated(self):
        return True


def publish_one(
        suite, pkg, command, subworker_result, main_branch_url,
        mode, log_id, local_branch_url,
        dry_run=False, reviewers=None, require_binary_diff=False,
        possible_hosters=None,
        possible_transports=None, allow_create_proposal=None):

    if command.startswith('new-upstream'):
        subrunner = NewUpstreamPublisher(command)
    elif command.startswith('lintian-brush'):
        subrunner = LintianBrushPublisher(command)
    elif command.startswith('apply-multiarch-hints'):
        subrunner = MultiArchHintsPublisher(command)
    else:
        raise AssertionError('unknown command %r' % command)

    try:
        local_branch = open_branch(
            local_branch_url, possible_transports=possible_transports)
    except BranchUnavailable as e:
        raise PublishFailure('local-branch-unavailable', str(e))
    except BranchMissing as e:
        raise PublishFailure('local-branch-missing', str(e))

    try:
        main_branch = open_branch(
            main_branch_url, possible_transports=possible_transports)
    except BranchUnavailable as e:
        raise PublishFailure('branch-unavailable', str(e))
    except BranchMissing as e:
        raise PublishFailure('branch-missing', str(e))

    subrunner.read_worker_result(subworker_result)
    branch_name = subrunner.branch_name()

    try:
        hoster = get_hoster(main_branch, possible_hosters=possible_hosters)
    except UnsupportedHoster as e:
        if mode not in (MODE_PUSH, MODE_BUILD_ONLY):
            netloc = urllib.parse.urlparse(main_branch.user_url).netloc
            raise PublishFailure(
                description='Hoster unsupported: %s.' % netloc,
                code='hoster-unsupported')
        # We can't figure out what branch to resume from when there's no hoster
        # that can tell us.
        resume_branch = None
        existing_proposal = None
        if mode == MODE_PUSH:
            warning('Unsupported hoster (%s), will attempt to push to %s',
                    e, main_branch.user_url)
        hoster = None
    else:
        try:
            (resume_branch, overwrite, existing_proposal) = (
                find_existing_proposed(
                    main_branch, hoster, branch_name))
        except NoSuchProject as e:
            if mode not in (MODE_PUSH, MODE_BUILD_ONLY):
                raise PublishFailure(
                    description='Project %s not found.' % e.project,
                    code='project-not-found')
            resume_branch = None
            existing_proposal = None
        except PermissionDenied as e:
            raise PublishFailure(
                description=(
                    'Permission denied while finding existing proposal: %s' %
                    e.extra),
                code='permission-denied')

    if allow_create_proposal is None:
        allow_create_proposal = subrunner.allow_create_proposal()

    debdiff_url = (
        'https://janitor.debian.net/api/run/%s/debdiff?filter_boring=1'
        % log_id)
    headers = {'Accept': 'text/plain'}

    try:
        with urllib.request.urlopen(debdiff_url, headers=headers) as f:
            debdiff = f.read()
    except urllib.error.HTTPError as e:
        if e.status == 404:
            debdiff = None
        else:
            raise

    if debdiff is None and require_binary_diff:
        raise PublishFailure(
            description='Binary debdiff is not available. No control build?',
            code='missing-binary-diff')

    try:
        publish_result = publish(
            suite, pkg, subrunner, mode, hoster, main_branch, local_branch,
            resume_branch, reviewers=reviewers,
            dry_run=dry_run, log_id=log_id,
            existing_proposal=existing_proposal,
            allow_create_proposal=allow_create_proposal,
            debdiff=debdiff)
    except EmptyMergeProposal:
        raise PublishFailure(
            code='empty-merge-proposal',
            description=(
                'No changes to propose; '
                'changes made independently upstream?'))
    except MergeConflict:
        raise PublishFailure(
            code='merge-conflict',
            description='merge would conflict (upstream changes?)')

    return publish_result, branch_name


if __name__ == '__main__':
    import argparse
    import json
    import sys
    parser = argparse.ArgumentParser()
    args = parser.parse_args()

    request = json.load(sys.stdin)

    try:
        publish_result, branch_name = publish_one(
            request['suite'], request['package'],
            request['command'], request['subworker_result'],
            request['main_branch_url'], request['mode'], request['log_id'],
            request['local_branch_url'], request['dry-run'],
            request['reviewers'], request['require-binary-diff'],
            possible_hosters=None, possible_transports=None,
            allow_create_proposal=request['allow_create_proposal'])
    except PublishFailure as e:
        json.dump({'code': e.code, 'description': e.description}, sys.stdout)
        sys.exit(1)

    result = {}
    if publish_result.proposal:
        result['proposal_url'] = publish_result.proposal.url
        result['is_new'] = publish_result.is_new
    result['branch_name'] = branch_name

    json.dump(result, sys.stdout)

    sys.exit(0)
