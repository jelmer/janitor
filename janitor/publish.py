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


from silver_platter.proposal import (
    publish_changes as publish_changes_from_workspace,
    propose_changes,
    push_changes,
    get_hoster,
    hosters,
    NoSuchProject,
    PermissionDenied,
    )

from prometheus_client import (
    Gauge,
    )

from . import state
from .trace import note, warning


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


open_proposal_count = Gauge(
    'open_proposal_count', 'Number of open proposals.',
    labelnames=('maintainer',))


def strip_janitor_blurb(text):
    return text[:text.index(JANITOR_BLURB)]


def add_janitor_blurb(text, pkg, log_id):
    text += JANITOR_BLURB
    text += (LOG_BLURB % {'package': pkg, 'log_id': log_id})
    return text


def get_open_mps_per_maintainer():
    """Retrieve the number of open merge proposals by maintainer.

    Returns:
      dictionary mapping maintainer emails to counts
    """
    # Don't put in the effort if we don't need the results.
    # Querying GitHub in particular is quite slow.
    open_proposals = []
    for name, hoster_cls in hosters.items():
        for instance in hoster_cls.iter_instances():
            note('Checking open merge proposals on %r...', instance)
            open_proposals.extend(instance.iter_my_proposals(status='open'))

    open_mps_per_maintainer = {}
    for proposal in open_proposals:
        maintainer_email = state.get_maintainer_email(proposal.url)
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

    def __init__(self, max_mps_per_maintainer=None):
        self._max_mps_per_maintainer = max_mps_per_maintainer
        self._open_mps_per_maintainer = get_open_mps_per_maintainer()

    def _check_limit(self, maintainer_email):
        return self._max_mps_per_maintainer and \
                self._open_mps_per_maintainer.get(maintainer_email, 0) \
                >= self._max_mps_per_maintainer

    def publish(self, pkg, maintainer_email, subrunner, mode, hoster,
                main_branch, local_branch, resume_branch=None,
                dry_run=False, log_id=None, existing_proposal=None):
        if self._check_limit(maintainer_email) and \
                mode in ('propose', 'attempt-push'):
            warning(
                'Not creating proposal for %s, maximum number of open merge '
                'proposals reached for maintainer %s', pkg, maintainer_email)
            if mode == 'propose':
                mode = 'build-only'
            if mode == 'attempt-push':
                mode = 'push'
        if mode == "attempt-push" and \
                "salsa.debian.org/debian/" in main_branch.user_url:
            # Make sure we don't accidentally push to unsuspecting collab-maint
            # repositories, even if debian-janitor becomes a member of "debian"
            # in the future.
            mode = "propose"

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

            if proposal and is_new:
                self._open_mps_per_maintainer.setdefault(maintainer_email, 0)
                self._open_mps_per_maintainer[maintainer_email] += 1
                open_proposal_count.labels(
                    maintainer=maintainer_email).inc()

        return proposal, is_new
