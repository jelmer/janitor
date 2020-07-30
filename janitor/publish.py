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

from aiohttp.web_middlewares import normalize_path_middleware
from aiohttp import web
from datetime import datetime, timedelta
from dulwich.repo import Repo as DulwichRepo
import asyncio
import functools
import gpg
from io import BytesIO
import json
import os
import shlex
import sys
import time
from typing import Dict, List, Optional, Any
import uuid

from breezy.controldir import ControlDir, format_registry
from breezy.bzr.smart import medium
from breezy.transport import get_transport_from_url


from dulwich.protocol import ReceivableProtocol
from dulwich.server import (
    DEFAULT_HANDLERS as DULWICH_SERVICE_HANDLERS,
    DictBackend,
    )

from prometheus_client import (
    Counter,
    Gauge,
    Histogram,
    push_to_gateway,
    REGISTRY,
)

from silver_platter.proposal import (
    iter_all_mps,
    Hoster,
    hosters,
    )
from silver_platter.utils import (
    open_branch,
    BranchMissing,
    BranchUnavailable,
    )

from breezy.propose import get_proposal_by_url, HosterLoginRequired
from breezy.transport import Transport
import breezy.plugins.gitlab  # noqa: F401
import breezy.plugins.launchpad  # noqa: F401
import breezy.plugins.github  # noqa: F401

from . import (
    state,
    )
from .config import read_config
from .prometheus import setup_metrics
from .pubsub import Topic, pubsub_handler, pubsub_reader
from .trace import note, warning
from .vcs import (
    VcsManager,
    LocalVcsManager,
    get_run_diff,
    bzr_to_browse_url,
    )


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


proposal_rate_limited_count = Counter(
    'proposal_rate_limited',
    'Number of attempts to create a proposal that was rate-limited',
    ['package', 'suite'])
open_proposal_count = Gauge(
    'open_proposal_count', 'Number of open proposals.',
    labelnames=('maintainer',))
merge_proposal_count = Gauge(
    'merge_proposal_count', 'Number of merge proposals by status.',
    labelnames=('status',))
last_success_gauge = Gauge(
    'job_last_success_unixtime',
    'Last time a batch job successfully finished')
publish_ready_count = Gauge(
    'publish_ready_count', 'Number of publish ready runs by status.',
    labelnames=('review_status', 'publish_mode'))
successful_push_count = Gauge(
    'successful_push_count', 'Number of successful pushes.')
publish_latency = Histogram(
    'publish_latency', 'Delay between build finish and publish.')


class RateLimited(Exception):
    """A rate limit was reached."""


class RateLimiter(object):

    def set_mps_per_maintainer(
            self, mps_per_maintainer: Dict[str, Dict[str, int]]
            ) -> None:
        raise NotImplementedError(self.set_mps_per_maintainer)

    def check_allowed(self, maintainer_email: str) -> None:
        raise NotImplementedError(self.check_allowed)

    def inc(self, maintainer_email: str) -> None:
        raise NotImplementedError(self.inc)


class MaintainerRateLimiter(RateLimiter):

    _open_mps_per_maintainer: Optional[Dict[str, int]]

    def __init__(self, max_mps_per_maintainer: Optional[int] = None):
        self._max_mps_per_maintainer = max_mps_per_maintainer
        self._open_mps_per_maintainer = None

    def set_mps_per_maintainer(
            self, mps_per_maintainer: Dict[str, Dict[str, int]]):
        self._open_mps_per_maintainer = mps_per_maintainer['open']

    def check_allowed(self, maintainer_email: str):
        if not self._max_mps_per_maintainer:
            return
        if self._open_mps_per_maintainer is None:
            # Be conservative
            raise RateLimited('Open mps per maintainer not yet determined.')
        current = self._open_mps_per_maintainer.get(maintainer_email, 0)
        if current > self._max_mps_per_maintainer:
            raise RateLimited(
                'Maintainer %s already has %d merge proposal open (max: %d)'
                % (maintainer_email, current, self._max_mps_per_maintainer))

    def inc(self, maintainer_email: str):
        if self._open_mps_per_maintainer is None:
            return
        self._open_mps_per_maintainer.setdefault(maintainer_email, 0)
        self._open_mps_per_maintainer[maintainer_email] += 1


class NonRateLimiter(RateLimiter):

    def check_allowed(self, email):
        pass

    def inc(self, maintainer_email):
        pass

    def set_mps_per_maintainer(self, mps_per_maintainer):
        pass


class SlowStartRateLimiter(RateLimiter):

    def __init__(self, max_mps_per_maintainer=None):
        self._max_mps_per_maintainer = max_mps_per_maintainer
        self._open_mps_per_maintainer: Optional[Dict[str, int]] = None
        self._merged_mps_per_maintainer: Optional[Dict[str, int]] = None

    def check_allowed(self, email: str) -> None:
        if (self._open_mps_per_maintainer is None or
                self._merged_mps_per_maintainer is None):
            # Be conservative
            raise RateLimited('Open mps per maintainer not yet determined.')
        current = self._open_mps_per_maintainer.get(email, 0)
        if (self._max_mps_per_maintainer and
                current >= self._max_mps_per_maintainer):
            raise RateLimited(
                'Maintainer %s already has %d merge proposal open (absmax: %d)'
                % (email, current, self._max_mps_per_maintainer))
        limit = self._merged_mps_per_maintainer.get(email, 0) + 1
        if current >= limit:
            raise RateLimited(
                'Maintainer %s has %d merge proposals open (current cap: %d)'
                % (email, current, limit))

    def inc(self, maintainer_email: str):
        if self._open_mps_per_maintainer is None:
            return
        self._open_mps_per_maintainer.setdefault(maintainer_email, 0)
        self._open_mps_per_maintainer[maintainer_email] += 1

    def set_mps_per_maintainer(
            self, mps_per_maintainer: Dict[str, Dict[str, int]]):
        self._open_mps_per_maintainer = mps_per_maintainer['open']
        self._merged_mps_per_maintainer = mps_per_maintainer['merged']


