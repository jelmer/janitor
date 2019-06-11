#!/usr/bin/python3

from aiohttp import web
import json

from janitor import state


async def handle_policy(request):
    package = request.match_info['package']
    # TODO(jelmer)
    response_obj = {'status': 'success', 'package': package}
    return web.Response(text=json.dumps(response_obj))


async def handle_publish(request):
    package = request.match_info['package']
    # TODO(jelmer)
    response_obj = {'status': 'success', 'package': package}
    return web.Response(text=json.dumps(response_obj))


async def handle_reschedule(request):
    package = request.match_info['package']
    command = request.match_info['command']
    # TODO(jelmer)
    response_obj = {
        'status': 'success',
        'package': package,
        'command': command,
        }
    return web.Response(text=json.dumps(response_obj))


async def handle_package_list(request):
    response_obj = []
    for name, maintainer_email, branch_url in state.iter_packages():
        response_obj.append({
            'name': name,
            'maintainer_email': maintainer_email,
            'branch_url': branch_url})
    return web.Response(
        text=json.dumps(response_obj, sort_keys=True, indent=4))


async def handle_merge_proposal_list(request):
    response_obj = []
    for package, url, status in state.iter_proposals(
            request.match_info.get('package')):
        response_obj.append({
            'package': package,
            'url': url,
            'status': status})
    return web.Response(
        text=json.dumps(response_obj, sort_keys=True, indent=4))


async def handle_queue(request):
    # TODO(jelmer): support limit argument
    limit = None
    response_obj = []
    for (queue_id, branch_url, env, command) in state.iter_queue(limit=limit):
        response_obj.append({
            'queue_id': queue_id,
            'branch_url': branch_url,
            'env': env,
            'command': command})
    return web.Response(
        text=json.dumps(response_obj, sort_keys=True, indent=4))


async def handle_run(request):
    # TODO(jelmer): support limit argument
    limit = None
    package = request.match_info.get('package')
    response_obj = []
    for (run_id, (start_time, finish_time), command, description,
         package_name, merge_proposal_url, build_version, build_distribution,
         result_code, branch_name) in state.iter_runs(package, limit=limit):
        response_obj.append({
            'run_id': run_id,
            'start_time': start_time.isoformat(),
            'finish_time': finish_time.isoformat(),
            'command': command,
            'description': description,
            'package': package_name,
            'merge_proposal_url': merge_proposal_url,
            'build_version': str(build_version) if build_version else None,
            'build_distribution': build_distribution,
            'result_code': result_code,
            'branch_name': branch_name,
            })
    return web.Response(
        text=json.dumps(response_obj, sort_keys=True, indent=4))


async def handle_package_branch(request):
    response_obj = []
    for (name, branch_url, revision) in state.iter_package_branches():
        response_obj.append({
            'name': name,
            'branch_url': branch_url,
            'revision': revision,
            })
    return web.Response(
        text=json.dumps(response_obj, sort_keys=True, indent=4))


app = web.Application()
app.router.add_get('/pkg', handle_package_list)
app.router.add_get('/merge-proposals', handle_merge_proposal_list)
app.router.add_get('/merge-proposals/{package}', handle_merge_proposal_list)
app.router.add_get('/pkg/{package}/policy', handle_policy)
app.router.add_get('/pkg/{package}/publish', handle_publish)
app.router.add_get('/pkg/{package}/schedule/{command}', handle_reschedule)
app.router.add_get('/queue', handle_queue)
app.router.add_get('/run', handle_run)
app.router.add_get('/pkg/{package}/run', handle_run)
app.router.add_get('/package-branch', handle_package_branch)
# TODO(jelmer): Published packages (iter_published_packages)
# TODO(jelmer): Previous runs (iter_previous_runs)
# TODO(jelmer): Last successes (iter_last_successes)
# TODO(jelmer): Last runs (iter_last_runs)
# TODO(jelmer): Build failures (iter_build_failures)
# TODO(jelmer): Publish ready (iter_publish_ready)
# TODO(jelmer): Unscanned branches (iter_unscanned_branches)

web.run_app(app)
