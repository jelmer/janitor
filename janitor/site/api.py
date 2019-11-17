#!/usr/bin/python3

from aiohttp import web, ClientSession, ContentTypeError, ClientConnectorError
import urllib.parse

from janitor.policy import apply_policy
from janitor import state, SUITES, DEFAULT_BUILD_ARCH
from . import (
    env,
    highlight_diff,
    run_changes_filename,
    )



from breezy.git.urls import git_url_to_bzr_url


async def handle_policy(request):
    package = request.match_info['package']
    async with request.app.db.acquire() as conn:
        package = await state.get_package(conn, package)
        if package is None:
            return web.json_response(
                {'reason': 'Package not found'}, status=404)
    suite_policies = {}
    for suite in SUITES:
        (publish_policy, changelog_policy, committer) = apply_policy(
            request.app.policy_config, suite, package.name,
            package.maintainer_email, package.uploader_emails)
        suite_policies[suite] = {
            'publish_policy': publish_policy,
            'changelog_policy': changelog_policy,
            'committer': committer}
    response_obj = {'by_suite': suite_policies}
    return web.json_response(response_obj)


async def handle_publish(request):
    publisher_url = request.app.publisher_url
    package = request.match_info['package']
    suite = request.match_info['suite']
    post = await request.post()
    mode = post.get('mode', 'push-derived')
    if mode not in ('push-derived', 'push', 'propose', 'attempt-push'):
        return web.json_response(
            {'error': 'Invalid mode', 'mode': mode}, status=400)
    url = urllib.parse.urljoin(
        publisher_url, '%s/%s/publish' % (suite, package))
    try:
        async with request.app.http_client_session.post(
                url, data={'mode': mode}) as resp:
            if resp.status in (200, 202):
                return web.json_response(
                    await resp.json(), status=resp.status)
            else:
                return web.json_response(await resp.json(), status=400)
    except ContentTypeError as e:
        return web.json_response(
            {'reason': 'publisher returned error %d' % e.code},
            status=400)
    except ClientConnectorError:
        return web.json_response(
            {'reason': 'unable to contact publisher'},
            status=400)


async def get_package_from_gitlab_webhook(conn, body):
    vcs_url = body['project']['git_http_url']
    for url in [
            git_url_to_bzr_url(vcs_url, ref=body['ref'].encode()),
            git_url_to_bzr_url(vcs_url)]:
        package = await state.get_package_by_branch_url(conn, vcs_url)
        if package is not None:
            break
    else:
        return None
    return package


async def schedule(conn, policy, package, suite, offset=None,
                   refresh=False, requestor=None):
    from ..schedule import (
        estimate_duration, full_command, DEFAULT_SCHEDULE_OFFSET)
    if offset is None:
        offset = DEFAULT_SCHEDULE_OFFSET
    unused_publish_mode, update_changelog, committer = apply_policy(
        policy, suite,
        package.name, package.maintainer_email, package.uploader_emails)
    command = full_command(suite, update_changelog)
    estimated_duration = await estimate_duration(conn, package.name, suite)
    await state.add_to_queue(
        conn, package.name, command, suite, offset,
        estimated_duration=estimated_duration, refresh=refresh,
        requestor=requestor, committer=committer)
    return offset, estimated_duration


async def handle_webhook(request):
    if request.headers.get('Content-Type') != 'application/json':
        template = env.get_template('webhook.html')
        return web.Response(
            content_type='text/html', text=await template.render_async(),
            headers={'Cache-Control': 'max-age=600'})
    if request.headers['X-Gitlab-Event'] != 'Push Hook':
        return web.json_response({}, status=200)
    body = await request.json()
    async with request.app.db.acquire() as conn:
        package = await get_package_from_gitlab_webhook(conn, body)
        if package is None:
            return web.Response(
                body=('VCS URL %s unknown' % body['project']['git_http_url']),
                status=404)
        # TODO(jelmer: If nothing found, then maybe fall back to
        # urlutils.basename(body['project']['path_with_namespace'])?
        requestor = 'GitLab Push hook for %s' % body['project']['git_http_url']
        for suite in SUITES:
            await schedule(
                conn, request.app.policy_config, package, suite,
                requestor=requestor)
        return web.json_response({})