class PublishFailure(Exception):

    def __init__(self, mode: str, code: str, description: str):
        self.mode = mode
        self.code = code
        self.description = description


async def publish_one(
        suite: str, pkg: str, command, subworker_result, main_branch_url: str,
        mode: str, log_id: str, maintainer_email: str, vcs_manager: VcsManager,
        branch_name: str, topic_merge_proposal, rate_limiter: RateLimiter,
        dry_run: bool, external_url: str,
        require_binary_diff: bool = False,
        possible_hosters=None,
        possible_transports: Optional[List[Transport]] = None,
        allow_create_proposal: Optional[bool] = None,
        reviewers: Optional[List[str]] = None):
    """Publish a single run in some form.

    Args:
      suite: The suite name
      pkg: Package name
      command: Command that was run
    """
    assert mode in SUPPORTED_MODES, 'mode is %r' % mode
    local_branch = vcs_manager.get_branch(pkg, branch_name)
    if local_branch is None:
        raise PublishFailure(
            mode, 'result-branch-not-found',
            'can not find local branch for %s / %s (%s)' % (
                pkg, branch_name, log_id))

    request = {
        'dry-run': dry_run,
        'suite': suite,
        'package': pkg,
        'command': command,
        'subworker_result': subworker_result,
        'main_branch_url': main_branch_url.rstrip('/'),
        'local_branch_url': local_branch.user_url,
        'mode': mode,
        'log_id': log_id,
        'require-binary-diff': require_binary_diff,
        'allow_create_proposal': allow_create_proposal,
        'external_url': external_url,
        'reviewers': reviewers}

    args = [sys.executable, '-m', 'janitor.publish_one']

    p = await asyncio.create_subprocess_exec(
        *args, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE,
        stdin=asyncio.subprocess.PIPE)

    (stdout, stderr) = await p.communicate(json.dumps(request).encode())

    if p.returncode == 1:
        try:
            response = json.loads(stdout.decode())
        except json.JSONDecodeError:
            raise PublishFailure(
                mode, 'publisher-invalid-response', stderr.decode())
        sys.stderr.write(stderr.decode())
        raise PublishFailure(mode, response['code'], response['description'])

    if p.returncode == 0:
        response = json.loads(stdout.decode())

        proposal_url = response.get('proposal_url')
        branch_name = response.get('branch_name')
        is_new = response.get('is_new')

        if proposal_url and is_new:
            topic_merge_proposal.publish(
                {'url': proposal_url, 'status': 'open', 'package': pkg})

            merge_proposal_count.labels(status='open').inc()
            rate_limiter.inc(maintainer_email)
            open_proposal_count.labels(maintainer=maintainer_email).inc()

        return proposal_url, branch_name, is_new

    raise PublishFailure(mode, 'publisher-invalid-response', stderr.decode())


async def publish_pending_new(db, rate_limiter, vcs_manager,
                              topic_publish, topic_merge_proposal,
                              dry_run: bool, external_url: str,
                              reviewed_only: bool = False,
                              push_limit: Optional[int] = None,
                              require_binary_diff: bool = False):
    start = time.time()
    possible_hosters: List[Hoster] = []
    possible_transports: List[Transport] = []

    if reviewed_only:
        review_status = ['approved']
    else:
        review_status = ['approved', 'unreviewed']

    async with db.acquire() as conn1, db.acquire() as conn:
        async for (run, maintainer_email, uploader_emails, main_branch_url,
                   publish_mode, update_changelog,
                   command) in state.iter_publish_ready(
                       conn1, review_status=review_status,
                       publishable_only=True):
            if run.revision is None:
                warning(
                    'Run %s is publish ready, but does not have revision set.',
                    run.id)
                continue
            # TODO(jelmer): next try in SQL query
            attempt_count = await state.get_publish_attempt_count(
                conn, run.revision)
            try:
                next_try_time = run.times[1] + (
                    2 ** attempt_count * timedelta(hours=1))
            except OverflowError:
                continue
            if datetime.now() < next_try_time:
                note('Not attempting to push %s / %s (%s) due to '
                     'exponential backoff. Next try in %s.',
                     run.package, run.suite, run.id,
                     next_try_time - datetime.now())
                continue
            if push_limit is not None and publish_mode in (
                    MODE_PUSH, MODE_ATTEMPT_PUSH):
                if push_limit == 0:
                    note('Not pushing %s / %s: push limit reached',
                         run.package, run.suite)
                    continue
            actual_mode = await publish_from_policy(
                    conn, rate_limiter, vcs_manager, run,
                    maintainer_email, uploader_emails, main_branch_url,
                    topic_publish, topic_merge_proposal,
                    publish_mode, update_changelog, command,
                    possible_hosters=possible_hosters,
                    possible_transports=possible_transports, dry_run=dry_run,
                    external_url=external_url,
                    require_binary_diff=require_binary_diff,
                    force=False, requestor='publisher (publish pending)')
            if actual_mode == MODE_PUSH and push_limit is not None:
                push_limit -= 1

    note('Done publishing pending changes; duration: %.2fs' % (
         time.time() - start))


