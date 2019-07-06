#!/usr/bin/python3

import argparse
from aiohttp import web
import os

from janitor.policy import read_policy, apply_policy
from janitor import state
from . import env, get_run_diff

DEFAULT_SCHEDULE_PRIORITY = 1000
SUITE_TO_COMMAND = {
    'lintian-fixes': ['lintian-brush'],
    'fresh-releases': ['new-upstream'],
    'fresh-snapshots': ['new-upstream', '--snapshot'],
    }
SUITE_TO_POLICY_FIELD = {
    'lintian-fixes': 'lintian_brush',
    'fresh-releases': 'new_upstream_releases',
    'fresh-snapshots': 'new_upstream_snapshots',
}


async def handle_policy(request):
    package = request.match_info['package']
    try:
        (name, maintainer_email, vcs_url) = list(
            await state.iter_packages(package=package))[0]
    except IndexError:
        return web.json_response({'reason': 'Package not found'}, status=404)
    suite_policies = {}
    # TODO(jelmer): Package uploaders?
    for suite, field in SUITE_TO_POLICY_FIELD.items():
        (publish_policy, changelog_policy, committer) = apply_policy(
            policy_config, field, name, maintainer_email, [])
        suite_policies[suite] = {
            'publish_policy': publish_policy,
            'changelog_policy': changelog_policy,
            'committer': committer}
    response_obj = {'by_suite': suite_policies}
    return web.json_response(response_obj)


async def handle_publish(request):
    package = request.match_info['package']
    post = await request.post()
    mode = post.get('mode', 'push-derived')
    if mode not in ('push-derived', 'push', 'propose', 'attempt-push'):
        return web.json_response(
            {'error': 'Invalid mode', 'mode': mode}, status=400)
    # TODO(jelmer): And now?
    response_obj = {'status': 'success', 'package': package}
    return web.json_response(response_obj)


async def handle_schedule(request):
    package = request.match_info['package']
    suite = request.match_info['suite']
    try:
        command = SUITE_TO_COMMAND[suite]
    except KeyError:
        return web.json_response(
            {'error': 'Unknown suite', 'suite': suite}, status=404)
    post = await request.post()
    priority = post.get('priority', DEFAULT_SCHEDULE_PRIORITY)
    try:
        (name, maintainer_email, vcs_url) = list(
            await state.iter_packages(package=package))[0]
    except IndexError:
        return web.json_response({'reason': 'Package not found'}, status=404)
    run_env = {
        'PACKAGE': name,
        'MAINTAINER_EMAIL': maintainer_email,
    }

    await state.add_to_queue(vcs_url, run_env, command, priority)
    response_obj = {
        'package': package,
        'command': command,
        'suite': suite,
        'priority': priority,
        }
    return web.json_response(response_obj)


async def handle_package_list(request):
    package = request.match_info.get('package')
    response_obj = []
    for name, maintainer_email, branch_url in await state.iter_packages(
            package=package):
        response_obj.append({
            'name': name,
            'maintainer_email': maintainer_email,
            'branch_url': branch_url})
    return web.json_response(
            response_obj, headers={'Cache-Control': 'max-age=600'})


async def handle_packagename_list(request):
    response_obj = []
    for name, maintainer_email, branch_url in await state.iter_packages():
        response_obj.append(name)
    return web.json_response(
        response_obj, headers={'Cache-Control': 'max-age=600'})


async def handle_merge_proposal_list(request):
    response_obj = []
    for package, url, status in await state.iter_proposals(
            request.match_info.get('package')):
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
    async for (queue_id, branch_url, run_env, command) in state.iter_queue(
            limit=limit):
        response_obj.append({
            'queue_id': queue_id,
            'branch_url': branch_url,
            'env': run_env,
            'command': command})
    return web.json_response(
        response_obj, headers={'Cache-Control': 'max-age=60'})


async def handle_diff(request):
    package = request.match_info.get('package')
    run_id = request.match_info['run_id']
    try:
        run = list(await state.iter_runs(package=package, run_id=run_id))[0]
    except IndexError:
        raise web.HTTPNotFoundError()
    f = get_run_diff(run)
    return web.Response(
            content_type='text/x-diff', text=f.getvalue(),
            headers={'Cache-Control': 'max-age=3600'})


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
            'build_info': run.build_info,
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


app = web.Application()
app.router.add_get('/pkgnames', handle_packagename_list)
app.router.add_get('/pkg', handle_package_list)
app.router.add_get('/pkg/{package}', handle_package_list)
app.router.add_get(
    '/pkg/{package}/merge-proposals',
    handle_merge_proposal_list)
app.router.add_get('/pkg/{package}/policy', handle_policy)
app.router.add_post('/{suite}/pkg/{package}/publish', handle_publish)
app.router.add_post('/{suite}/pkg/{package}/schedule', handle_schedule)
app.router.add_get('/merge-proposals', handle_merge_proposal_list)
app.router.add_get('/queue', handle_queue)
app.router.add_get('/run', handle_run)
app.router.add_get('/run/{run_id}', handle_run)
app.router.add_get('/run/{run_id}/diff', handle_diff)
app.router.add_get('/pkg/{package}/run', handle_run)
app.router.add_get('/pkg/{package}/run/{run_id}', handle_run)
app.router.add_get('/pkg/{package}/run/{run_id}/diff', handle_diff)
app.router.add_get('/package-branch', handle_package_branch)
app.router.add_get('/', handle_index)
app.router.add_get(
    '/{suite}/published-packages', handle_published_packages)
app.router.add_get('/policy', handle_global_policy)
# TODO(jelmer): Previous runs (iter_previous_runs)
# TODO(jelmer): Last successes (iter_last_successes)
# TODO(jelmer): Last runs (iter_last_runs)
# TODO(jelmer): Build failures (iter_build_failures)
# TODO(jelmer): Publish ready (iter_publish_ready)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--host', type=str, help='Host to listen on')
    parser.add_argument("--policy",
                        help="Policy file to read.", type=str,
                        default=os.path.join(
                            os.path.dirname(__file__), '..', 'policy.conf'))
    args = parser.parse_args()

    with open(args.policy, 'r') as f:
        policy_config = read_policy(f)

    web.run_app(app, host=args.host)