async def handle_schedule(request):
    package = request.match_info['package']
    suite = request.match_info['suite']
    if suite not in SUITES:
        return web.json_response(
            {'error': 'Unknown suite', 'suite': suite}, status=404)
    post = await request.post()
    offset = post.get('offset')
    try:
        refresh = bool(int(post.get('refresh', '0')))
    except ValueError:
        return web.json_response(
            {'error': 'invalid boolean for refresh'}, status=400)
    async with request.app.db.acquire() as conn:
        package = await state.get_package(conn, package)
        if package is None:
            return web.json_response(
                {'reason': 'Package not found'}, status=404)
        if request.debsso_email:
            requestor = request.debsso_email
        else:
            requestor = 'user from web UI'
        if package.branch_url is None:
            return web.json_response(
                {'reason': 'No branch URL defined.'}, status=400)
        offset, estimated_duration = await schedule(
            conn, request.app.policy_config, package, suite, offset, refresh,
            requestor=requestor)
        (queue_position, queue_wait_time) = await state.get_queue_position(
            conn, suite, package.name)
    response_obj = {
        'package': package.name,
        'suite': suite,
        'offset': offset,
        'estimated_duration_seconds': estimated_duration.total_seconds(),
        'queue_position': queue_position,
        'queue_wait_time': queue_wait_time.total_seconds(),
        }
    return web.json_response(response_obj)


async def handle_package_list(request):
    name = request.match_info.get('package')
    response_obj = []
    async with request.app.db.acquire() as conn:
        for package in await state.iter_packages(conn, package=name):
            if not name and package.removed:
                continue
            response_obj.append({
                'name': package.name,
                'maintainer_email': package.maintainer_email,
                'branch_url': package.branch_url})
    return web.json_response(
            response_obj, headers={'Cache-Control': 'max-age=600'})


async def handle_packagename_list(request):
    response_obj = []
    async with request.app.db.acquire() as conn:
        for package in await state.iter_packages(conn):
            if package.removed:
                continue
            response_obj.append(package.name)
    return web.json_response(
        response_obj, headers={'Cache-Control': 'max-age=600'})


async def handle_merge_proposal_list(request):
    response_obj = []
    async with request.app.db.acquire() as conn:
        for package, url, status in await state.iter_proposals(
                conn,
                request.match_info.get('package'),
                request.match_info.get('suite')):
            response_obj.append({
                'package': package,
                'url': url,
                'status': status})
    return web.json_response(response_obj)


async def handle_queue(request):
    limit = request.query.get('limit')
    if limit is not None:
        limit = int(limit)
    response_obj = []
    async with request.app.db.acquire() as conn:
        async for entry in state.iter_queue(
                conn, limit=limit):
            response_obj.append({
                'queue_id': entry.id,
                'branch_url': entry.branch_url,
                'package': entry.package,
                'env': entry.env,
                'command': entry.command})
    return web.json_response(
        response_obj, headers={'Cache-Control': 'max-age=60'})


async def handle_diff(request):
    run_id = request.match_info['run_id']
    publisher_url = request.app.publisher_url
    url = urllib.parse.urljoin(publisher_url, 'diff/%s' % run_id)
    try:
        async with request.app.http_client_session.get(url) as resp:
            if resp.status == 200:
                diff = await resp.read()
                for accept in request.headers.get('ACCEPT', '').split(','):
                    if accept in ('text/x-diff', 'text/plain'):
                        return web.Response(
                            body=diff,
                            content_type='text/x-diff',
                            headers={'Cache-Control': 'max-age=3600'})
                    if accept == 'text/html':
                        return web.Response(
                            text=highlight_diff(
                                diff.decode('utf-8', 'replace')),
                            content_type='text/html',
                            headers={'Cache-Control': 'max-age=3600'})
                raise web.HTTPNotAcceptable(
                    text='Acceptable content types: '
                         'text/html, text/x-diff')
            else:
                return web.Response(body=await resp.read(), status=400)
    except ContentTypeError as e:
        return web.Response(
            'publisher returned error %d' % e.code,
            status=400)
    except ClientConnectorError:
        return web.json_response(
            'unable to contact publisher',
            status=400)