async def publish_from_policy(
        conn, rate_limiter, vcs_manager, run: state.Run,
        maintainer_email: str,
        uploader_emails: List[str], main_branch_url: str,
        topic_publish, topic_merge_proposal,
        mode: str, update_changelog: str, command: List[str],
        dry_run: bool, external_url: str,
        possible_hosters: Optional[List[Hoster]] = None,
        possible_transports: Optional[List[Transport]] = None,
        require_binary_diff: bool = False, force: bool = False,
        requestor: Optional[str] = None):
    from .schedule import (
        full_command,
        estimate_duration,
        do_schedule,
        do_schedule_control,
        )
    if not command:
        warning('no command set for %s', run.id)
        return
    expected_command = full_command(update_changelog, command)
    if ' '.join(expected_command) != run.command:
        warning(
            'Not publishing %s/%s: command is different (policy changed?). '
            'Build used %r, now: %r. Rescheduling.',
            run.package, run.suite, run.command, ' '.join(expected_command))
        estimated_duration = await estimate_duration(
            conn, run.package, run.suite)
        await state.add_to_queue(
            conn, run.package, expected_command, run.suite, -2,
            estimated_duration=estimated_duration, refresh=True,
            requestor='publisher (changed policy)')
        return

    publish_id = str(uuid.uuid4())
    if mode in (None, MODE_BUILD_ONLY, MODE_SKIP):
        return
    if run.branch_name is None:
        warning('no branch name set for %s', run.id)
        return
    if run.revision is None:
        warning('no revision set for %s', run.id)
        return
    if not force and await state.already_published(
            conn, run.package, run.branch_name, run.revision, mode):
        return
    if mode in (MODE_PROPOSE, MODE_ATTEMPT_PUSH):
        open_mp = await state.get_open_merge_proposal(
            conn, run.package, run.branch_name)
        if not open_mp:
            try:
                rate_limiter.check_allowed(maintainer_email)
            except RateLimited as e:
                proposal_rate_limited_count.labels(
                    package=run.package, suite=run.suite).inc()
                warning('Not creating proposal for %s: %s', run.package, e)
                mode = MODE_BUILD_ONLY
    if mode == MODE_ATTEMPT_PUSH and \
            "salsa.debian.org/debian/" in main_branch_url:
        # Make sure we don't accidentally push to unsuspecting
        # collab-maint repositories, even if debian-janitor becomes a
        # member of "debian" in the future.
        warning('Refusing to push directly to %s, switch back to propose.',
                main_branch_url)
        mode = MODE_PROPOSE
    if mode in (MODE_BUILD_ONLY, MODE_SKIP):
        return

    note('Publishing %s / %r (mode: %s)', run.package, run.command, mode)
    try:
        proposal_url, branch_name, is_new = await publish_one(
            run.suite, run.package, run.command, run.result,
            main_branch_url, mode, run.id, maintainer_email,
            vcs_manager=vcs_manager, branch_name=run.branch_name,
            topic_merge_proposal=topic_merge_proposal,
            dry_run=dry_run, external_url=external_url,
            require_binary_diff=require_binary_diff,
            possible_hosters=possible_hosters,
            possible_transports=possible_transports,
            rate_limiter=rate_limiter)
    except PublishFailure as e:
        code = e.code
        description = e.description
        if e.code == 'merge-conflict':
            note('Merge proposal would cause conflict; restarting.')
            await do_schedule(
                conn, run.package, run.suite,
                requestor='publisher (pre-creation merge conflict)')
        elif e.code == 'missing-binary-diff':
            unchanged_run = await state.get_unchanged_run(
                conn, run.main_branch_revision)
            if unchanged_run and unchanged_run.result_code == 'success':
                description = (
                    'Missing binary diff, but unchanged run exists. '
                    'Not published yet?')
            elif unchanged_run:
                description = (
                    'Missing binary diff; last control run failed (%s).' %
                    unchanged_run.result_code)
            else:
                description = (
                    'Missing binary diff; requesting control run.')
                if run.main_branch_revision is not None:
                    await do_schedule_control(
                        conn, run.package, run.main_branch_revision,
                        requestor='publisher (missing binary diff)')
                else:
                    warning(
                        'Successful run (%s) does not have main branch '
                        'revision set', run.id)
        branch_name = None
        proposal_url = None
        note('Failed(%s): %s', code, description)
    else:
        code = 'success'
        description = 'Success'

    if mode == MODE_ATTEMPT_PUSH:
        if proposal_url:
            mode = MODE_PROPOSE
        else:
            mode = MODE_PUSH

    await state.store_publish(
        conn, run.package, branch_name, run.main_branch_revision,
        run.revision, mode, code, description,
        proposal_url if proposal_url else None,
        publish_id=publish_id, requestor=requestor)

    if code == 'success' and mode == MODE_PUSH:
        # TODO(jelmer): Call state.update_branch_status() for the
        # main branch URL
        pass

    publish_delay: Optional[timedelta]
    if code == 'success':
        publish_delay = datetime.now() - run.times[1]
        publish_latency.observe(publish_delay.total_seconds())
    else:
        publish_delay = None

    topic_entry: Dict[str, Any] = {
         'id': publish_id,
         'package': run.package,
         'suite': run.suite,
         'proposal_url': proposal_url or None,
         'mode': mode,
         'main_branch_url': main_branch_url,
         'main_branch_browse_url': bzr_to_browse_url(main_branch_url),
         'branch_name': branch_name,
         'result_code': code,
         'result': run.result,
         'run_id': run.id,
         'publish_delay': (
             publish_delay.total_seconds() if publish_delay else None)
         }

    topic_publish.publish(topic_entry)

    if code == 'success':
        return mode


async def diff_request(request):
    run_id = request.match_info['run_id']
    async with request.app.db.acquire() as conn:
        run = await state.get_run(conn, run_id)
        if not run:
            raise web.HTTPNotFound(text='No such run: %r' % run_id)
    diff = get_run_diff(request.app.vcs_manager, run)
    return web.Response(body=diff, content_type='text/x-diff')


