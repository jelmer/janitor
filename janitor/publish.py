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

from aiohttp import web
from datetime import datetime
from http.client import parse_headers
import asyncio
import functools
from io import BytesIO
import os
import json
import shlex
import sys
import time
import uuid

from lintian_brush.vcs import determine_browser_url

from prometheus_client import (
    Counter,
    Gauge,
    Histogram,
    push_to_gateway,
    REGISTRY,
)

from silver_platter.proposal import (
    iter_all_mps,
    )
from silver_platter.utils import (
    open_branch,
    BranchMissing,
    BranchUnavailable,
    )

from . import (
    state,
    )
from .config import read_config
from .prometheus import setup_metrics
from .pubsub import Topic, pubsub_handler, pubsub_reader
from .trace import note, warning
from .vcs import (
    LocalVcsManager,
    get_run_diff,
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

    def set_mps_per_maintainer(self, mps_per_maintainer):
        raise NotImplementedError(self.set_mps_per_maintainer)

    def check_allowed(self, maintainer_email):
        raise NotImplementedError(self.allowed)

    def inc(self, maintainer_email):
        raise NotImplementedError(self.inc)


class MaintainerRateLimiter(RateLimiter):

    def __init__(self, max_mps_per_maintainer=None):
        self._max_mps_per_maintainer = max_mps_per_maintainer
        self._open_mps_per_maintainer = None

    def set_mps_per_maintainer(self, mps_per_maintainer):
        self._open_mps_per_maintainer = mps_per_maintainer['open']

    def check_allowed(self, maintainer_email):
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

    def inc(self, maintainer_email):
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
        self._open_mps_per_maintainer = None
        self._merged_mps_per_maintainer = None

    def check_allowed(self, email):
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

    def inc(self, maintainer_email):
        if self._open_mps_per_maintainer is None:
            return
        self._open_mps_per_maintainer.setdefault(maintainer_email, 0)
        self._open_mps_per_maintainer[maintainer_email] += 1

    def set_mps_per_maintainer(self, mps_per_maintainer):
        self._open_mps_per_maintainer = mps_per_maintainer['open']
        self._merged_mps_per_maintainer = mps_per_maintainer['merged']


class PublishFailure(Exception):

    def __init__(self, mode, code, description):
        self.mode = mode
        self.code = code
        self.description = description


def select_reviewers(maintainer_email, uploader_emails):
    # TODO(jelmer): Select some reviewers
    return None


async def publish_one(
        suite, pkg, command, subworker_result, main_branch_url,
        mode, log_id, maintainer_email, vcs_manager, branch_name,
        topic_merge_proposal, rate_limiter, dry_run=False,
        require_binary_diff=False, possible_hosters=None,
        possible_transports=None, allow_create_proposal=None, reviewers=None):
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
            'can not find local branch for %s / %s' % (pkg, branch_name))

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
        'reviewers': reviewers,
        'require-binary-diff': require_binary_diff,
        'allow_create_proposal': allow_create_proposal}

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


async def export_stats(db):
    while True:
        async with db.acquire() as conn:
            ready_count = {}
            async for (run, maintainer_email, uploader_emails, main_branch_url,
                       publish_mode, update_changelog, command,
                       ) in state.iter_publish_ready(conn):
                ready_count.setdefault((run.review_status, publish_mode), 0)
                ready_count[(run.review_status, publish_mode)] += 1

            for (review_status, publish_mode), count in ready_count.items():
                publish_ready_count.labels(
                    review_status=review_status,
                    publish_mode=publish_mode).set(count)

            push_count = await state.get_successful_push_count(conn)
            successful_push_count.set(push_count)

        # Every 30 minutes
        await asyncio.sleep(60 * 30)


