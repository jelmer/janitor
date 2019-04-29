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

from aiohttp import web
import asyncio
from datetime import datetime
import json
import os
import shutil
import sys
import tempfile
import uuid

from debian.deb822 import Changes

from breezy.branch import Branch
from breezy.controldir import ControlDir, format_registry
from breezy.errors import NotBranchError
from breezy.plugins.debian.util import (
    debsign,
    dget_changes,
    )

from prometheus_client import (
    Counter,
    Gauge,
    generate_latest,
    CONTENT_TYPE_LATEST,
)

from silver_platter.debian.lintian import (
    create_mp_description,
    parse_mp_description,
    update_proposal_commit_message,
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
from .trace import note, warning

open_proposal_count = Gauge(
    'open_proposal_count', 'Number of open proposals.',
    labelnames=('maintainer',))
packages_processed_count = Counter(
    'package_count', 'Number of packages processed.')
queue_length = Gauge(
    'queue_length', 'Number of items in the queue.')
last_success_gauge = Gauge(
    'job_last_success_unixtime',
    'Last time a batch job successfully finished')


ADDITIONAL_COLOCATED_BRANCHES = ['pristine-tar', 'upstream']

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

    def get_proposal_commit_message(self, existing_commit_message):
        fixed_tags = set()
        for result in self.applied:
            fixed_tags.update(result['fixed_lintian_tags'])
        return update_proposal_commit_message(
            existing_commit_message, fixed_tags)

    def read_worker_result(self, result):
        self.applied = result['applied']
        self.failed = result['failed']
        self.add_on_only = result['add_on_only']

    def describe(self, result):
        # TODO(jelmer): This method is unused
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
        return self.applied and not self.add_on_only


class NewUpstreamRunner(object):

    branch_name = "new-upstream"

    def describe(self, result):
        # TODO(jelmer): This method is unused
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

    def get_proposal_commit_message(self, existing_commit_message):
        return self.get_proposal_description(None)

    def allow_create_proposal(self):
        # No upstream release too small...
        return True


class JanitorResult(object):

    def __init__(self, pkg, log_id, description=None,
                 code=None, proposal=None, is_new=None,
                 build_distribution=None, build_version=None,
                 changes_filename=None, worker_result=None):
        self.package = pkg
        self.log_id = log_id
        self.description = description
        self.code = code
        self.proposal = proposal
        self.is_new = is_new
        self.build_distribution = build_distribution
        self.build_version = build_version
        self.changes_filename = changes_filename
        if worker_result:
            self.context = worker_result.context
            if self.code is None:
                self.code = worker_result.code
            if self.description is None:
                self.description = worker_result.description
            self.main_branch_revision = worker_result.main_branch_revision
        else:
            self.context = None
            self.main_branch_revision = None


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


class WorkerResult(object):

    def __init__(self, code, description, context=None, subworker=None,
                 main_branch_revision=None):
        self.code = code
        self.description = description
        self.context = context
        self.subworker = subworker
        self.main_branch_revision = main_branch_revision

    @classmethod
    def from_file(cls, path):
        with open(path, 'r') as f:
            worker_result = json.load(f)
        return cls(
                worker_result.get('code'), worker_result.get('description'),
                worker_result.get('context'), worker_result.get('subworker'),
                worker_result.get('main_branch_revision'))


async def invoke_subprocess_worker(
        main_branch, env, command, output_directory, resume_branch=None,
        pre_check=None, post_check=None, build_command=None,
        log_path=None):
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

    if log_path:
        read, write = os.pipe()
        p = await asyncio.create_subprocess_exec(
            *args, env=subprocess_env, stdout=write, stderr=write)
        os.close(write)
        tee = await asyncio.create_subprocess_exec('tee', log_path, stdin=read)
        os.close(read)
        await tee.wait()
        return await p.wait()
    else:
        p = await asyncio.create_subprocess_exec(
            *args, env=subprocess_env)
        return await p.wait()


def publish_vcs_dir(ws, vcs_result_dir, pkg, name,
                    additional_colocated_branches=None):
    """Publish resulting changes in VCS form.

    This creates a repository with the following branches:
     * master - the original Debian packaging branch
     * KIND - whatever command was run
     * upstream - the upstream branch (optional)
     * pristine-tar the pristine tar packaging branch (optional)
    """
    if getattr(ws.main_branch.repository, '_git', None):
        path = os.path.join(vcs_result_dir, 'git', pkg)
        os.makedirs(path, exist_ok=True)
        try:
            vcs_result_controldir = ControlDir.open(path)
        except NotBranchError:
            vcs_result_controldir = ControlDir.create(
                path, format=format_registry.get('git-bare')())
        try:
            target_branch = vcs_result_controldir.open_branch(name='msater')
        except NotBranchError:
            target_branch = vcs_result_controldir.create_branch(name='master')
        for (from_branch, target_branch_name) in [
                (ws.main_branch, 'master'),
                (ws.local_branch, name)]:
            try:
                target_branch = vcs_result_controldir.open_branch(
                    name=target_branch_name)
            except NotBranchError:
                target_branch = vcs_result_controldir.create_branch(
                    name=target_branch_name)
            # TODO(jelmer): Set depth
            from_branch.push(target_branch, overwrite=True)
        for branch_name in ADDITIONAL_COLOCATED_BRANCHES:
            try:
                from_branch = ws.local_branch.controldir.open_branch(
                    name=branch_name)
            except NotBranchError:
                continue
            try:
                target_branch = vcs_result_controldir.open_branch(
                    name=branch_name)
            except NotBranchError:
                target_branch = vcs_result_controldir.create_branch(
                    name=branch_name)
            from_branch.push(target_branch, overwrite=True)
    else:
        path = os.path.join(vcs_result_dir, 'bzr', pkg)
        os.makedirs(path, exist_ok=True)
        try:
            vcs_result_controldir = ControlDir.open(path)
        except NotBranchError:
            vcs_result_controldir = ControlDir.create(
                path, format=format_registry.get('bzr')())
        vcs_result_controldir.create_repository(shared=True)
        for (from_branch, target_branch_name) in [
                (ws.local_branch, name),
                (ws.main_branch, 'master')]:
            target_branch_path = os.path.join(path, target_branch_name)
            try:
                target_branch = Branch.open(target_branch_path)
            except NotBranchError:
                target_branch = ControlDir.create_branch_convenience(
                    target_branch_path)
            target_branch.set_stacked_on(ws.main_branch)
            from_branch.push(target_branch, overwrite=True)


async def process_one(
        vcs_url, mode, env, command,
        max_mps_per_maintainer,
        build_command, open_mps_per_maintainer,
        refresh=False, pre_check=None, post_check=None,
        dry_run=False, incoming=None, log_dir=None,
        debsign_keyid=None, vcs_result_dir=None,
        possible_transports=None, possible_hosters=None):
    maintainer_email = env['MAINTAINER_EMAIL']
    pkg = env['PACKAGE']
    if max_mps_per_maintainer and \
            open_mps_per_maintainer.get(maintainer_email, 0) \
            >= max_mps_per_maintainer and mode in ('propose', 'attempt-push'):
        warning(
            'Not creating proposal for %s, maximum number of open merge '
            'proposals reached for maintainer %s', pkg, maintainer_email)
        if mode == 'propose':
            mode = 'build-only'
        if mode == 'attempt-push':
            mode = 'push'
    if mode == "attempt-push" and "salsa.debian.org/debian/" in vcs_url:
        # Make sure we don't accidentally push to unsuspecting collab-maint
        # repositories, even if debian-janitor becomes a member of "debian"
        # in the future.
        mode = "propose"
    packages_processed_count.inc()
    log_id = str(uuid.uuid4())

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
            pkg, log_id=log_id, description=str(e), code='branch-unavailable')
    except KeyError as e:
        if e.args == ('www-authenticate not found',):
            return JanitorResult(
                pkg, log_id=log_id, description=str(e),
                code='401-without-www-authenticate')
        else:
            raise
    try:
        hoster = get_hoster(main_branch, possible_hosters=possible_hosters)
    except UnsupportedHoster as e:
        if mode not in ('push', 'build-only'):
            return JanitorResult(
                pkg, log_id, description='Hoster unsupported.',
                code='hoster-unsupported')
        # We can't figure out what branch to resume from when there's no hoster
        # that can tell us.
        resume_branch = None
        existing_proposal = None
        if mode == 'push':
            warning('Unsupported hoster (%s), will attempt to push to %s',
                    e, main_branch.user_url)
    else:
        try:
            (resume_branch, overwrite, existing_proposal) = (
                find_existing_proposed(
                    main_branch, hoster, subrunner.branch_name))
        except NoSuchProject as e:
            if mode not in ('push', 'build-only'):
                return JanitorResult(
                    pkg, log_id,
                    description='Project %s not found.' % e.project,
                    code='project-not-found')
            resume_branch = None
            existing_proposal = None

    if refresh:
        resume_branch = None

    with tempfile.TemporaryDirectory() as output_directory:
        retcode = await invoke_subprocess_worker(
                main_branch, env, command, output_directory,
                resume_branch=resume_branch, pre_check=pre_check,
                post_check=post_check, build_command=build_command,
                log_path=os.path.join(output_directory, 'worker.log'))

        for name in ['build.log', 'worker.log', 'result.json']:
            src_build_log_path = os.path.join(output_directory, name)
            if os.path.exists(src_build_log_path):
                dest_build_log_path = os.path.join(
                    log_dir, pkg, log_id)
                os.makedirs(dest_build_log_path, exist_ok=True)
                shutil.copy(src_build_log_path, dest_build_log_path)

        json_result_path = os.path.join(output_directory, 'result.json')
        if os.path.exists(json_result_path):
            worker_result = WorkerResult.from_file(json_result_path)
        else:
            worker_result = WorkerResult(
                'worker-missing-result',
                'Worker failed and did not write a result file.')
        if worker_result.subworker:
            subrunner.read_worker_result(worker_result.subworker)

        if retcode != 0:
            return JanitorResult(
                pkg, log_id=log_id, worker_result=worker_result)

        assert worker_result.code is None

        result = JanitorResult(
            pkg, log_id=log_id,
            code='success', worker_result=worker_result)

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

        def get_proposal_commit_message(existing_proposal):
            if existing_proposal:
                existing_commit_message = (
                    getattr(existing_proposal, 'get_commit_message',
                            lambda: None)())
            else:
                existing_commit_message = None
            return subrunner.get_proposal_commit_message(
                existing_commit_message)

        try:
            local_branch = open_branch(os.path.join(output_directory, pkg))
        except BranchUnavailable as e:
            return JanitorResult(
                pkg, log_id,
                description='result branch missing: %s' % e,
                code='result-branch-unavailable',
                worker_result=worker_result)
        with Pending(main_branch, local_branch,
                     resume_branch=resume_branch) as ws:
            enable_tag_pushing(local_branch)
            if mode != 'build-only':
                try:
                    (result.proposal, is_new) = publish_changes(
                        ws, mode, subrunner.branch_name,
                        get_proposal_description=get_proposal_description,
                        get_proposal_commit_message=(
                            get_proposal_commit_message),
                        dry_run=dry_run, hoster=hoster,
                        allow_create_proposal=(
                            subrunner.allow_create_proposal()),
                        overwrite_existing=True,
                        existing_proposal=existing_proposal)
                except NoSuchProject as e:
                    return JanitorResult(
                        pkg, log_id,
                        description='project %s was not found' % e.project,
                        code='project-not-found',
                        worker_result=worker_result)
                except PermissionDenied as e:
                    return JanitorResult(
                        pkg, log_id, description=str(e),
                        code='permission-denied',
                        worker_result=worker_result)
            if vcs_result_dir:
                publish_vcs_dir(
                    ws, vcs_result_dir, pkg, subrunner.branch_name,
                    additional_colocated_branches=(
                        ws.additional_colocated_branches))

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