async def publish_and_store(
        db, topic_publish, topic_merge_proposal, publish_id, run, mode,
        maintainer_email, uploader_emails, vcs_manager, rate_limiter,
        dry_run, external_url: str, allow_create_proposal: bool = True,
        require_binary_diff: bool = False, requestor: Optional[str] = None):
    async with db.acquire() as conn:
        try:
            proposal_url, branch_name, is_new = await publish_one(
                run.suite, run.package, run.command, run.result,
                run.branch_url, mode, run.id, maintainer_email, vcs_manager,
                run.branch_name, dry_run=dry_run,
                external_url=external_url,
                require_binary_diff=require_binary_diff,
                possible_hosters=None, possible_transports=None,
                allow_create_proposal=allow_create_proposal,
                topic_merge_proposal=topic_merge_proposal,
                rate_limiter=rate_limiter)
        except PublishFailure as e:
            await state.store_publish(
                conn, run.package, run.branch_name,
                run.main_branch_revision,
                run.revision, e.mode, e.code, e.description,
                None, publish_id=publish_id, requestor=requestor)
            topic_publish.publish({
                'id': publish_id,
                'mode': e.mode,
                'result_code': e.code,
                'description': e.description,
                'package': run.package,
                'suite': run.suite,
                'main_branch_url': run.branch_url,
                'main_branch_browse_url': bzr_to_browse_url(run.branch_url),
                'result': run.result,
                })
            return

        if mode == MODE_ATTEMPT_PUSH:
            if proposal_url:
                mode = MODE_PROPOSE
            else:
                mode = MODE_PUSH

        await state.store_publish(
            conn, run.package, branch_name,
            run.main_branch_revision,
            run.revision, mode, 'success', 'Success',
            proposal_url if proposal_url else None,
            publish_id=publish_id, requestor=requestor)

        publish_delay = run.times[1] - datetime.now()
        publish_latency.observe(publish_delay.total_seconds())

        topic_publish.publish(
            {'id': publish_id,
             'package': run.package,
             'suite': run.suite,
             'proposal_url': proposal_url or None,
             'mode': mode,
             'main_branch_url': run.branch_url,
             'main_branch_browse_url': bzr_to_browse_url(run.branch_url),
             'branch_name': branch_name,
             'result_code': 'success',
             'result': run.result,
             'publish_delay': publish_delay.total_seconds(),
             'run_id': run.id})


async def publish_request(request):
    dry_run = request.app.dry_run
    vcs_manager = request.app.vcs_manager
    rate_limiter = request.app.rate_limiter
    package = request.match_info['package']
    suite = request.match_info['suite']
    post = await request.post()
    mode = post.get('mode', MODE_PROPOSE)
    async with request.app.db.acquire() as conn:
        try:
            package = await state.get_package(conn, package)
        except IndexError:
            return web.json_response({}, status=400)

        if mode in (MODE_SKIP, MODE_BUILD_ONLY):
            return web.json_response(
                {'code': 'done',
                 'description':
                    'Nothing to do'})

        run = await state.get_last_effective_run(conn, package.name, suite)
        if run is None:
            return web.json_response({}, status=400)
        note('Handling request to publish %s/%s', package.name, suite)

    publish_id = str(uuid.uuid4())

    request.loop.create_task(publish_and_store(
        request.app.db, request.app.topic_publish,
        request.app.topic_merge_proposal, publish_id, run, mode,
        package.maintainer_email, package.uploader_emails,
        vcs_manager=vcs_manager, rate_limiter=rate_limiter, dry_run=dry_run,
        external_url=request.db.external_url, allow_create_proposal=True,
        require_binary_diff=False, requestor=post.get('requestor')))

    return web.json_response(
        {'run_id': run.id, 'mode': mode, 'publish_id': publish_id},
        status=202)


async def _git_open_repo(vcs_manager, db, package):
    repo = vcs_manager.get_repository(package, 'git')

    if repo is None:
        async with db.acquire() as conn:
            if not await state.package_exists(conn, package):
                raise web.HTTPNotFound()
        controldir = ControlDir.create(
            vcs_manager.get_repository_url(package, 'git'),
            format=format_registry.get('git-bare')())
        note('Created missing git repository for %s at %s',
             package, controldir.user_url)
        return controldir.open_repository()
    else:
        return repo


def _git_check_service(service: str, allow_writes: bool = False):
    if service == 'git-upload-pack':
        return

    if service == 'git-receive-pack':
        if not allow_writes:
            raise web.HTTPUnauthorized(text='git-receive-pack requires login')
        return

    raise web.HTTPForbidden(text='Unsupported service %s' % service)


async def handle_klaus(request):
    package = request.match_info['package']

    repo = await _git_open_repo(
        request.app.vcs_manager, request.app.db, package)

    from klaus import views, utils, KLAUS_VERSION
    from flask import Flask
    from klaus.repo import FancyRepo

    class Klaus(Flask):

        def __init__(self, package, repo):
            super(Klaus, self).__init__('klaus')
            self.package = package
            self.valid_repos = {
                package: FancyRepo(repo._transport.local_abspath('.'))}

        def should_use_ctags(self, git_repo, git_commit):
            return False

        def create_jinja_environment(self):
            """Called by Flask.__init__"""
            env = super(Klaus, self).create_jinja_environment()
            for func in [
                    'force_unicode',
                    'timesince',
                    'shorten_sha1',
                    'shorten_message',
                    'extract_author_name',
                    'formattimestamp']:
                env.filters[func] = getattr(utils, func)

            env.globals['KLAUS_VERSION'] = KLAUS_VERSION
            env.globals['USE_SMARTHTTP'] = False
            env.globals['SITE_NAME'] = 'Package list'
            return env

    app = Klaus(package, repo)

    for endpoint, rule in [
            ('blob',        '/blob/'),
            ('blob',        '/blob/<rev>/<path:path>'),
            ('blame',       '/blame/'),
            ('blame',       '/blame/<rev>/<path:path>'),
            ('raw',         '/raw/<path:path>/'),
            ('raw',         '/raw/<rev>/<path:path>'),
            ('submodule',   '/submodule/<rev>/'),
            ('submodule',   '/submodule/<rev>/<path:path>'),
            ('commit',      '/commit/<path:rev>/'),
            ('patch',       '/commit/<path:rev>.diff'),
            ('patch',       '/commit/<path:rev>.patch'),
            ('index',       '/'),
            ('index',       '/<path:rev>'),
            ('history',     '/tree/<rev>/'),
            ('history',     '/tree/<rev>/<path:path>'),
            ('download',    '/tarball/<path:rev>/'),
            ('repo_list',   '/..'),
          ]:
        app.add_url_rule(
            rule, view_func=getattr(views, endpoint),
            defaults={'repo': package})

    from aiohttp_wsgi import WSGIHandler
    wsgi_handler = WSGIHandler(app)

    return await wsgi_handler(request)


