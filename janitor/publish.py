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

import asyncio
import os
import sys
import urllib.parse

from prometheus_client import (
    Gauge,
    push_to_gateway,
    REGISTRY,
)

from breezy.plugins.propose.propose import (
    MergeProposalExists,
    )

from silver_platter.proposal import (
    publish_changes as publish_changes_from_workspace,
    propose_changes,
    push_changes,
    find_existing_proposed,
    get_hoster,
    hosters,
    NoSuchProject,
    PermissionDenied,
    UnsupportedHoster,
    )
from silver_platter.debian.lintian import (
    create_mp_description,
    parse_mp_description,
    update_proposal_commit_message,
    )
from silver_platter.utils import (
    open_branch,
    BranchUnavailable,
    )

from . import state
from .policy import (
    read_policy,
    apply_policy,
    )
from .trace import note, warning
from .vcs import get_local_vcs_branch


JANITOR_BLURB = """
This merge proposal was created automatically by the Janitor bot
(https://janitor.debian.net/).

You can follow up to this merge proposal as you normally would.
"""


LOG_BLURB = """
Build and test logs for this branch can be found at
https://janitor.debian.net/pkg/%(package)s/%(log_id)s/.
"""


# TODO(jelmer): Dedupe this with janitor.runner.ADDITIONAL_COLOCATED_BRANCHES
ADDITIONAL_COLOCATED_BRANCHES = ['pristine-tar', 'upstream']

MODE_SKIP = 'skip'
MODE_BUILD_ONLY = 'build-only'
MODE_PUSH = 'push'
MODE_PUSH_DERIVED = 'push-derived'
MODE_PROPOSE = 'propose'
MODE_ATTEMPT_PUSH = 'attempt-push'
SUPPORTED_MODES = [
    MODE_PUSH,
    MODE_SKIP,
    MODE_BUILD_ONLY,
    MODE_PUSH_DERIVED,
    MODE_PROPOSE,
    MODE_ATTEMPT_PUSH,
    ]


open_proposal_count = Gauge(
    'open_proposal_count', 'Number of open proposals.',
    labelnames=('maintainer',))
merge_proposal_count = Gauge(
    'merge_proposal_count', 'Number of merge proposals by status.',
    labelnames=('status',))
last_success_gauge = Gauge(
    'job_last_success_unixtime',
    'Last time a batch job successfully finished')


def strip_janitor_blurb(text):
    return text[:text.index(JANITOR_BLURB)]


def add_janitor_blurb(text, pkg, log_id):
    text += JANITOR_BLURB
    text += (LOG_BLURB % {'package': pkg, 'log_id': log_id})
    return text


async def get_open_mps_per_maintainer():
    """Retrieve the number of open merge proposals by maintainer.

    Returns:
      dictionary mapping maintainer emails to counts
    """
    # Don't put in the effort if we don't need the results.
    # Querying GitHub in particular is quite slow.
    open_proposals = []
    for name, hoster_cls in hosters.items():
        for instance in hoster_cls.iter_instances():
            note('Checking merge proposals on %r...', instance)
            for status in ['open', 'merged', 'closed']:
                for mp in instance.iter_my_proposals(status=status):
                    await state.set_proposal_status(mp.url, status)
                    merge_proposal_count.labels(status=status).inc()
                    if status == 'open':
                        open_proposals.append(mp)

    open_mps_per_maintainer = {}
    for proposal in open_proposals:
        maintainer_email = await state.get_maintainer_email_for_proposal(
            proposal.url)
        if maintainer_email is None:
            warning('No maintainer email known for %s', proposal.url)
            continue
        open_mps_per_maintainer.setdefault(maintainer_email, 0)
        open_mps_per_maintainer[maintainer_email] += 1
        open_proposal_count.labels(maintainer=maintainer_email).inc()
    return open_mps_per_maintainer


class PublishFailure(Exception):

    def __init__(self, code, description):
        self.code = code
        self.description = description


class BranchWorkspace(object):
    """Workspace-like object that doesn't use working trees.
    """

    def __init__(self, main_branch, local_branch, resume_branch=None):
        self.main_branch = main_branch
        self.local_branch = local_branch
        self.resume_branch = resume_branch
        self.orig_revid = (resume_branch or main_branch).last_revision()
        self.additional_colocated_branches = ADDITIONAL_COLOCATED_BRANCHES

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
                commit_message=None):
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
            additional_colocated_branches=self.additional_colocated_branches)

    def push(self, hoster=None, dry_run=False):
        if hoster is None:
            hoster = get_hoster(self.main_branch)
        return push_changes(
            self.local_branch, self.main_branch, hoster=hoster,
            additional_colocated_branches=self.additional_colocated_branches,
            dry_run=dry_run)