async def handle_debdiff(request):
    run_id = request.match_info['run_id']
    async with request.app.db.acquire() as conn:
        run = await state.get_run(conn, run_id)
        if run is None:
            raise web.HTTPNotFound(text='No such run: %s' % run_id)
        unchanged_run = await state.get_unchanged_run(
            conn, run.main_branch_revision)
        if unchanged_run is None:
            raise web.HTTPNotFound(
                text='No matching unchanged build for %s' % run_id)
    runner_url = request.app.runner_url
    url = urllib.parse.urljoin(runner_url, 'debdiff')
    payload = {
        'old_suite': 'unchanged',
        'new_suite': run.suite,
        'old_changes_filename': run_changes_filename(unchanged_run),
        'new_changes_filename': run_changes_filename(run),
    }
    try:
        async with request.app.http_client_session.post(
                url, data=payload) as resp:
            if resp.status == 200:
                diff = await resp.read()
                return web.Response(
                    body=diff,
                    content_type=resp.content_type,
                    headers={'Cache-Control': 'max-age=3600'})
            else:
                return web.Response(body=await resp.read(), status=400)
    except ContentTypeError as e:
        return web.Response(
            'runner returned error %d' % e.code,
            status=400)
    except ClientConnectorError:
        return web.json_response(
            'unable to contact runner',
            status=400)


async def handle_run_post(request):
    run_id = request.match_info['run_id']
    post = await request.post()
    review_status = post.get('review-status')
    if review_status:
        async with request.app.db.acquire() as conn:
            review_status = review_status.lower()
            if review_status == 'reschedule':
                run = await state.get_run(conn, run_id)
                package = await state.get_package(conn, run.package)
                await schedule(
                    conn, request.app.policy_config, package, run.suite,
                    refresh=True, requestor='reviewer')
                review_status = 'rejected'
            await state.set_run_review_status(conn, run_id, review_status)
    return web.json_response(
            {'review-status': review_status})


async def handle_run(request):
    package = request.match_info.get('package')
    run_id = request.match_info.get('run_id')
    limit = request.query.get('limit')
    if limit is not None:
        limit = int(limit)
    response_obj = []
    async with request.app.db.acquire() as conn:
        async for run in state.iter_runs(
                    conn, package, run_id=run_id, limit=limit):
            if run.build_version:
                build_info = {
                    'version': str(run.build_version),
                    'distribution': run.build_distribution}
            else:
                build_info = None
            (start_time, finish_time) = run.times
            response_obj.append({
                'run_id': run.id,
                'start_time': start_time.isoformat(),
                'finish_time': finish_time.isoformat(),
                'command': run.command,
                'description': run.description,
                'package': run.package,
                'build_info': build_info,
                'result_code': run.result_code,
                'branch_name': run.branch_name,
                })
    return web.json_response(
        response_obj, headers={'Cache-Control': 'max-age=600'})


async def handle_package_branch(request):
    response_obj = []
    async with request.app.db.acquire() as conn:
        for (name, branch_url, revision, last_scanned, description) in (
                await state.iter_package_branches(conn)):
            response_obj.append({
                'name': name,
                'branch_url': branch_url,
                'revision': revision,
                'last_scanned': last_scanned.isoformat()
                if last_scanned else None,
                'description': description,
                })
    return web.json_response(
        response_obj, headers={'Cache-Control': 'max-age=60'})


async def handle_published_packages(request):
    suite = request.match_info['suite']
    async with request.app.db.acquire() as conn:
        response_obj = []
        for package, build_version, archive_version in (
                await state.iter_published_packages(conn, suite)):
            response_obj.append({
                'package': package,
                'build_version': build_version,
                'archive_version': archive_version})
    return web.json_response(response_obj)