async def dulwich_refs(request):
    package = request.match_info['package']

    allow_writes = request.query.get('allow_writes')

    repo = await _git_open_repo(
        request.app.vcs_manager, request.app.db, package)
    r = DulwichRepo(repo._transport.local_abspath('.'))

    service = request.query.get('service')
    _git_check_service(service, allow_writes)

    headers = {
          'Expires': 'Fri, 01 Jan 1980 00:00:00 GMT',
          'Pragma': 'no-cache',
          'Cache-Control': 'no-cache, max-age=0, must-revalidate',
          }

    handler_cls = DULWICH_SERVICE_HANDLERS[service.encode('ascii')]

    response = web.StreamResponse(
        status=200,
        headers=headers)
    response.content_type = 'application/x-%s-advertisement' % service

    await response.prepare(request)

    out = BytesIO()
    proto = ReceivableProtocol(BytesIO().read, out.write)
    handler = handler_cls(DictBackend({'.': r}), ['.'], proto,
                          stateless_rpc=True, advertise_refs=True)
    handler.proto.write_pkt_line(
        b'# service=' + service.encode('ascii') + b'\n')
    handler.proto.write_pkt_line(None)
    handler.handle()

    await response.write(out.getvalue())

    await response.write_eof()

    return response


async def dulwich_service(request):
    package = request.match_info['package']
    service = request.match_info['service']

    allow_writes = bool(request.query.get('allow_writes'))

    repo = await _git_open_repo(
        request.app.vcs_manager, request.app.db, package)
    r = DulwichRepo(repo._transport.local_abspath('.'))

    _git_check_service(service, allow_writes)

    headers = {
          'Expires': 'Fri, 01 Jan 1980 00:00:00 GMT',
          'Pragma': 'no-cache',
          'Cache-Control': 'no-cache, max-age=0, must-revalidate',
          }
    handler_cls = DULWICH_SERVICE_HANDLERS[service.encode('ascii')]

    response = web.StreamResponse(
        status=200,
        headers=headers)
    response.content_type = 'application/x-%s-result' % service

    await response.prepare(request)

    inf = BytesIO(await request.read())
    outf = BytesIO()

    proto = ReceivableProtocol(inf.read, outf.write)
    handler = handler_cls(
        DictBackend({'.': r}), ['.'], proto, stateless_rpc=True)
    handler.handle()

    await response.write(outf.getvalue())

    await response.write_eof()
    return response


async def bzr_backend(request):
    vcs_manager = request.app.vcs_manager
    package = request.match_info['package']
    branch = request.match_info.get('branch')
    repo = vcs_manager.get_repository(package, 'bzr')
    if request.query.get('allow_writes'):
        if repo is None:
            controldir = ControlDir.create(
                vcs_manager.get_repository_url(package, 'bzr'))
            repo = controldir.create_repository(shared=True)
        backing_transport = repo.user_transport
    else:
        if repo is None:
            raise web.HTTPNotFound()
        backing_transport = get_transport_from_url('readonly+' + repo.user_url)
    transport = backing_transport.clone(branch)
    out_buffer = BytesIO()
    request_data_bytes = await request.read()

    protocol_factory, unused_bytes = medium._get_protocol_factory_for_bytes(
        request_data_bytes)
    smart_protocol_request = protocol_factory(
        transport, out_buffer.write, '.', backing_transport)
    smart_protocol_request.accept_bytes(unused_bytes)
    if smart_protocol_request.next_read_size() != 0:
        # The request appears to be incomplete, or perhaps it's just a
        # newer version we don't understand.  Regardless, all we can do
        # is return an error response in the format of our version of the
        # protocol.
        response_data = b'error\x01incomplete request\n'
    else:
        response_data = out_buffer.getvalue()
    # TODO(jelmer): Use StreamResponse
    return web.Response(
        status=200, body=response_data,
        content_type='application/octet-stream')


async def get_vcs_type(request):
    package = request.match_info['package']
    vcs_type = request.app.vcs_manager.get_vcs_type(package)
    if vcs_type is None:
        raise web.HTTPNotFound()
    return web.Response(body=vcs_type.encode('utf-8'))


async def credentials_request(request):
    ssh_keys = []
    for entry in os.scandir(os.path.expanduser('~/.ssh')):
        if entry.name.endswith('.pub'):
            with open(entry.path, 'r') as f:
                ssh_keys.extend([line.strip() for line in f.readlines()])
    pgp_keys = []
    for entry in list(request.app.gpg.keylist(secret=True)):
        pgp_keys.append(request.app.gpg.key_export_minimal(entry.fpr).decode())
    hosting = []
    for name, hoster_cls in hosters.items():
        for instance in hoster_cls.iter_instances():
            try:
                current_user = instance.get_current_user()
            except HosterLoginRequired:
                continue
            if current_user:
                current_user_url = instance.get_user_url(current_user)
            else:
                current_user_url = None
            hoster = {
                'kind': name,
                'name': instance.name,
                'url': instance.base_url,
                'user': current_user,
                'user_url': current_user_url,
            }
            hosting.append(hoster)

    return web.json_response({
        'ssh_keys': ssh_keys,
        'pgp_keys': pgp_keys,
        'hosting': hosting,
    })