class Publisher(object):
    """Publishes results made to a VCS, by pushing/proposing."""

    def __init__(self, max_mps_per_maintainer=None):
        self._max_mps_per_maintainer = max_mps_per_maintainer
        loop = asyncio.get_event_loop()
        loop.run_until_complete(self._refresh_open_mps_per_maintainer())

    async def _refresh_open_mps_per_maintainer(self):
        self._open_mps_per_maintainer = await get_open_mps_per_maintainer()

    def _check_limit(self, maintainer_email):
        return self._max_mps_per_maintainer and \
                self._open_mps_per_maintainer.get(maintainer_email, 0) \
                >= self._max_mps_per_maintainer

    async def publish(
            self, pkg, maintainer_email, subrunner, mode, hoster,
            main_branch, local_branch, resume_branch=None,
            dry_run=False, log_id=None, existing_proposal=None):
        if self._check_limit(maintainer_email) and \
                mode in (MODE_PROPOSE, MODE_ATTEMPT_PUSH):
            warning(
                'Not creating proposal for %s, maximum number of open merge '
                'proposals reached for maintainer %s', pkg, maintainer_email)
            if mode == MODE_PROPOSE:
                mode = MODE_BUILD_ONLY
            if mode == MODE_ATTEMPT_PUSH:
                mode = MODE_PUSH
        if mode == "attempt-push" and \
                "salsa.debian.org/debian/" in main_branch.user_url:
            # Make sure we don't accidentally push to unsuspecting collab-maint
            # repositories, even if debian-janitor becomes a member of "debian"
            # in the future.
            mode = MODE_PROPOSE

        def get_proposal_description(existing_proposal):
            if existing_proposal:
                existing_description = existing_proposal.get_description()
                existing_description = strip_janitor_blurb(
                    existing_description)
            else:
                existing_description = None
            description = subrunner.get_proposal_description(
                existing_description)
            return add_janitor_blurb(description, pkg, log_id)

        def get_proposal_commit_message(existing_proposal):
            if existing_proposal:
                existing_commit_message = (
                    getattr(existing_proposal, 'get_commit_message',
                            lambda: None)())
            else:
                existing_commit_message = None
            return subrunner.get_proposal_commit_message(
                existing_commit_message)

        with BranchWorkspace(
                main_branch, local_branch, resume_branch=resume_branch) as ws:
            try:
                (proposal, is_new) = publish_changes_from_workspace(
                    ws, mode, subrunner.branch_name(),
                    get_proposal_description=get_proposal_description,
                    get_proposal_commit_message=(
                        get_proposal_commit_message),
                    dry_run=dry_run, hoster=hoster,
                    allow_create_proposal=(
                        subrunner.allow_create_proposal()),
                    overwrite_existing=True,
                    existing_proposal=existing_proposal)
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

            if proposal and is_new:
                self._open_mps_per_maintainer.setdefault(maintainer_email, 0)
                self._open_mps_per_maintainer[maintainer_email] += 1
                merge_proposal_count.labels(status='open').inc()
                open_proposal_count.labels(
                    maintainer=maintainer_email).inc()

        return proposal, is_new


class LintianBrushPublisher(object):

    def __init__(self, args):
        self.args = args

    def branch_name(self):
        return "lintian-fixes"

    def get_proposal_description(self, existing_description):
        if existing_description:
            existing_lines = parse_mp_description(existing_description)
        else:
            existing_lines = []
        return create_mp_description(
            existing_lines + [l['summary'] for l in self.applied])

    def get_proposal_commit_message(self, existing_commit_message):
        applied = []
        for result in self.applied:
            applied.append((result['fixed_lintian_tags'], result['summary']))
        return update_proposal_commit_message(existing_commit_message, applied)

    def read_worker_result(self, result):
        self.applied = result['applied']
        self.failed = result['failed']
        self.add_on_only = result['add_on_only']

    def allow_create_proposal(self):
        return self.applied and not self.add_on_only


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

    def get_proposal_description(self, existing_description):
        return "New upstream version %s" % self._upstream_version

    def get_proposal_commit_message(self, existing_commit_message):
        return self.get_proposal_description(None)

    def allow_create_proposal(self):
        # No upstream release too small...
        return True