async def export_queue_length():
    while True:
        queue_length.set(state.queue_length())
        await asyncio.sleep(60)


async def process_queue(
        max_mps_per_maintainer,
        build_command, open_mps_per_maintainer,
        refresh=False, pre_check=None, post_check=None,
        dry_run=False, incoming=None, log_dir=None,
        debsign_keyid=None, vcs_result_dir=None):
    while True:
        try:
            (queue_id, vcs_url, mode, env, command) = next(
                state.iter_queue(limit=1))
        except StopIteration:
            break
        start_time = datetime.now()
        result = await process_one(
            vcs_url, mode, env, command,
            max_mps_per_maintainer=max_mps_per_maintainer,
            open_mps_per_maintainer=open_mps_per_maintainer,
            refresh=refresh, pre_check=pre_check,
            build_command=build_command, post_check=post_check,
            dry_run=dry_run, incoming=incoming,
            debsign_keyid=debsign_keyid, vcs_result_dir=vcs_result_dir,
            log_dir=log_dir)
        finish_time = datetime.now()
        state.store_run(
            result.log_id, env['PACKAGE'], vcs_url, env['MAINTAINER_EMAIL'],
            start_time, finish_time, command,
            result.description,
            result.context,
            result.main_branch_revision,
            result.code,
            result.proposal.url if result.proposal else None,
            build_version=result.build_version,
            build_distribution=result.build_distribution)

        state.drop_queue_item(queue_id)
        last_success_gauge.set_to_current_time()