async def run_web_server(listen_addr: str, port: int,
                         rate_limiter: RateLimiter,
                         vcs_manager: VcsManager, db: state.Database,
                         topic_merge_proposal: Topic, topic_publish: Topic,
                         dry_run: bool, external_url: str,
                         require_binary_diff: bool = False,
                         push_limit: Optional[int] = None,
                         modify_mp_limit: Optional[int] = None):
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[trailing_slash_redirect])
    app.gpg = gpg.Context(armor=True)
    app.vcs_manager = vcs_manager
    app.db = db
    app.external_url = external_url
    app.rate_limiter = rate_limiter
    app.modify_mp_limit = modify_mp_limit
    app.topic_publish = topic_publish
    app.topic_merge_proposal = topic_merge_proposal
    app.dry_run = dry_run
    app.push_limit = push_limit
    app.require_binary_diff = require_binary_diff
    setup_metrics(app)
    app.router.add_post("/{suite}/{package}/publish", publish_request)
    app.router.add_get("/diff/{run_id}", diff_request)
    app.router.add_post(
        "/git/{package}/{service:git-receive-pack|git-upload-pack}",
        dulwich_service)
    app.router.add_get(
        "/git/{package}/info/refs", dulwich_refs)
    app.router.add_get(
        "/git/{package}/{path_info:.*}",
        handle_klaus)
    app.router.add_post(
        "/bzr/{package}/.bzr/smart", bzr_backend)
    app.router.add_post(
        "/bzr/{package}/{branch}/.bzr/smart", bzr_backend)
    app.router.add_get(
        '/vcs-type/{package}', get_vcs_type)
    app.router.add_get(
        '/ws/publish', functools.partial(pubsub_handler, topic_publish))
    app.router.add_get(
        '/ws/merge-proposal', functools.partial(
            pubsub_handler, topic_merge_proposal))
    app.router.add_post('/scan', scan_request)
    app.router.add_post('/refresh-status', refresh_proposal_status_request)
    app.router.add_post('/autopublish', autopublish_request)
    app.router.add_get('/credentials', credentials_request)
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    note('Listening on %s:%s', listen_addr, port)
    await site.start()


async def scan_request(request):
    async def scan():
        async with request.app.db.acquire() as conn:
            await check_existing(
                conn, request.app.rate_limiter,
                request.app.vcs_manager, request.app.topic_merge_proposal,
                dry_run=request.app.dry_run,
                modify_limit=request.app.modify_mp_limit)
    request.loop.create_task(scan())
    return web.Response(status=202, text="Scan started.")


async def refresh_proposal_status_request(request):
    post = await request.post()
    try:
        url = post['url']
    except KeyError:
        raise web.HTTPBadRequest(body="missing url parameter")
    note('Request to refresh proposal status for %s', url)

    async def scan():
        mp = get_proposal_by_url(url)
        async with request.app.db.acquire() as conn:
            if mp.is_merged():
                status = 'merged'
            elif mp.is_closed():
                status = 'closed'
            else:
                status = 'open'
            await check_existing_mp(
                conn, mp, status,
                vcs_manager=request.app.vcs_manager,
                rate_limiter=request.app.rate_limiter,
                topic_merge_proposal=request.app.topic_merge_proposal,
                dry_run=request.app.dry_run,
                external_url=request.app.external_url)
    request.loop.create_task(scan())
    return web.Response(status=202, text="Refresh of proposal started.")


async def autopublish_request(request):
    reviewed_only = ('unreviewed' not in request.query)

    async def autopublish():
        await publish_pending_new(
            request.app.db, request.app.rate_limiter, request.app.vcs_manager,
            dry_run=request.app.dry_run,
            topic_publish=request.app.topic_publish,
            topic_merge_proposal=request.app.topic_merge_proposal,
            reviewed_only=reviewed_only, push_limit=request.app.push_limit,
            require_binary_diff=request.app.require_binary_diff)

    request.loop.create_task(autopublish())
    return web.Response(status=202, text="Autopublish started.")


async def process_queue_loop(
        db, rate_limiter, dry_run, vcs_manager, interval,
        topic_merge_proposal, topic_publish,
        external_url: str,
        auto_publish: bool = True,
        reviewed_only: bool = False, push_limit: Optional[int] = None,
        modify_mp_limit: Optional[int] = None,
        require_binary_diff: bool = False):
    while True:
        async with db.acquire() as conn:
            await check_existing(
                conn, rate_limiter, vcs_manager, topic_merge_proposal,
                dry_run=dry_run, external_url=external_url,
                modify_limit=modify_mp_limit)
        await asyncio.sleep(interval)
        if auto_publish:
            await publish_pending_new(
                db, rate_limiter, vcs_manager, dry_run=dry_run,
                external_url=external_url,
                topic_publish=topic_publish,
                topic_merge_proposal=topic_merge_proposal,
                reviewed_only=reviewed_only, push_limit=push_limit,
                require_binary_diff=require_binary_diff)


def is_conflicted(mp):
    try:
        return not mp.can_be_merged()
    except NotImplementedError:
        # TODO(jelmer): Download and attempt to merge locally?
        return None