async def publish_one(
        pkg, publisher, command, subworker_result, main_branch_url,
        mode, log_id, maintainer_email, vcs_directory, branch_name,
        dry_run=False, possible_hosters=None,
        possible_transports=None):
    assert mode in SUPPORTED_MODES
    local_branch = get_local_vcs_branch(vcs_directory, pkg, branch_name)
    if local_branch is None:
        raise PublishFailure(
            'result-branch-not-found', 'can not find local branch')

    if command.startswith('new-upstream'):
        subrunner = NewUpstreamPublisher(command)
    elif command == 'lintian-brush':
        subrunner = LintianBrushPublisher(command)
    else:
        raise AssertionError('unknown command %r' % command)

    try:
        main_branch = open_branch(
            main_branch_url, possible_transports=possible_transports)
    except BranchUnavailable as e:
        raise PublishFailure('branch-unavailable', str(e))

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

    proposal, is_new = await publisher.publish(
        pkg, maintainer_email,
        subrunner, mode, hoster, main_branch, local_branch,
        resume_branch,
        dry_run=dry_run, log_id=log_id,
        existing_proposal=existing_proposal)

    return proposal, branch_name


async def publish_pending(publisher, policy, vcs_directory, dry_run=False):
    possible_hosters = []
    possible_transports = []

    for (pkg, command, build_version, result_code, context,
         start_time, log_id, revision,
         subworker_result, branch_name, maintainer_email, main_branch_url,
         main_branch_revision) in await state.iter_publish_ready():
        # TODO(jelmer): uploader_emails ??
        uploader_emails = []
        if command == 'new-upstream':
            policy_name = 'new_upstream_releases'
        elif command == 'lintian-brush':
            policy_name = 'lintian_brush'
        elif command == 'new-upstream --snapshot':
            policy_name = 'new_upstream_snapshots'
        else:
            raise AssertionError('unknown command %r' % command)

        mode, unused_update_changelog, unused_committer = apply_policy(
            policy, policy_name, pkg, maintainer_email,
            uploader_emails)
        if mode in (MODE_BUILD_ONLY, MODE_SKIP):
            continue
        if await state.already_published(
                pkg, branch_name, revision, mode):
            continue
        note('Publishing %s / %r (mode: %s)', pkg, command, mode)
        try:
            proposal, branch_name = await publish_one(
                pkg, publisher, command, subworker_result,
                main_branch_url, mode, log_id, maintainer_email,
                vcs_directory=vcs_directory, branch_name=branch_name,
                dry_run=dry_run, possible_hosters=possible_hosters,
                possible_transports=possible_transports)
        except PublishFailure as e:
            code = e.code
            description = e.description
            branch_name = None
            proposal = None
            note('Failed(%s): %s', code, description)
        else:
            code = 'success'
            description = 'Success'
        await state.store_publish(
            pkg, branch_name, main_branch_revision,
            revision, mode, code, description,
            proposal.url if proposal else None)


def main(argv=None):
    import argparse
    parser = argparse.ArgumentParser(prog='janitor.publish')
    parser.add_argument(
        '--max-mps-per-maintainer',
        default=0,
        type=int,
        help='Maximum number of open merge proposals per maintainer.')
    parser.add_argument(
        "--dry-run",
        help="Create branches but don't push or propose anything.",
        action="store_true", default=False)
    parser.add_argument(
        '--vcs-result-dir', type=str,
        help='Directory to store VCS repositories in.',
        default='vcs')
    parser.add_argument(
        "--policy",
        help="Policy file to read.", type=str,
        default='policy.conf')
    parser.add_argument(
        '--prometheus', type=str,
        help='Prometheus push gateway to export to.')

    args = parser.parse_args()

    with open(args.policy, 'r') as f:
        policy = read_policy(f)

    publisher = Publisher(args.max_mps_per_maintainer)

    loop = asyncio.get_event_loop()
    loop.run_until_complete(publish_pending(
        publisher, policy, dry_run=args.dry_run,
        vcs_directory=args.vcs_result_dir))

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        push_to_gateway(
            args.prometheus, job='janitor.publish',
            registry=REGISTRY)


if __name__ == '__main__':
    sys.exit(main(sys.argv))
