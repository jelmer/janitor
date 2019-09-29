#!/usr/bin/python3

from aiohttp import web, ClientSession, ContentTypeError, ClientConnectorError
import functools
import urllib.parse

from janitor.policy import apply_policy
from janitor.vcs import get_run_diff
from janitor import state, SUITES
from . import env

DEFAULT_SCHEDULE_OFFSET = -1
SUITE_TO_COMMAND = {
    'lintian-fixes': ['lintian-brush'],
    'fresh-releases': ['new-upstream'],
    'fresh-snapshots': ['new-upstream', '--snapshot'],
    'unchanged': ['just-build'],
    }


async def handle_policy(policy_config, request):
    package = request.match_info['package']
    try:
        package = await state.get_package(package)
    except IndexError:
        return web.json_response({'reason': 'Package not found'}, status=404)
    suite_policies = {}
    for suite in SUITES:
        (publish_policy, changelog_policy, committer) = apply_policy(
            policy_config, suite.replace('-', '_'), package.name,
            package.maintainer_email, package.uploader_emails)
        suite_policies[suite] = {
            'publish_policy': publish_policy,
            'changelog_policy': changelog_policy,
            'committer': committer}
    response_obj = {'by_suite': suite_policies}
    return web.json_response(response_obj)


async def handle_publish(publisher_url, request):
    package = request.match_info['package']
    suite = request.match_info['suite']
    post = await request.post()
    mode = post.get('mode', 'push-derived')
    if mode not in ('push-derived', 'push', 'propose', 'attempt-push'):
        return web.json_response(
            {'error': 'Invalid mode', 'mode': mode}, status=400)
    url = urllib.parse.urljoin(
        publisher_url, '%s/%s/publish' % (suite, package))
    async with ClientSession() as client:
        try:
            async with client.post(url, data={'mode': mode}) as resp:
                if resp.status == 200:
                    return web.json_response(await resp.json())
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


async def get_package_from_gitlab_webhook(body):
    vcs_url = body['project']['git_http_url']
    package = await state.get_package_by_vcs_url(vcs_url)
    if package is None:
        ref = body['ref']
        if not ref.startswith('refs/heads/'):
            return None
        branch_name = ref[len('refs/heads/'):]
        url_with_branch = '%s -b %s' % (vcs_url, branch_name)
        package = await state.get_package_by_vcs_url(
            url_with_branch)
        if package is None:
            return None
    return package


async def schedule(package, suite, offset=DEFAULT_SCHEDULE_OFFSET,
                   refresh=False):
    from ..schedule import estimate_duration
    command = SUITE_TO_COMMAND[suite]
    estimated_duration = await estimate_duration(package.name, suite)
    await state.add_to_queue(
        package.branch_url, package.name, command, suite, offset,
        estimated_duration=estimated_duration, refresh=refresh,
        'user from web UI')
    return estimated_duration


async def handle_webhook(request):
    if request.headers.get('Content-Type') != 'application/json':
        template = env.get_template('webhook.html')
        return web.Response(
            content_type='text/html', text=await template.render_async(),
            headers={'Cache-Control': 'max-age=600'})
    if request.headers['X-Gitlab-Event'] != 'Push Hook':
        return web.json_response({}, status=200)
    body = await request.json()
    package = await get_package_from_gitlab_webhook(body)
    if package is None:
        return web.Response(
            body=('VCS URL %s unknown' % body['project']['git_http_url']),
            status=404)
    # TODO(jelmer: If nothing found, then maybe fall back to
    # urlutils.basename(body['project']['path_with_namespace'])?
    for suite in SUITES:
        await schedule(package, suite)
    return web.json_response({})


async def handle_schedule(request):
    package = request.match_info['package']
    suite = request.match_info['suite']
    if suite not in SUITES:
        return web.json_response(
            {'error': 'Unknown suite', 'suite': suite}, status=404)
    post = await request.post()
    offset = post.get('offset', DEFAULT_SCHEDULE_OFFSET)
    try:
        refresh = bool(int(post.get('refresh', '0')))
    except ValueError:
        return web.json_response(
            {'error': 'invalid boolean for refresh'}, status=400)
    try:
        package = await state.get_package(package)
    except IndexError:
        return web.json_response({'reason': 'Package not found'}, status=404)
    estimated_duration = await schedule(package, suite, offset, refresh)
    response_obj = {
        'package': package.name,
        'suite': suite,
        'offset': offset,
        'estimated_duration_seconds': estimated_duration.total_seconds(),
        }
    return web.json_response(response_obj)


async def handle_package_list(request):
    name = request.match_info.get('package')
    response_obj = []
    for package in await state.iter_packages(package=name):
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
    for package in await state.iter_packages():
        if package.removed:
            continue
        response_obj.append(package.name)
    return web.json_response(
        response_obj, headers={'Cache-Control': 'max-age=600'})


