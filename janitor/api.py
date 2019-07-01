#!/usr/bin/python3

import argparse
from aiohttp import web
import json
import os

from jinja2 import Environment, PackageLoader, select_autoescape
from janitor.policy import read_policy, apply_policy
from janitor import state

DEFAULT_SCHEDULE_PRIORITY = 1000
SUITE_TO_COMMAND = {
    'lintian-fixes': ['lintian-brush'],
    'fresh-releases': ['merge-upstream'],
    'fresh-snapshots': ['merge-upstream', '--snapshot'],
    }
SUITE_TO_POLICY_FIELD = {
    'lintian-fixes': 'lintian_brush',
    'fresh-releases': 'new_upstream_releases',
    'fresh-snapshots': 'new_upstream_snapshots',
}


async def handle_policy(request):
    package = request.match_info['package']
    try:
        (name, maintainer_email, vcs_url) = list(await state.iter_packages(package=package))[0]
    except IndexError:
        raise web.HTTPNotFound(
            text=json.dumps({'reason': 'Package not found'}),
            content_type='application/json')
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
    return web.Response(
        text=json.dumps(response_obj), content_type='application/json')


async def handle_publish(request):
    package = request.match_info['package']
    # TODO(jelmer)
    response_obj = {'status': 'success', 'package': package}
    return web.Response(
        text=json.dumps(response_obj), content_type='application/json')


async def handle_schedule(request):
    package = request.match_info['package']
    suite = request.match_info['suite']
    command = SUITE_TO_COMMAND[suite]
    post = await request.post()
    priority = post.get('priority', DEFAULT_SCHEDULE_PRIORITY)
    try:
        (name, maintainer_email, vcs_url) = list(await state.iter_packages(package=package))[0]
    except IndexError:
        raise web.HTTPNotFound(
            text=json.dumps({'reason': 'Package not found'}),
            content_type='application/json')
    env = {
        'PACKAGE': name,
        'MAINTAINER_EMAIL': maintainer_email,
    }

    await state.add_to_queue(vcs_url, env, command, priority)
    response_obj = {
        'package': package,
        'command': command,
        'suite': suite,
        'priority': priority,
        }
    return web.Response(
        text=json.dumps(response_obj), content_type='application/json')


async def handle_package_list(request):
    package = request.match_info.get('package')
    response_obj = []
    for name, maintainer_email, branch_url in await state.iter_packages(
            package=package):
        response_obj.append({
            'name': name,
            'maintainer_email': maintainer_email,
            'branch_url': branch_url})
    return web.Response(
        text=json.dumps(response_obj, sort_keys=True, indent=4),
        content_type='application/json')


async def handle_merge_proposal_list(request):
    response_obj = []
    for package, url, status in await state.iter_proposals(
            request.match_info.get('package')):
        response_obj.append({
            'package': package,
            'url': url,
            'status': status})
    return web.Response(
        text=json.dumps(response_obj, sort_keys=True, indent=4),
        content_type='application/json')


async def handle_queue(request):
    limit = request.query.get('limit')
    if limit is not None:
        limit = int(limit)
    response_obj = []
    async for (queue_id, branch_url, env, command) in state.iter_queue(
            limit=limit):
        response_obj.append({
            'queue_id': queue_id,
            'branch_url': branch_url,
            'env': env,
            'command': command})
    return web.Response(
        text=json.dumps(response_obj, sort_keys=True, indent=4),
        content_type='application/json')


async def handle_run(request):
    package = request.match_info.get('package')
    run_id = request.match_info.get('run_id')
    limit = request.query.get('limit')
    if limit is not None:
        limit = int(limit)
    response_obj = []
    async for (run_id, (start_time, finish_time), command, description,
         package_name, merge_proposal_url, build_version, build_distribution,
         result_code, branch_name) in state.iter_runs(
                 package, run_id=run_id, limit=limit):
        if build_version:
            build_info = {
                'version': str(build_version),
                'distribution': build_distribution}
        else:
            build_info = None
        response_obj.append({
            'run_id': run_id,
            'start_time': start_time.isoformat(),
            'finish_time': finish_time.isoformat(),
            'command': command,
            'description': description,
            'package': package_name,
            'merge_proposal_url': merge_proposal_url,
            'build_info': build_info,
            'result_code': result_code,
            'branch_name': branch_name,
            })
    return web.Response(
        text=json.dumps(response_obj, sort_keys=True, indent=4),
        content_type='application/json')


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
    return web.Response(
        text=json.dumps(response_obj, sort_keys=True, indent=4),
        content_type='application/json')


async def handle_published_packages(request):
    suite = request.match_info['suite']
    response_obj = []
    for package, build_version in await state.iter_published_packages(suite):
        response_obj.append({
            'package': package,
            'build_version': build_version})
    return web.Response(
        text=json.dumps(response_obj, sort_keys=True, indent=4),
        content_type='application/json')


async def handle_index(request):
    template = jinja2_env.get_template('api-index.html')
    return web.Response(
        content_type='text/html', text=await template.render_async())


async def handle_global_policy(request):
    with open('policy.conf', 'r') as f:
        return web.Response(content_type='text/protobuf', text=f.read())


jinja2_env = Environment(
    loader=PackageLoader('janitor', 'templates'),
    autoescape=select_autoescape(['html', 'xml']),
    enable_async=True,
)

app = web.Application()
app.router.add_get('/pkg', handle_package_list)
app.router.add_get('/pkg/{package}', handle_package_list)
app.router.add_get(
    '/pkg/{package}/merge-proposals',
    handle_merge_proposal_list)
app.router.add_get('/pkg/{package}/policy', handle_policy)
app.router.add_post('/pkg/{package}/publish', handle_publish)
app.router.add_post('/pkg/{package}/schedule/{suite}', handle_schedule)
app.router.add_get('/merge-proposals', handle_merge_proposal_list)
app.router.add_get('/queue', handle_queue)
app.router.add_get('/run', handle_run)
app.router.add_get('/run/{run_id}', handle_run)
app.router.add_get('/pkg/{package}/run', handle_run)
app.router.add_get('/pkg/{package}/run/{run_id}', handle_run)
app.router.add_get('/package-branch', handle_package_branch)
app.router.add_get('/', handle_index)
app.router.add_get(
    '/apt/{suite}/published-packages', handle_published_packages)
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
                        default=os.path.join(os.path.dirname(__file__), '..', 'policy.conf'))
    args = parser.parse_args()

    with open(args.policy, 'r') as f:
        policy_config = read_policy(f)

    web.run_app(app, host=args.host)