async def handle_index(request):
    template = env.get_template('api-index.html')
    return web.Response(
        content_type='text/html', text=await template.render_async(),
        headers={'Cache-Control': 'max-age=600'})


async def handle_global_policy(request):
    return web.Response(
        content_type='text/protobuf', text=str(request.app.policy_config),
        headers={'Cache-Control': 'max-age=60'})


async def forward_to_runner(runner_url, path):
    url = urllib.parse.urljoin(runner_url, path)
    try:
        async with request.app.http_client_session.get(url) as resp:
            return web.json_response(
                await resp.json(), status=resp.status)
    except ContentTypeError as e:
        return web.json_response({
            'reason': 'runner returned error %s' % e},
            status=400)
    except ClientConnectorError:
        return web.json_response({
            'reason': 'unable to contact runner'},
            status=500)


async def handle_runner_status(request):
    return await forward_to_runner(request.app.runner_url, 'status')


async def handle_runner_log_index(request):
    run_id = request.match_info['run_id']
    return await forward_to_runner(request.app.runner_url, 'log/%s' % run_id)


async def handle_runner_log(request):
    run_id = request.match_info['run_id']
    filename = request.match_info['filename']
    url = urllib.parse.urljoin(
        request.app.runner_url, 'log/%s/%s' % (run_id, filename))
    try:
        async with request.app.http_client_session.get(url) as resp:
            body = await resp.read()
            return web.Response(body=body, status=resp.status)
    except ContentTypeError as e:
        return web.Response(
            text='runner returned error %s' % e,
            status=400)
    except ClientConnectorError:
        return web.Response(
            text='unable to contact runner',
            status=500)


async def handle_publish_id(request):
    publish_id = request.match_info['publish_id']
    async with request.app.db.acquire() as conn:
        (package, branch_name, main_branch_revision, revision, mode,
         merge_proposal_url, result_code,
         description) = await state.get_publish(conn, publish_id)
    return web.json_response({
        'package': package,
        'branch': branch_name,
        'main_branch_revision': main_branch_revision,
        'revision': revision,
        'mode': mode,
        'merge_proposal_url': merge_proposal_url,
        'result_code': result_code,
        'description': description,
        })


async def handle_report(request):
    suite = request.match_info['suite']
    report = {}
    async with request.app.db.acquire() as conn:
        async for (package, command, build_version, result_code, context,
                   start_time, log_id, revision, result, branch_name, suite,
                   maintainer_email, uploader_emails, branch_url,
                   main_branch_revision, review_status
                   ) in state.iter_publish_ready(
                       conn, suite=suite):
            data = {
                'timestamp': start_time.isoformat(),
            }
            if suite == 'lintian-fixes':
                data['fixed-tags'] = []
                for entry in result['applied']:
                    data['fixed-tags'].extend(entry['fixed_lintian_tags'])
            if suite in ('fresh-releases', 'fresh-snapshots'):
                data['upstream-version'] = result.get('upstream_version')
                data['old-upstream-version'] = result.get(
                    'old_upstream_version')
            report[package] = data
    return web.json_response(
        report,
        headers={'Cache-Control': 'max-age=600'},
        status=200)


async def handle_publish_ready(request):
    suite = request.match_info.get('suite')
    review_status = request.query.get('review-status')
    limit = request.query.get('limit', 200)
    if limit:
        limit = int(limit)
    else:
        limit = None
    for_publishing = set()
    ret = []
    async with request.app.db.acquire() as conn:
        async for (package, command, build_version, result_code, context,
                   start_time, log_id, revision, result, branch_name, suite,
                   maintainer_email, uploader_emails, branch_url,
                   main_branch_revision, review_status
                   ) in state.iter_publish_ready(
                       conn, suite=suite, review_status=review_status):
            (publish_policy, changelog_policy, committer) = apply_policy(
                request.app.policy_config, suite, package,
                maintainer_email, uploader_emails)
            if publish_policy in (
                    'propose', 'attempt-push', 'push-derived', 'push'):
                for_publishing.add(log_id)
            ret.append((package, log_id))
    ret.sort(key=lambda x: (x[1] not in for_publishing, x[0]))
    return web.json_response(ret, status=200)


