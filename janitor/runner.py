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

from datetime import datetime
import json
import os
import shutil
import subprocess
import sys
import tempfile
import uuid

from debian.deb822 import Changes

from breezy.plugins.debian.util import (
    debsign,
    dget_changes,
    )

from prometheus_client import (
    Counter,
    Gauge,
    push_to_gateway,
    REGISTRY,
)

from silver_platter.debian.lintian import (
    create_mp_description,
    parse_mp_description,
    )
from silver_platter.proposal import (
    publish_changes,
    find_existing_proposed,
    enable_tag_pushing,
    push_changes,
    propose_changes,
    UnsupportedHoster,
    NoSuchProject,
    PermissionDenied,
    get_hoster,
    hosters,
    )
from silver_platter.utils import (
    open_branch,
    BranchUnavailable,
    )

from . import state
from .worker import process_package
from .trace import note, warning

open_proposal_count = Gauge(
    'open_proposal_count', 'Number of open proposals.',
    labelnames=('maintainer',))
packages_processed_count = Counter(
    'package_count', 'Number of packages processed.')
queue_length = Gauge(
    'queue_length', 'Number of items in the queue.')


JANITOR_BLURB = """
This merge proposal was created automatically by the Janitor bot
(https://janitor.debian.net/).

You can follow up to this merge proposal as you normally would.
"""


LOG_BLURB = """
Build and test logs for this branch can be found at
https://janitor.debian.net/pkg/%(package)s/%(log_id)s/.
"""


def strip_janitor_blurb(text):
    return text[:text.index(JANITOR_BLURB)]


def add_janitor_blurb(text, pkg, log_id):
    text += JANITOR_BLURB
    text += (LOG_BLURB % {'package': pkg, 'log_id': log_id})
    return text


class NoChangesFile(Exception):
    """No changes file found."""


class LintianBrushRunner(object):

    branch_name = "lintian-fixes"

    def get_proposal_description(self, existing_description):
        if existing_description:
            existing_lines = parse_mp_description(existing_description)
        else:
            existing_lines = []
        return create_mp_description(
            existing_lines + [l['summary'] for l in self.applied])

    def read_worker_result(self, result):
        self.applied = result['applied']
        self.failed = result['failed']
        self.add_on_only = result['add_on_only']

    def describe(self, result):
        tags = set()
        for brush_result, unused_summary in self.applied:
            tags.update(brush_result.fixed_lintian_tags)
        if result.proposal:
            if result.is_new:
                return 'Proposed fixes %r' % tags
            elif tags:
                return 'Updated proposal with fixes for %r' % tags
            else:
                return 'No new fixes for proposal'
        else:
            if tags:
                return 'Pushed fixes %r' % tags
            else:
                return 'Nothing to do.'

    def allow_create_proposal(self):
        return self.applied


class NewUpstreamRunner(object):

    branch_name = "new-upstream"

    def describe(self, result):
        if result.proposal:
            if result.is_new:
                return (
                    'Created merge proposal %s merging new '
                    'upstream version %s.' % (
                        result.proposal.url,
                        self._upstream_version))
            else:
                return 'Updated merge proposal %s for upstream version %s.' % (
                    result.proposal.url, self._upstream_version)
        return 'Did nothing.'

    def read_worker_result(self, result):
        self._upstream_version = result['upstream_version']

    def get_proposal_description(self, existing_description):
        return "New upstream version %s" % self._upstream_version

    def allow_create_proposal(self):
        # No upstream release too small...
        return True


class JanitorResult(object):

    def __init__(self, pkg, log_id, start_time, finish_time, description,
                 proposal=None, is_new=None, build_distribution=None,
                 build_version=None, changes_filename=None):
        self.package = pkg
        self.log_id = log_id
        self.start_time = start_time
        self.finish_time = finish_time
        self.description = description
        self.proposal = proposal
        self.is_new = is_new
        self.build_distribution = build_distribution
        self.build_version = build_version
        self.changes_filename = changes_filename