async def handle_merge_proposal_list(request):
    response_obj = []
    for package, url, status in await state.iter_proposals(
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
    async for entry in state.iter_queue(
            limit=limit):
        response_obj.append({
            'queue_id': entry.id,
            'branch_url': entry.branch_url,
            'package': entry.package,
            'env': entry.env,
            'command': entry.command})
    return web.json_response(
        response_obj, headers={'Cache-Control': 'max-age=60'})


async def handle_diff(vcs_manager, publisher_url, request):
    run_id = request.match_info['run_id']
    url = urllib.parse.urljoin(publisher_url, 'diff/%s' % run_id)
    async with ClientSession() as client:
        try:
            async with client.get(url) as resp:
                if resp.status == 200:
                    return web.Response(
                        body=await resp.read(),
                        content_type='text/x-diff',
                        headers={'Cache-Control': 'max-age=3600'})
                else:
                    return web.Response(await resp.read(), status=400)
        except ContentTypeError as e:
            return web.Response(
                'publisher returned error %d' % e.code,
                status=400)
        except ClientConnectorError:
            return web.json_response(
                'unable to contact publisher',
                status=400)


async def handle_run(request):
    package = request.match_info.get('package')
    run_id = request.match_info.get('run_id')
    limit = request.query.get('limit')
    if limit is not None:
        limit = int(limit)
    response_obj = []
    async for run in state.iter_runs(
                package, run_id=run_id, limit=limit):
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
            'package': run.package_name,
            'build_info': build_info,
            'result_code': run.result_code,
            'branch_name': run.branch_name,
            })
    return web.json_response(
        response_obj, headers={'Cache-Control': 'max-age=600'})


async def handle_package_branch(request):
    response_obj = []
    for (name, branch_url, revision, last_scanned, description) in (
            await state.iter_package_branches()):
        response_obj.append({
            'name': name,
            'branch_url': branch_url,
            'revision': revision,
            'last_scanned': last_scanned.isoformat() if last_scanned else None,
            'description': description,
            })
    return web.json_response(
        response_obj, headers={'Cache-Control': 'max-age=60'})


async def handle_published_packages(request):
    suite = request.match_info['suite']
    response_obj = []
    for package, build_version in await state.iter_published_packages(suite):
        response_obj.append({
            'package': package,
            'build_version': build_version})
    return web.json_response(response_obj)


async def handle_index(request):
    template = env.get_template('api-index.html')
    return web.Response(
        content_type='text/html', text=await template.render_async(),
        headers={'Cache-Control': 'max-age=600'})


async def handle_global_policy(request):
    with open('policy.conf', 'r') as f:
        return web.Response(
            content_type='text/protobuf', text=f.read(),
            headers={'Cache-Control': 'max-age=60'})


async def forward_to_runner(runner_url, path):
    url = urllib.parse.urljoin(runner_url, path)
    async with ClientSession() as client:
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


async def handle_runner_status(runner_url, request):
    return await forward_to_runner(runner_url, 'status')


async def handle_runner_log_index(runner_url, request):
    run_id = request.match_info['run_id']
    return await forward_to_runner(runner_url, 'log/%s' % run_id)


async def handle_runner_log(runner_url, request):
    run_id = request.match_info['run_id']
    filename = request.match_info['filename']
    url = urllib.parse.urljoin(runner_url, 'log/%s/%s' % (run_id, filename))
    async with ClientSession() as client:
        try:
            async with client.get(url) as resp:
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


def create_app(publisher_url, runner_url, policy_config, vcs_manager):
    app = web.Application()
    app.router.add_get('/pkgnames', handle_packagename_list)
    app.router.add_get('/pkg', handle_package_list)
    app.router.add_get('/pkg/{package}', handle_package_list)
    app.router.add_get(
        '/pkg/{package}/merge-proposals',
        handle_merge_proposal_list)
    app.router.add_get(
        '/pkg/{package}/policy',
        functools.partial(handle_policy, policy_config))
    app.router.add_post(
        '/{suite}/pkg/{package}/publish',
        functools.partial(handle_publish, publisher_url))
    app.router.add_post('/{suite}/pkg/{package}/schedule', handle_schedule)
    app.router.add_get('/merge-proposals', handle_merge_proposal_list)
    app.router.add_get('/queue', handle_queue)
    app.router.add_get('/run', handle_run)
    app.router.add_get('/run/{run_id}', handle_run)
    app.router.add_get(
        '/run/{run_id}/diff',
        functools.partial(handle_diff, vcs_manager, publisher_url))
    app.router.add_get('/pkg/{package}/run', handle_run)
    app.router.add_get('/pkg/{package}/run/{run_id}', handle_run)
    app.router.add_get(
        '/pkg/{package}/run/{run_id}/diff',
        functools.partial(handle_diff, vcs_manager, publisher_url))
    app.router.add_get('/package-branch', handle_package_branch)
    app.router.add_get('/', handle_index)
    app.router.add_get(
        '/{suite}/published-packages', handle_published_packages)
    app.router.add_get('/policy', handle_global_policy)
    app.router.add_post('/webhook', handle_webhook)
    app.router.add_get('/webhook', handle_webhook)
    app.router.add_get(
        '/runner/status', functools.partial(handle_runner_status, runner_url))
    app.router.add_get(
        '/runner/log/{run_id}',
        functools.partial(handle_runner_log_index, runner_url))
    app.router.add_get(
        '/runner/log/{run_id}/{filename}',
        functools.partial(handle_runner_log, runner_url))
    # TODO(jelmer): Previous runs (iter_previous_runs)
    # TODO(jelmer): Last successes (iter_last_successes)
    # TODO(jelmer): Last runs (iter_last_runs)
    # TODO(jelmer): Build failures (iter_build_failures)
    # TODO(jelmer): Publish ready (iter_publish_ready)
    return app