def create_app(db, publisher_url, runner_url, policy_config):
    app = web.Application()
    app.http_client_session = ClientSession()
    app.db = db
    app.policy_config = policy_config
    app.publisher_url = publisher_url
    app.runner_url = runner_url
    app.router.add_get(
        '/pkgnames', handle_packagename_list, name='api-package-names')
    app.router.add_get(
        '/pkg', handle_package_list, name='api-package-list')
    app.router.add_get(
        '/pkg/{package}', handle_package_list, name='api-package')
    app.router.add_get(
        '/pkg/{package}/merge-proposals',
        handle_merge_proposal_list, name='api-package-merge-proposals')
    app.router.add_get(
        '/pkg/{package}/policy',
        handle_policy, name='api-package-policy')
    app.router.add_post(
        '/{suite}/pkg/{package}/publish',
        handle_publish,
        name='api-package-publish')
    app.router.add_post(
        '/{suite}/pkg/{package}/schedule', handle_schedule,
        name='api-package-schedule')
    app.router.add_get(
        '/merge-proposals', handle_merge_proposal_list,
        name='api-merge-proposals')
    app.router.add_get('/queue', handle_queue, name='api-queue')
    app.router.add_get('/run', handle_run, name='api-run-list')
    app.router.add_get('/run/{run_id}', handle_run, name='api-run')
    app.router.add_post(
        '/run/{run_id}', handle_run_post, name='api-run-update')
    app.router.add_get('/run/{run_id}/diff', handle_diff, name='api-run-diff')
    app.router.add_get(
        '/run/{run_id}/debdiff',
        handle_debdiff,
        name='api-run-debdiff')
    app.router.add_get(
        '/pkg/{package}/run', handle_run, name='api-package-run-list')
    app.router.add_get(
        '/pkg/{package}/run/{run_id}', handle_run, name='api-package-run')
    app.router.add_post(
        '/pkg/{package}/run/{run_id}', handle_run_post,
        name='api-package-run')
    app.router.add_get(
        '/pkg/{package}/run/{run_id}/diff',
        handle_diff, name='api-package-run-diff')
    app.router.add_get(
        '/pkg/{package}/run/{run_id}/debdiff',
        handle_debdiff,
        name='api-package-run-debdiff')
    app.router.add_get(
        '/package-branch', handle_package_branch, name='api-package-branch')
    app.router.add_get(
        '/', handle_index, name='api-index')
    app.router.add_get(
        '/{suite}/published-packages', handle_published_packages,
        name='api-published-packages')
    app.router.add_get('/policy', handle_global_policy, name='api-policy')
    app.router.add_post('/webhook', handle_webhook, name='api-webhook')
    app.router.add_get('/webhook', handle_webhook, name='api-webhook-help')
    app.router.add_get(
        '/publish/{publish_id}', handle_publish_id, name='publish-details')
    app.router.add_get(
        '/runner/status', handle_runner_status,
        name='api-runner-status')
    app.router.add_get(
        '/runner/log/{run_id}',
        handle_runner_log_index,
        name='api-runner-log-list')
    app.router.add_get(
        '/runner/log/{run_id}/{filename}',
        handle_runner_log, name='api-runner-log')
    app.router.add_get(
        '/{suite:' + '|'.join(SUITES) + '}/report',
        handle_report, name='api-report')
    app.router.add_get(
        '/publish-ready',
        handle_publish_ready, name='api-publish-ready')
    app.router.add_get(
        '/{suite:' + '|'.join(SUITES) + '}/publish-ready',
        handle_publish_ready, name='api-publish-ready-suite')
    # TODO(jelmer): Previous runs (iter_previous_runs)
    # TODO(jelmer): Last successes (iter_last_successes)
    # TODO(jelmer): Last runs (iter_last_runs)
    # TODO(jelmer): Build failures (iter_build_failures)
    return app
