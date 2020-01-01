#!/usr/bin/python3

from aiohttp import web, ClientSession, ContentTypeError, ClientConnectorError
import urllib.parse

from janitor import state, SUITE_REGEX
from . import (
    env,
    highlight_diff,
    get_debdiff,
    DebdiffRetrievalError,
    )
from ..schedule import do_schedule


from breezy.git.urls import git_url_to_bzr_url


async def handle_policy(request):
    package = request.match_info['package']
    suite_policies = {}
    async with request.app.db.acquire() as conn:
        async for unused_package, suite, policy in state.iter_publish_policy(
                conn, package):
            suite_policies[suite] = policy
    if not suite_policies:
        return web.json_response(
            {'reason': 'Package not found'}, status=404)
    for suite, (publish_policy,
                changelog_policy, command) in suite_policies.items():
        suite_policies[suite] = {
            'publish_policy': publish_policy,
            'changelog_policy': changelog_policy,
            'command': command}
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
        async for package_name, suite, policy in state.iter_publish_policy(
                package.name):
            if policy[0] == 'skip':
                continue
            await do_schedule(conn, package.name, suite, requestor=requestor)
        return web.json_response({})


async def handle_schedule(request):
    package = request.match_info['package']
    suite = request.match_info['suite']
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
        offset, estimated_duration = await do_schedule(
            conn, package.name, suite, offset, refresh,
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
                'context': entry.context,
                'command': entry.command})
    return web.json_response(
        response_obj, headers={'Cache-Control': 'max-age=60'})


async def handle_diff(request):
    try:
        run_id = request.match_info['run_id']
    except KeyError:
        async with request.app.db.acquire() as conn:
            run = await state.get_last_unabsorbed_run(
                conn, request.match_info['package'],
                request.match_info['suite'])
        run_id = run.id
    publisher_url = request.app.publisher_url
    url = urllib.parse.urljoin(publisher_url, 'diff/%s' % run_id)
    try:
        async with request.app.http_client_session.get(url) as resp:
            if resp.status == 200:
                diff = await resp.read()
                for accept in request.headers.get('ACCEPT', '*/*').split(','):
                    if accept in ('text/x-diff', 'text/plain', '*/*'):
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

    if run.build_version is None:
        raise web.HTTPNotFound(
            text='Build %s was not successful' % run_id)

    if unchanged_run.build_version is None:
        raise web.HTTPNotFound(
            text='Unchanged build %s was not successful' % unchanged_run.id)

    filter_boring = ('filter_boring' in request.query)

    try:
        debdiff = await get_debdiff(
            request.app.http_client_session, request.app.archiver_url, run,
            unchanged_run, filter_boring=filter_boring)
    except FileNotFoundError:
        raise web.HTTPNotFound(text='debdiff not calculated yet')
    except DebdiffRetrievalError as e:
        return web.json_response(
            'unable to contact archiver for debdiff: %r' % e,
            status=400)
    return web.Response(
        body=debdiff,
        content_type='text/x-diff',
        headers={'Cache-Control': 'max-age=3600'})


async def handle_run_post(request):
    run_id = request.match_info['run_id']
    post = await request.post()
    review_status = post.get('review-status')
    if review_status:
        async with request.app.db.acquire() as conn:
            review_status = review_status.lower()
            if review_status == 'reschedule':
                run = await state.get_run(conn, run_id)
                await do_schedule(
                    conn, run.package, run.suite, refresh=True,
                    requestor='reviewer')
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


async def handle_publish_scan(request):
    if request.debsso_email != 'jelmer@debian.org':
        return web.Response(text='Unauthorized', status=401)
    publisher_url = request.app.publisher_url
    url = urllib.parse.urljoin(publisher_url, '/scan')
    try:
        async with request.app.http_client_session.post(url) as resp:
            return web.Response(body=await resp.read(), status=resp.status)
    except ClientConnectorError:
        return web.Response(
            text='unable to contact publisher',
            status=400)