async def check_existing_mp(
        conn, mp, status, topic_merge_proposal, vcs_manager,
        rate_limiter, dry_run: bool, external_url: str,
        mps_per_maintainer=None,
        possible_transports: Optional[List[Transport]] = None) -> bool:
    async def update_proposal_status(mp, status, revision, package_name):
        if status == 'closed':
            # TODO(jelmer): Check if changes were applied manually and mark
            # as applied rather than closed?
            pass
        if status == 'merged':
            merged_by = mp.get_merged_by()
            merged_at = mp.get_merged_at()
            if merged_at is not None:
                merged_at = merged_at.replace(tzinfo=None)
        else:
            merged_by = None
            merged_at = None
        if not dry_run:
            await state.set_proposal_info(
                conn, mp.url, status, revision, package_name, merged_by,
                merged_at)
            topic_merge_proposal.publish(
               {'url': mp.url, 'status': status, 'package': package_name,
                'merged_by': merged_by, 'merged_at': str(merged_at)})

    old_status: Optional[str]
    maintainer_email: Optional[str]
    package_name: Optional[str]
    try:
        (old_revision, old_status, package_name,
            maintainer_email) = await state.get_proposal_info(conn, mp.url)
    except KeyError:
        old_revision = None
        old_status = None
        maintainer_email = None
        package_name = None
    revision = mp.get_source_revision()
    if revision is None:
        source_branch_url = mp.get_source_branch_url()
        if source_branch_url is None:
            warning('No source branch for %r', mp)
            revision = None
        else:
            try:
                revision = open_branch(
                    source_branch_url,
                    possible_transports=possible_transports).last_revision()
            except (BranchMissing, BranchUnavailable):
                revision = None
    if revision is None:
        revision = old_revision
    if maintainer_email is None:
        target_branch_url = mp.get_target_branch_url()
        package = await state.get_package_by_branch_url(
            conn, target_branch_url)
        if package is not None:
            maintainer_email = package.maintainer_email
            package_name = package.name
        else:
            if revision is not None:
                package_name, maintainer_email = (
                        await state.guess_package_from_revision(
                            conn, revision))
            if package_name is None:
                warning('No package known for %s (%s)',
                        mp.url, target_branch_url)
            else:
                note('Guessed package name for %s based on revision.',
                     mp.url)
    if old_status == 'applied' and status == 'closed':
        status = old_status
    if old_status != status:
        await update_proposal_status(mp, status, revision, package_name)
    if maintainer_email is not None and mps_per_maintainer is not None:
        mps_per_maintainer[status].setdefault(maintainer_email, 0)
        mps_per_maintainer[status][maintainer_email] += 1
    if status != 'open':
        return False
    mp_run = await state.get_merge_proposal_run(conn, mp.url)
    if mp_run is None:
        warning('Unable to find local metadata for %s, skipping.', mp.url)
        return False

    last_run = await state.get_last_effective_run(
        conn, mp_run.package, mp_run.suite)
    if last_run is None:
        warning('%s: Unable to find any relevant runs.', mp.url)
        return False

    if last_run.result_code == 'nothing-to-do':
        # A new run happened since the last, but there was nothing to
        # do.
        note('%s: Last run did not produce any changes, '
             'closing proposal.', mp.url)
        if not dry_run:
            await update_proposal_status(mp, 'applied', revision, package_name)
            mp.post_comment("""
This merge proposal will be closed, since all remaining changes have been
applied independently.
""")
            mp.close()
        return True

    if last_run.result_code != 'success':
        from .schedule import TRANSIENT_ERROR_RESULT_CODES
        if last_run.result_code in TRANSIENT_ERROR_RESULT_CODES:
            note('%s: Last run failed with transient error (%s). '
                 'Rescheduling.', mp.url, last_run.result_code)
            await state.add_to_queue(
                conn, last_run.package, shlex.split(last_run.command),
                last_run.suite, offset=1, refresh=False,
                requestor='publisher (transient error)')
        else:
            note('%s: Last run failed (%s). Not touching merge proposal.',
                 mp.url, last_run.result_code)
        return False

    if last_run.branch_name is None:
        note('%s: Last run (%s) does not have branch name set.', mp.url,
             last_run.id)
        return False

    if maintainer_email is None:
        note('%s: No maintainer email known.', mp.url)
        return False

    if last_run != mp_run:
        publish_id = str(uuid.uuid4())
        note('%s (%s) needs to be updated (%s => %s).',
             mp.url, mp_run.package, mp_run.id, last_run.id)
        if last_run.revision == mp_run.revision:
            warning('%s (%s): old run (%s) has same revision as new run (%s)'
                    ': %r', mp.url, mp.package, mp_run.id, last_run.id,
                    mp_run.revision)
        try:
            mp_url, branch_name, is_new = await publish_one(
                last_run.suite, last_run.package, last_run.command,
                last_run.result, last_run.branch_url, MODE_PROPOSE,
                last_run.id, maintainer_email,
                vcs_manager=vcs_manager, branch_name=last_run.branch_name,
                dry_run=dry_run, external_url=external_url,
                require_binary_diff=False,
                allow_create_proposal=True,
                topic_merge_proposal=topic_merge_proposal,
                rate_limiter=rate_limiter)
        except PublishFailure as e:
            note('%s: Updating merge proposal failed: %s (%s)',
                 mp.url, e.code, e.description)
            if not dry_run:
                await state.store_publish(
                    conn, last_run.package, mp_run.branch_name,
                    last_run.main_branch_revision,
                    last_run.revision, e.mode, e.code,
                    e.description, mp.url,
                    publish_id=publish_id,
                    requestor='publisher (regular refresh)')
        else:
            if not dry_run:
                await state.store_publish(
                    conn, last_run.package, branch_name,
                    last_run.main_branch_revision,
                    last_run.revision, MODE_PROPOSE, 'success',
                    'Succesfully updated', mp_url,
                    publish_id=publish_id,
                    requestor='publisher (regular refresh)')

            assert not is_new, "Intended to update proposal %r" % mp_url
        return True
    else:
        # It may take a while for the 'conflicted' bit on the proposal to
        # be refreshed, so only check it if we haven't made any other
        # changes.
        if is_conflicted(mp):
            note('%s is conflicted. Rescheduling.', mp.url)
            if not dry_run:
                await state.add_to_queue(
                    conn, mp_run.package, shlex.split(mp_run.command),
                    mp_run.suite, offset=-2, refresh=True,
                    requestor='publisher (merge conflict)')
        return False


async def check_existing(
        conn, rate_limiter, vcs_manager, topic_merge_proposal,
        dry_run: bool, external_url: str, modify_limit=None):
    mps_per_maintainer: Dict[str, Dict[str, int]] = {
        'open': {}, 'closed': {}, 'merged': {}, 'applied': {}}
    possible_transports: List[Transport] = []
    status_count = {'open': 0, 'closed': 0, 'merged': 0, 'applied': 0}

    modified_mps = 0

    for hoster, mp, status in iter_all_mps():
        status_count[status] += 1
        if modify_limit and modified_mps > modify_limit:
            warning('Already modified %d merge proposals, '
                    'waiting with the rest.', modified_mps)
            return
        modified = await check_existing_mp(
            conn, mp, status, topic_merge_proposal=topic_merge_proposal,
            vcs_manager=vcs_manager, dry_run=dry_run,
            external_url=external_url,
            rate_limiter=rate_limiter,
            possible_transports=possible_transports,
            mps_per_maintainer=mps_per_maintainer)
        if modified:
            modified_mps += 1

    for status, count in status_count.items():
        merge_proposal_count.labels(status=status).set(count)

    rate_limiter.set_mps_per_maintainer(mps_per_maintainer)
    for maintainer_email, count in mps_per_maintainer['open'].items():
        open_proposal_count.labels(maintainer=maintainer_email).set(count)