def find_changes(path, package):
    for name in os.listdir(path):
        if name.startswith('%s_' % package) and name.endswith('.changes'):
            break
    else:
        raise NoChangesFile(path, package)

    with open(os.path.join(path, name), 'r') as f:
        changes = Changes(f)
        return (name, changes["Version"], changes["Distribution"])


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
    return open_mps_per_maintainer


class Pending(object):
    """Workspace-like object that doesn't use working trees.

    TODO(jelmer): better name
    """

    def __init__(self, main_branch, local_branch, resume_branch=None):
        self.main_branch = main_branch
        self.local_branch = local_branch
        self.resume_branch = resume_branch
        self.orig_revid = (resume_branch or main_branch).last_revision()
        self.additional_colocated_branches = ['pristine-tar', 'upstream']

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
                overwrite_existing=None, labels=None, dry_run=False):
        if hoster is None:
            hoster = get_hoster(self.main_branch)
        return propose_changes(
            self.local_branch, self.main_branch,
            hoster=hoster, name=name, mp_description=description,
            resume_branch=self.resume_branch,
            resume_proposal=existing_proposal,
            overwrite_existing=overwrite_existing,
            labels=labels, dry_run=dry_run,
            additional_colocated_branches=self.additional_colocated_branches)

    def push(self, hoster=None, dry_run=False):
        if hoster is None:
            hoster = get_hoster(self.main_branch)
        return push_changes(
            self.local_branch, self.main_branch, hoster=hoster,
            additional_colocated_branches=self.additional_colocated_branches,
            dry_run=dry_run)


def invoke_worker(
        main_branch, env, command, output_directory, resume_branch=None,
        pre_check=None, post_check=None, build_command=None,
        possible_transports=None, possible_hosters=None):
    process_package(
        main_branch.user_url, env, command,
        resume_branch_url=(resume_branch.user_url if resume_branch else None),
        output_directory=output_directory,
        build_command=build_command,
        pre_check_command=pre_check,
        post_check_command=post_check,
        possible_transports=possible_transports,
        possible_hosters=possible_hosters)


def invoke_subprocess_worker(
        main_branch, env, command, output_directory, resume_branch=None,
        pre_check=None, post_check=None, build_command=None):
    subprocess_env = dict(os.environ.items())
    for k, v in env.items():
        if v is not None:
            subprocess_env[k] = v
    args = [sys.executable, '-m', 'janitor.worker',
            '--branch-url=%s' % main_branch.user_url,
            '--output-directory=%s' % output_directory]
    if resume_branch:
        args.append('--resume-branch-url=%s' % resume_branch.user_url)
    if pre_check:
        args.append('--pre-check=%s' % pre_check)
    if post_check:
        args.append('--post-check=%s' % post_check)
    if build_command:
        args.append('--build-command=%s' % build_command)

    args.extend(command)

    subprocess.check_call(args, env=subprocess_env)