async def handle_publish_autopublish(request):
    if request.debsso_email != 'jelmer@debian.org':
        return web.Response(text='Unauthorized', status=401)
    publisher_url = request.app.publisher_url
    url = urllib.parse.urljoin(publisher_url, '/autopublish')
    try:
        async with request.app.http_client_session.post(url) as resp:
            return web.Response(body=await resp.read(), status=resp.status)
    except ClientConnectorError:
        return web.Response(
            text='unable to contact publisher',
            status=400)


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


async def forward_to_runner(client, runner_url, path):
    url = urllib.parse.urljoin(runner_url, path)
    try:
        async with client.get(url) as resp:
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
    return await forward_to_runner(
        request.app.http_client_session, request.app.runner_url, 'status')


async def handle_runner_log_index(request):
    run_id = request.match_info['run_id']
    return await forward_to_runner(
        request.app.http_client_session, request.app.runner_url,
        'log/%s' % run_id)


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
        publish = await state.get_publish(conn, publish_id)
        if publish is None:
            raise web.HTTPNotFound(text='no such publish: %s' % publish_id)
        (package, branch_name, main_branch_revision, revision, mode,
         merge_proposal_url, result_code,
         description) = publish
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
        async for (run, maintainer_email, uploader_emails, branch_url,
                   publish_mode, changelog_mode, command
                   ) in state.iter_publish_ready(
                       conn, suite=suite):
            data = {
                'timestamp': run.times[0].isoformat(),
            }
            if run.suite == 'lintian-fixes':
                data['fixed-tags'] = []
                for entry in run.result['applied']:
                    data['fixed-tags'].extend(entry['fixed_lintian_tags'])
            if run.suite in ('fresh-releases', 'fresh-snapshots'):
                data['upstream-version'] = run.result.get('upstream_version')
                data['old-upstream-version'] = run.result.get(
                    'old_upstream_version')
            report[run.package] = data
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
        async for (run, maintainer_email, uploader_emails, branch_url,
                   publish_policy, changelog_mode, command
                   ) in state.iter_publish_ready(
                       conn, suite=suite, review_status=review_status):
            if publish_policy in (
                    'propose', 'attempt-push', 'push-derived', 'push'):
                for_publishing.add(run.id)
            ret.append((run.package, run.id))
    ret.sort(key=lambda x: (x[1] not in for_publishing, x[0]))
    return web.json_response(ret, status=200)


def create_app(db, publisher_url, runner_url, archiver_url, policy_config):
    app = web.Application()
    app.http_client_session = ClientSession()
    app.db = db
    app.policy_config = policy_config
    app.publisher_url = publisher_url
    app.runner_url = runner_url
    app.archiver_url = archiver_url
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
        '/{suite:' + SUITE_REGEX + '}/pkg/{package}/schedule', handle_schedule,
        name='api-package-schedule')
    app.router.add_get(
        '/{suite}/pkg/{package}/diff', handle_diff,
        name='api-package-diff')
    app.router.add_get(
        '/merge-proposals', handle_merge_proposal_list,
        name='api-merge-proposals')
    app.router.add_get('/queue', handle_queue, name='api-queue')
    app.router.add_get('/run', handle_run, name='api-run-list')
    app.router.add_post(
        '/publish/scan', handle_publish_scan,
        name='api-publish-scan')
    app.router.add_post(
        '/publish/autopublish', handle_publish_autopublish,
        name='api-publish-autopublish')
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
        '/{suite:' + SUITE_REGEX + '}/report',
        handle_report, name='api-report')
    app.router.add_get(
        '/publish-ready',
        handle_publish_ready, name='api-publish-ready')
    app.router.add_get(
        '/{suite:' + SUITE_REGEX + '}/publish-ready',
        handle_publish_ready, name='api-publish-ready-suite')
    # TODO(jelmer): Previous runs (iter_previous_runs)
    # TODO(jelmer): Last successes (iter_last_successes)
    # TODO(jelmer): Last runs (iter_last_runs)
    # TODO(jelmer): Build failures (iter_build_failures)
    return app