async def listen_to_runner(db, rate_limiter, vcs_manager, runner_url,
                           topic_publish, topic_merge_proposal, dry_run: bool,
                           external_url: str,
                           require_binary_diff: bool = False):
    async def process_run(conn, run, package):
        mode, update_changelog, command = (
            await state.get_publish_policy(
                conn, run.package, run.suite))
        await publish_from_policy(
            conn, rate_limiter, vcs_manager,
            run, package.maintainer_email, package.uploader_emails,
            package.branch_url,
            topic_publish, topic_merge_proposal, mode,
            update_changelog, command, dry_run=dry_run,
            external_url=external_url,
            require_binary_diff=require_binary_diff,
            force=True, requestor='runner')
    from aiohttp.client import ClientSession
    import urllib.parse
    url = urllib.parse.urljoin(runner_url, 'ws/result')
    async with ClientSession() as session:
        async for result in pubsub_reader(session, url):
            if result['code'] != 'success':
                continue
            async with db.acquire() as conn:
                # TODO(jelmer): Fold these into a single query ?
                package = await state.get_package(conn, result['package'])
                run = await state.get_run(conn, result['log_id'])
                if run.suite != 'unchanged':
                    await process_run(conn, run, package)
                else:
                    async for run in state.iter_last_runs(
                            conn, main_branch_revision=run.revision):
                        if run.package != package.name:
                            continue
                        if run.suite != 'unchanged':
                            await process_run(conn, run, package)


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
        '--prometheus', type=str,
        help='Prometheus push gateway to export to.')
    parser.add_argument(
        '--once', action='store_true',
        help="Just do one pass over the queue, don't run as a daemon.")
    parser.add_argument(
        '--listen-address', type=str,
        help='Listen address', default='localhost')
    parser.add_argument(
        '--port', type=int,
        help='Listen port', default=9912)
    parser.add_argument(
        '--interval', type=int,
        help=('Seconds to wait in between publishing '
              'pending proposals'), default=7200)
    parser.add_argument(
        '--no-auto-publish',
        action='store_true',
        help='Do not create merge proposals automatically.')
    parser.add_argument(
        '--config', type=str, default='janitor.conf',
        help='Path to load configuration from.')
    parser.add_argument(
        '--runner-url', type=str, default=None,
        help='URL to reach runner at.')
    parser.add_argument(
        '--slowstart',
        action='store_true', help='Use slow start rate limiter.')
    parser.add_argument(
        '--reviewed-only',
        action='store_true', help='Only publish changes that were reviewed.')
    parser.add_argument(
        '--push-limit', type=int, help='Limit number of pushes per cycle.')
    parser.add_argument(
        '--require-binary-diff', action='store_true', default=False,
        help='Require a binary diff when publishing merge requests.')
    parser.add_argument(
        '--modify-mp-limit', type=int, default=10,
        help='Maximum number of merge proposals to update per cycle.')
    parser.add_argument(
        '--external-url', type=str, help='External URL',
        default='https://janitor.debian.net/')

    args = parser.parse_args()

    with open(args.config, 'r') as f:
        config = read_config(f)

    state.DEFAULT_URL = config.database_location

    if args.slowstart:
        rate_limiter = SlowStartRateLimiter(args.max_mps_per_maintainer)
    elif args.max_mps_per_maintainer > 0:
        rate_limiter = MaintainerRateLimiter(args.max_mps_per_maintainer)
    else:
        rate_limiter = NonRateLimiter()

    if args.no_auto_publish and args.once:
        sys.stderr.write('--no-auto-publish and --once are mutually exclude.')
        sys.exit(1)

    topic_merge_proposal = Topic()
    topic_publish = Topic()
    loop = asyncio.get_event_loop()
    vcs_manager = LocalVcsManager(config.vcs_location)
    db = state.Database(config.database_location)
    if args.once:
        loop.run_until_complete(publish_pending_new(
            db, rate_limiter, dry_run=args.dry_run,
            external_url=args.external_url,
            vcs_manager=vcs_manager, topic_publish=topic_publish,
            topic_merge_proposal=topic_merge_proposal,
            reviewed_only=args.reviewed_only,
            require_binary_diff=args.require_binary_diff))

        last_success_gauge.set_to_current_time()
        if args.prometheus:
            push_to_gateway(
                args.prometheus, job='janitor.publish',
                registry=REGISTRY)
    else:
        tasks = [
            loop.create_task(process_queue_loop(
                db, rate_limiter, dry_run=args.dry_run,
                vcs_manager=vcs_manager, interval=args.interval,
                topic_merge_proposal=topic_merge_proposal,
                topic_publish=topic_publish,
                auto_publish=not args.no_auto_publish,
                external_url=args.external_url,
                reviewed_only=args.reviewed_only,
                push_limit=args.push_limit,
                modify_mp_limit=args.modify_mp_limit,
                require_binary_diff=args.require_binary_diff)),
            loop.create_task(
                run_web_server(
                    args.listen_address, args.port, rate_limiter,
                    vcs_manager, db, topic_merge_proposal, topic_publish,
                    dry_run=args.dry_run,
                    external_url=args.external_url,
                    require_binary_diff=args.require_binary_diff,
                    modify_mp_limit=args.modify_mp_limit,
                    push_limit=args.push_limit)),
        ]
        if args.runner_url and not args.reviewed_only:
            tasks.append(loop.create_task(
                listen_to_runner(
                    db, rate_limiter, vcs_manager,
                    args.runner_url, topic_publish,
                    topic_merge_proposal, dry_run=args.dry_run,
                    external_url=args.external_url,
                    require_binary_diff=args.require_binary_diff)))
        loop.run_until_complete(asyncio.gather(*tasks))


if __name__ == '__main__':
    sys.exit(main(sys.argv))