def process_one(
        vcs_url, mode, env, command,
        max_mps_per_maintainer,
        build_command, open_mps_per_maintainer,
        refresh=False, pre_check=None, post_check=None,
        dry_run=False, incoming=None, log_dir=None,
        debsign_keyid=None, possible_transports=None, possible_hosters=None):
    maintainer_email = env['MAINTAINER_EMAIL']
    pkg = env['PACKAGE']
    if max_mps_per_maintainer and \
            open_mps_per_maintainer.get(maintainer_email, 0) \
            >= max_mps_per_maintainer:
        warning(
            'Skipping %s, maximum number of open merge proposals reached '
            'for maintainer %s', pkg, maintainer_email)
        return
    if mode == "attempt-push" and "salsa.debian.org/debian/" in vcs_url:
        # Make sure we don't accidentally push to unsuspecting collab-maint
        # repositories, even if debian-janitor becomes a member of "debian"
        # in the future.
        mode = "propose"
    packages_processed_count.inc()
    log_id = str(uuid.uuid4())
    start_time = datetime.now()

    if command[0] == "new-upstream":
        subrunner = NewUpstreamRunner()
    elif command[0] == "lintian-brush":
        subrunner = LintianBrushRunner()
    else:
        raise AssertionError('Unknown command %s' % command[0])

    try:
        main_branch = open_branch(
            vcs_url, possible_transports=possible_transports)
    except BranchUnavailable as e:
        return JanitorResult(
            pkg, log_id=log_id, start_time=start_time,
            finish_time=datetime.now(), description=str(e))
    try:
        hoster = get_hoster(main_branch, possible_hosters=possible_hosters)
    except UnsupportedHoster as e:
        if mode not in ('push', 'build-only'):
            return JanitorResult(
                pkg, log_id, start_time, datetime.now(), 'Hoster unsupported.')
        # We can't figure out what branch to resume from when there's no hoster
        # that can tell us.
        resume_branch = None
        existing_proposal = None
        warning('Unsupported hoster (%s), will attempt to push to %s',
                e, main_branch.user_url)
    else:
        (resume_branch, overwrite, existing_proposal) = (
            find_existing_proposed(main_branch, hoster, subrunner.branch_name))

    if refresh:
        resume_branch = None

    with tempfile.TemporaryDirectory() as output_directory:
        try:
            invoke_subprocess_worker(
                    main_branch, env, command, output_directory,
                    resume_branch=resume_branch, pre_check=pre_check,
                    post_check=post_check, build_command=build_command)
        except subprocess.CalledProcessError:
            return JanitorResult(
                pkg, log_id=log_id,
                start_time=start_time, finish_time=datetime.now(),
                description="Build failed")
        finally:
            src_build_log_path = os.path.join(output_directory, 'build.log')
            if os.path.exists(src_build_log_path):
                dest_build_log_path = os.path.join(
                    log_dir, pkg, log_id)
                os.makedirs(dest_build_log_path, exist_ok=True)
                shutil.copy(src_build_log_path, dest_build_log_path)

        result = JanitorResult(
            pkg, log_id=log_id,
            start_time=start_time, finish_time=datetime.now(),
            description="Built succeeded.")

        json_result_path = os.path.join(output_directory, 'result.json')
        if os.path.exists(json_result_path):
            with open(json_result_path, 'r') as f:
                subrunner.read_worker_result(json.load(f))

        try:
            (result.changes_filename, result.build_version,
             result.build_distribution) = find_changes(
                 output_directory, result.package)
        except NoChangesFile as e:
            # Oh, well.
            note('No changes file found: %s', e)

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

        if mode != 'build-only':
            try:
                local_branch = open_branch(os.path.join(output_directory, pkg))
            except BranchUnavailable as e:
                return JanitorResult(
                    pkg, log_id, start_time, datetime.now(),
                    'result branch missing: %s' % e)
            try:
                with Pending(main_branch, local_branch,
                             resume_branch=resume_branch) as ws:
                    enable_tag_pushing(local_branch)
                    (result.proposal, is_new) = publish_changes(
                        ws, mode, subrunner.branch_name,
                        get_proposal_description=get_proposal_description,
                        dry_run=dry_run, hoster=hoster,
                        allow_create_proposal=(
                            subrunner.allow_create_proposal()),
                        overwrite_existing=True,
                        existing_proposal=existing_proposal)
            except NoSuchProject as e:
                return JanitorResult(
                    pkg, log_id, start_time, datetime.now(),
                    'project %s was not found' % e.project)
            except PermissionDenied as e:
                return JanitorResult(
                    pkg, log_id, start_time, datetime.now(), str(e))

        if result.proposal and result.is_new:
            open_mps_per_maintainer.setdefault(maintainer_email, 0)
            open_mps_per_maintainer[maintainer_email] += 1
            open_proposal_count.labels(maintainer=maintainer_email).inc()

        if result.proposal:
            note('%s: %s: %s', result.package, result.description,
                 result.proposal.url)
        else:
            note('%s: %s', result.package, result.description)
        if result.changes_filename:
            changes_path = os.path.join(
                output_directory, result.changes_filename)
            debsign(changes_path, debsign_keyid)
            if incoming is not None:
                dget_changes(changes_path, incoming)

    return result


