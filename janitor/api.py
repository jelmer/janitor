#!/usr/bin/python3

import argparse
from aiohttp import web
import json

from jinja2 import Environment, PackageLoader, select_autoescape
from janitor import state


async def handle_policy(request):
    package = request.match_info['package']
    # TODO(jelmer)
    response_obj = {'status': 'success', 'package': package}
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
    command = request.match_info['command']
    # TODO(jelmer)
    response_obj = {
        'status': 'success',
        'package': package,
        'command': command,
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
    # TODO(jelmer): support limit argument
    limit = None
    response_obj = []
    for (queue_id, branch_url, env, command) in await state.iter_queue(
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
    # TODO(jelmer): support limit argument
    limit = None
    package = request.match_info.get('package')
    run_id = request.match_info.get('run_id')
    response_obj = []
    for (run_id, (start_time, finish_time), command, description,
         package_name, merge_proposal_url, build_version, build_distribution,
         result_code, branch_name) in await state.iter_runs(
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
app.router.add_get('/pkg/{package}/publish', handle_publish)
app.router.add_get('/pkg/{package}/schedule/{suite}', handle_schedule)
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

parser = argparse.ArgumentParser()
parser.add_argument('--host', type=str, help='Host to listen on')
args = parser.parse_args()

web.run_app(app, host=args.host)