async def run_web_server(listen_addr, port):
    async def metrics(request):
        resp = web.Response(body=generate_latest())
        resp.content_type = CONTENT_TYPE_LATEST
        return resp

    app = web.Application()
    app.router.add_get("/metrics", metrics)
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()


def main(argv=None):
    import argparse
    parser = argparse.ArgumentParser(prog='janitor.runner')
    parser.add_argument(
        '--listen-address', type=str,
        help='Listen address', default='localhost')
    parser.add_argument(
        '--port', type=int,
        help='Listen port', default=9911)
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
        default=None)
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
    parser.add_argument(
        '--vcs-result-dir', type=str,
        help='Directory to store VCS repositories in.')

    args = parser.parse_args()

    open_mps_per_maintainer = get_open_mps_per_maintainer()
    for maintainer_email, count in open_mps_per_maintainer.items():
        open_proposal_count.labels(maintainer=maintainer_email).inc(count)

    loop = asyncio.get_event_loop()
    loop.run_until_complete(asyncio.gather(
        loop.create_task(process_queue(
            args.max_mps_per_maintainer,
            args.build_command, open_mps_per_maintainer,
            args.refresh, args.pre_check, args.post_check,
            args.dry_run, args.incoming, args.log_dir,
            args.debsign_keyid,
            args.vcs_result_dir)),
        loop.create_task(export_queue_length()),
        loop.create_task(run_web_server(args.listen_address, args.port)),
        ))


if __name__ == '__main__':
    sys.exit(main(sys.argv))