def process_queue(
        todo, max_mps_per_maintainer,
        build_command, open_mps_per_maintainer,
        refresh=False, pre_check=None, post_check=None,
        dry_run=False, incoming=None, output_directory=None):

    possible_transports = []
    possible_hosters = []

    for (vcs_url, mode, env, command) in todo:
        result = process_one(
            vcs_url, mode, env, command, max_mps_per_maintainer,
            build_command, open_mps_per_maintainer,
            refresh=refresh, pre_check=pre_check, post_check=post_check,
            dry_run=dry_run, incoming=incoming,
            output_directory=output_directory,
            possible_transports=possible_transports,
            possible_hosters=possible_hosters)
        state.store_run(
            result.log_id, env['PACKAGE'], vcs_url, env['MAINTAINER_EMAIL'],
            result.start_time, result.finish_time, command,
            result.description, result.proposal.url,
            build_version=result.build_version,
            build_distribution=result.build_distribution)


def main(argv=None):
    import argparse
    parser = argparse.ArgumentParser(prog='janitor.runner')
    parser.add_argument(
        '--prometheus', type=str,
        help='Prometheus push gateway to export to.')
    parser.add_argument(
        '--max-mps-per-maintainer',
        default=0,
        type=int,
        help='Maximum number of open merge proposals per maintainer.')
    parser.add_argument(
        '--refresh',
        help='Discard old branch and apply fixers from scratch.',
        action='store_true')
    parser.add_argument(
        '--pre-check',
        help='Command to run to check whether to process package.',
        type=str)
    parser.add_argument(
        '--post-check',
        help='Command to run to check package before pushing.',
        type=str)
    parser.add_argument(
        '--build-command',
        help='Build package to verify it.', type=str,
        default='sbuild -v')
    parser.add_argument(
        "--dry-run",
        help="Create branches but don't push or propose anything.",
        action="store_true", default=False)
    parser.add_argument(
        '--log-dir', help='Directory to store logs in.',
        type=str, default='site/pkg')
    parser.add_argument(
        '--incoming', type=str,
        help='Path to copy built Debian packages into.')
    parser.add_argument(
        '--debsign-keyid', type=str,
        help='GPG key to sign Debian package with.')

    args = parser.parse_args()

    last_success_gauge = Gauge(
        'job_last_success_unixtime',
        'Last time a batch job successfully finished')

    if args.max_mps_per_maintainer or args.prometheus:
        open_mps_per_maintainer = get_open_mps_per_maintainer()
        for maintainer_email, count in open_mps_per_maintainer.items():
            open_proposal_count.labels(maintainer=maintainer_email).inc(count)
    else:
        open_mps_per_maintainer = None

    for (queue_id, vcs_url, mode, env, command) in state.iter_queue():
        result = process_one(
            vcs_url, mode, env, command,
            max_mps_per_maintainer=args.max_mps_per_maintainer,
            open_mps_per_maintainer=open_mps_per_maintainer,
            refresh=args.refresh, pre_check=args.pre_check,
            build_command=args.build_command, post_check=args.post_check,
            dry_run=args.dry_run, incoming=args.incoming,
            debsign_keyid=args.debsign_keyid,
            log_dir=args.log_dir)
        state.store_run(
            result.log_id, env['PACKAGE'], vcs_url, env['MAINTAINER_EMAIL'],
            result.start_time, result.finish_time, command,
            result.description,
            result.proposal.url if result.proposal else None,
            build_version=result.build_version,
            build_distribution=result.build_distribution)

        state.drop_queue_item(queue_id)

        queue_length.set(state.queue_length())

        last_success_gauge.set_to_current_time()
        if args.prometheus:
            push_to_gateway(
                args.prometheus, job='janitor.runner', registry=REGISTRY)


if __name__ == '__main__':
    sys.exit(main(sys.argv))