async def publish_pending_new(db, rate_limiter, vcs_manager,
                              topic_publish, topic_merge_proposal,
                              dry_run=False, reviewed_only=False,
                              push_limit=None, require_binary_diff=False):
    start = time.time()
    possible_hosters = []
    possible_transports = []

    if reviewed_only:
        review_status = ['approved']
    else:
        review_status = ['approved', 'unreviewed']

    async with db.acquire() as conn1, db.acquire() as conn:
        async for (run, maintainer_email, uploader_emails, main_branch_url,
                   publish_mode, update_changelog,
                   command) in state.iter_publish_ready(
                       conn1, review_status=review_status):
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
                    require_binary_diff=require_binary_diff,
                    force=False)
            if actual_mode == MODE_PUSH and push_limit is not None:
                push_limit -= 1

    note('Done publishing pending changes; duration: %.2fs' % (
         time.time() - start))


async def publish_from_policy(
        conn, rate_limiter, vcs_manager, run, maintainer_email,
        uploader_emails, main_branch_url, topic_publish, topic_merge_proposal,
        mode, update_changelog, command, possible_hosters=None,
        possible_transports=None, dry_run=False, require_binary_diff=False,
        force=False):
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

    reviewers = select_reviewers(maintainer_email, uploader_emails)
    note('Publishing %s / %r (mode: %s)', run.package, run.command, mode)
    try:
        proposal_url, branch_name, is_new = await publish_one(
            run.suite, run.package, run.command, run.result,
            main_branch_url, mode, run.id, maintainer_email,
            vcs_manager=vcs_manager, branch_name=run.branch_name,
            topic_merge_proposal=topic_merge_proposal,
            dry_run=dry_run, require_binary_diff=require_binary_diff,
            possible_hosters=possible_hosters,
            possible_transports=possible_transports,
            reviewers=reviewers, rate_limiter=rate_limiter)
    except PublishFailure as e:
        if e.code == 'merge-conflict':
            note('Merge proposal would cause conflict; restarting.')
            await do_schedule(
                conn, run.package, run.suite,
                requestor='publisher (pre-creation merge conflict)')
            return
        if e.code == 'missing-binary-diff':
            note('Missing binary diff; requesting control run.')
            await do_schedule_control(
                conn, run.package, run.main_branch_revision,
                requestor='publisher (missing binary diff)')
            return
        code = e.code
        description = e.description
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
        publish_id=publish_id)

    if code == 'success' and mode == MODE_PUSH:
        # TODO(jelmer): Call state.update_branch_status() for the
        # main branch URL
        pass

    if code == 'success':
        publish_delay = datetime.now() - run.times[1]
        publish_latency.observe(publish_delay.total_seconds())
    else:
        publish_delay = None

    topic_entry = {
        'id': publish_id,
         'package': run.package,
         'suite': run.suite,
         'proposal_url': proposal_url or None,
         'mode': mode,
         'main_branch_url': main_branch_url,
         'main_branch_browse_url': determine_browser_url(
             None, main_branch_url),
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
        dry_run=False, allow_create_proposal=True, require_binary_diff=False):
    reviewers = select_reviewers(maintainer_email, uploader_emails)
    async with db.acquire() as conn:
        try:
            proposal_url, branch_name, is_new = await publish_one(
                run.suite, run.package, run.command, run.result,
                run.branch_url, mode, run.id, maintainer_email, vcs_manager,
                run.branch_name, dry_run=dry_run,
                require_binary_diff=require_binary_diff,
                possible_hosters=None, possible_transports=None,
                reviewers=reviewers,
                allow_create_proposal=allow_create_proposal,
                topic_merge_proposal=topic_merge_proposal,
                rate_limiter=rate_limiter)
        except PublishFailure as e:
            await state.store_publish(
                conn, run.package, run.branch_name,
                run.main_branch_revision,
                run.revision, e.mode, e.code, e.description,
                None, publish_id=publish_id)
            topic_publish.publish({
                'id': publish_id,
                'mode': e.mode,
                'result_code': e.code,
                'description': e.description,
                'package': run.package,
                'suite': run.suite,
                'main_branch_url': run.branch_url,
                'main_branch_browse_url': determine_browser_url(
                     None, run.branch_url),
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
            publish_id=publish_id)

        publish_delay = run.times[1] - datetime.now()
        publish_latency.observe(publish_delay.total_seconds())

        topic_publish.publish(
            {'id': publish_id,
             'package': run.package,
             'suite': run.suite,
             'proposal_url': proposal_url or None,
             'mode': mode,
             'main_branch_url': run.branch_url,
             'main_branch_browse_url': determine_browser_url(
                 None, run.branch_url),
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

        run = await state.get_last_unabsorbed_run(conn, package.name, suite)
        if run is None:
            return web.json_response({}, status=400)
        note('Handling request to publish %s/%s', package.name, suite)

    publish_id = str(uuid.uuid4())

    request.loop.create_task(publish_and_store(
        request.app.db, request.app.topic_publish,
        request.app.topic_merge_proposal, publish_id, run, mode,
        package.maintainer_email, package.uploader_emails,
        vcs_manager=vcs_manager, rate_limiter=rate_limiter, dry_run=dry_run,
        allow_create_proposal=True,
        require_binary_diff=False))

    return web.json_response(
        {'run_id': run.id, 'mode': mode, 'publish_id': publish_id},
        status=202)


async def git_backend(request):
    package = request.match_info['package']
    subpath = request.match_info['subpath']

    args = ['/usr/lib/git-core/git-http-backend']
    repo = request.app.vcs_manager.get_repository(package, 'git')
    if repo is None:
        raise web.HTTPNotFound()
    local_path = repo.user_transport.local_abspath('.')
    full_path = local_path + '/' + subpath
    env = {
        'GIT_HTTP_EXPORT_ALL': 'true',
        'REQUEST_METHOD': request.method,
        'REMOTE_ADDR': request.remote,
        'CONTENT_TYPE': request.content_type,
        'PATH_TRANSLATED': full_path,
        'QUERY_STRING': request.query_string,
        # REMOTE_USER is not set
        }

    for key, value in request.headers.items():
        env['HTTP_' + key.replace('-', '_').upper()] = value

    for name in ['HTTP_CONTENT_ENCODING', 'HTTP_CONTENT_LENGTH']:
        try:
            del env[name]
        except KeyError:
            pass

    p = await asyncio.create_subprocess_exec(
        *args, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE,
        env=env, stdin=asyncio.subprocess.PIPE)

    stdin = await request.read()

    # TODO(jelmer): Stream output, rather than reading everything into a buffer
    # and then sending it on.

    (stdout, stderr) = await p.communicate(stdin)
    if stderr:
        warning('Git %s error: %s', subpath, stderr.decode())
        return web.Response(
            status=400, reason='Bad Request', body=stderr)

    b = BytesIO(stdout)

    headers = parse_headers(b)
    status = headers.get('Status')
    if status:
        del headers['Status']
        (status_code, status_reason) = status.split(b' ', 1)
        status_code = status_code.decode()
        status_reason = status_reason.decode()
    else:
        status_code = 200
        status_reason = 'OK'

    response = web.StreamResponse(
        headers=headers.items(), status=status_code,
        reason=status_reason)
    await response.prepare(request)

    await response.write(b.read())

    await response.write_eof()

    return response


async def bzr_backend(request):
    vcs_manager = request.app.vcs_manager
    package = request.match_info['package']
    subpath = request.match_info['subpath']
    repo = vcs_manager.get_repository(package, 'bzr')
    if repo is None:
        raise web.HTTPNotFound()
    local_path = repo.user_transport.local_abspath('.')
    full_path = os.path.join(local_path, subpath)
    if not os.path.exists(full_path):
        raise web.HTTPNotFound()
    if not os.path.isfile(full_path):
        return web.Response(body=b'This is a directory.')
    return web.FileResponse(full_path)


async def get_vcs_type(request):
    package = request.match_info['package']
    vcs_type = request.app.vcs_manager.get_vcs_type(package)
    if vcs_type is None:
        raise web.HTTPNotFound()
    return web.Response(body=vcs_type.encode('utf-8'))


async def run_web_server(listen_addr, port, rate_limiter, vcs_manager, db,
                         topic_merge_proposal, topic_publish, dry_run=False,
                         require_binary_diff=False, push_limit=None):
    app = web.Application()
    app.vcs_manager = vcs_manager
    app.db = db
    app.rate_limiter = rate_limiter
    app.topic_publish = topic_publish
    app.topic_merge_proposal = topic_merge_proposal
    app.dry_run = dry_run
    app.push_limit = push_limit
    app.require_binary_diff = require_binary_diff
    setup_metrics(app)
    app.router.add_post("/{suite}/{package}/publish", publish_request)
    app.router.add_get("/diff/{run_id}", diff_request)
    app.router.add_route("*", "/git/{package}/{subpath:.*}", git_backend)
    app.router.add_route(
        "*", "/bzr/{package}/{subpath:.*}", bzr_backend)
    app.router.add_get(
        '/vcs-type/{package}', get_vcs_type)
    app.router.add_get(
        '/ws/publish', functools.partial(pubsub_handler, topic_publish))
    app.router.add_get(
        '/ws/merge-proposal', functools.partial(
            pubsub_handler, topic_merge_proposal))
    app.router.add_post('/scan', scan_request)
    app.router.add_post('/autopublish', autopublish_request)
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
                request.app.dry_run)
    request.loop.create_task(scan())
    return web.Response(status=202, text="Scan started.")


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
        topic_merge_proposal, topic_publish, auto_publish=True,
        reviewed_only=False, push_limit=None, require_binary_diff=False):
    while True:
        async with db.acquire() as conn:
            await check_existing(
                conn, rate_limiter, vcs_manager, topic_merge_proposal, dry_run)
        await asyncio.sleep(interval)
        if auto_publish:
            await publish_pending_new(
                db, rate_limiter, vcs_manager, dry_run=dry_run,
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


async def check_existing(conn, rate_limiter, vcs_manager, topic_merge_proposal,
                         dry_run=False):
    mps_per_maintainer = {'open': {}, 'closed': {}, 'merged': {}}
    possible_transports = []
    status_count = {'open': 0, 'closed': 0, 'merged': 0}

    async def update_proposal_status(mp, status, revision, package_name):
        if status == 'merged':
            try:
                merged_by = mp.get_merged_by()
            except (NotImplementedError, AttributeError):
                merged_by = None
            try:
                merged_at = mp.get_merged_at().replace(tzinfo=None)
            except (NotImplementedError, AttributeError):
                merged_at = None
        else:
            merged_by = None
            merged_at = None
        await state.set_proposal_info(
            conn, mp.url, status, revision, package_name, merged_by,
            merged_at)
        topic_merge_proposal.publish(
           {'url': mp.url, 'status': status, 'package': package_name,
            'merged_by': merged_by, 'merged_at': str(merged_at)})

    for hoster, mp, status in iter_all_mps():
        status_count[status] += 1
        try:
            (revision, old_status, package_name,
                maintainer_email) = await state.get_proposal_info(conn, mp.url)
        except KeyError:
            try:
                revision = open_branch(
                    mp.get_source_branch_url(),
                    possible_transports=possible_transports).last_revision()
            except (BranchMissing, BranchUnavailable):
                revision = None
            old_status = None
            maintainer_email = None
            package_name = None
        if maintainer_email is None:
            target_branch_url = mp.get_target_branch_url()
            package = await state.get_package_by_branch_url(
                conn, target_branch_url)
            if package is None:
                warning('No package known for %s (%s)',
                        mp.url, target_branch_url)
                package_name = None
            else:
                maintainer_email = package.maintainer_email
                package_name = package.name
        if old_status != status:
            await update_proposal_status(mp, status, revision, package_name)
        if maintainer_email is not None:
            mps_per_maintainer[status].setdefault(maintainer_email, 0)
            mps_per_maintainer[status][maintainer_email] += 1
        if status != 'open':
            continue
        mp_run = await state.get_merge_proposal_run(conn, mp.url)
        if mp_run is None:
            warning('Unable to find local metadata for %s, skipping.', mp.url)
            continue

        last_run = await state.get_last_unabsorbed_run(
            conn, mp_run.package, mp_run.suite)
        if last_run is None:
            # A new run happened since the last, but there was nothing to
            # do.
            note('%s: Last run did not produce any changes, '
                 'closing proposal.', mp.url)
            await update_proposal_status(mp, 'closed', revision, package_name)
            mp.close()
            continue

        if last_run.result_code not in ('success', 'nothing-to-do'):
            note('%s: Last run failed (%s). Not touching merge proposal.',
                 mp.url, last_run.result_code)
            continue

        if last_run != mp_run:
            publish_id = str(uuid.uuid4())
            note('%s needs to be updated.', mp.url)
            try:
                mp_url, branch_name, is_new = await publish_one(
                    last_run.suite, last_run.package, last_run.command,
                    last_run.result, last_run.branch_url, MODE_PROPOSE,
                    last_run.id, maintainer_email,
                    vcs_manager=vcs_manager, branch_name=mp_run.branch_name,
                    dry_run=dry_run, require_binary_diff=False,
                    allow_create_proposal=True, reviewers=None,
                    topic_merge_proposal=topic_merge_proposal,
                    rate_limiter=rate_limiter)
            except PublishFailure as e:
                note('%s: Updating merge proposal failed: %s (%s)',
                     mp.url, e.code, e.description)
                await state.store_publish(
                    conn, last_run.package, mp_run.branch_name,
                    last_run.main_branch_revision,
                    last_run.revision, e.mode, e.code,
                    e.description, mp.url,
                    publish_id=publish_id)
            else:
                await state.store_publish(
                    conn, last_run.package, branch_name,
                    last_run.main_branch_revision,
                    last_run.revision, MODE_PROPOSE, 'success',
                    'Succesfully updated', mp_url,
                    publish_id=publish_id)

                assert not is_new, "Intended to update proposal %r" % mp_url
        else:
            # It may take a while for the 'conflicted' bit on the proposal to
            # be refreshed, so only check it if we haven't made any other
            # changes.
            if is_conflicted(mp):
                note('%s is conflicted. Rescheduling.', mp.url)
                await state.add_to_queue(
                    conn, mp_run.package, shlex.split(mp_run.command),
                    mp_run.suite, offset=-2, refresh=True,
                    requestor='publisher (merge conflict)')

    for status, count in status_count.items():
        merge_proposal_count.labels(status=status).set(count)

    rate_limiter.set_mps_per_maintainer(mps_per_maintainer)
    for maintainer_email, count in mps_per_maintainer['open'].items():
        open_proposal_count.labels(maintainer=maintainer_email).set(count)


async def listen_to_runner(db, rate_limiter, vcs_manager, runner_url,
                           topic_publish, topic_merge_proposal, dry_run=False,
                           require_binary_diff=False):
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
            require_binary_diff=require_binary_diff,
            force=True)
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
                    for run in await state.iter_last_runs(
                            main_branch_revision=run.revision):
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
                reviewed_only=args.reviewed_only,
                push_limit=args.push_limit,
                require_binary_diff=args.require_binary_diff)),
            loop.create_task(
                run_web_server(
                    args.listen_address, args.port, rate_limiter,
                    vcs_manager, db, topic_merge_proposal, topic_publish,
                    args.dry_run, args.require_binary_diff,
                    push_limit=args.push_limit)),
            loop.create_task(export_stats(db)),
        ]
        if args.runner_url and not args.reviewed_only:
            tasks.append(loop.create_task(
                listen_to_runner(
                    db, rate_limiter, vcs_manager,
                    args.runner_url, topic_publish,
                    topic_merge_proposal, dry_run=args.dry_run,
                    require_binary_diff=args.require_binary_diff)))
        loop.run_until_complete(asyncio.gather(*tasks))


if __name__ == '__main__':
    sys.exit(main(sys.argv))
