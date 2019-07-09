#!/usr/bin/python
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


if __name__ == '__main__':
    import argparse
    import functools
    import os
    from janitor.logs import LogFileManager
    from janitor.policy import read_policy
    from aiohttp import web
    from aiohttp.web_middlewares import normalize_path_middleware
    parser = argparse.ArgumentParser()
    parser.add_argument('--host', type=str, help='Host to listen on')
    parser.add_argument('--logdirectory', type=str,
                        help='Logs directory path.', default='site/pkg')
    parser.add_argument('--publisher-url', type=str,
                        default='http://localhost:9912/',
                        help='URL for publisher.')
    parser.add_argument("--policy",
                        help="Policy file to read.", type=str,
                        default=os.path.join(
                            os.path.dirname(__file__), '..', '..',
                            'policy.conf'))

    args = parser.parse_args()

    logfile_manager = LogFileManager(args.logdirectory)

    async def handle_simple(templatename, request):
        from .generate import render_simple
        return web.Response(
            content_type='text/html', text=await render_simple(templatename),
            headers={'Cache-Control': 'max-age=3600'})

    async def handle_lintian_fixes(request):
        from .generate import render_lintian_fixes
        return web.Response(
            content_type='text/html', text=await render_lintian_fixes(),
            headers={'Cache-Control': 'max-age=3600'})

    async def handle_merge_proposals(suite, request):
        from .merge_proposals import write_merge_proposals
        return web.Response(
            content_type='text/html', text=await write_merge_proposals(suite),
            headers={'Cache-Control': 'max-age=60'})

    async def handle_apt_repo(suite, request):
        from .apt_repo import write_apt_repo
        return web.Response(
            content_type='text/html', text=await write_apt_repo(suite),
            headers={'Cache-Control': 'max-age=60'})

    async def handle_history(request):
        limit = int(request.query.get('limit', '100'))
        from .history import write_history
        return web.Response(
            content_type='text/html', text=await write_history(limit=limit),
            headers={'Cache-Control': 'max-age=60'})

    async def handle_queue(request):
        limit = int(request.query.get('limit', '100'))
        from .queue import write_queue
        return web.Response(
            content_type='text/html', text=await write_queue(limit=limit),
            headers={'Cache-Control': 'max-age=600'})

    async def handle_result_codes(request):
        from .result_codes import (
            get_results_by_code, generate_result_code_index,
            generate_result_code_page)
        code = request.match_info.get('code')
        by_code = await get_results_by_code()
        if not code:
            text = await generate_result_code_index(by_code)
        else:
            text = await generate_result_code_page(code, by_code.get(code, []))
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_pkg_list(request):
        # TODO(jelmer): The javascript plugin thingy should just redirect to
        # the right URL, not rely on query parameters here.
        pkg = request.query.get('package')
        if pkg:
            return web.HTTPFound(pkg)
        from .pkg import generate_pkg_list
        from .. import state
        packages = [(item[0], item[1]) for item in await state.iter_packages()]
        text = await generate_pkg_list(packages)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_maintainer_list(request):
        from .pkg import generate_maintainer_list
        from .. import state
        packages = [(item[0], item[1]) for item in await state.iter_packages()]
        text = await generate_maintainer_list(packages)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_pkg(request):
        from .pkg import generate_pkg_file
        from .. import state
        name, maintainer_email, branch_url = list(
            await state.iter_packages(request.match_info['pkg']))[0]
        merge_proposals = []
        for package, url, status in await state.iter_proposals(package=name):
            merge_proposals.append((url, status))
        runs = [x async for x in state.iter_runs(package=name)]
        text = await generate_pkg_file(
            name, merge_proposals, maintainer_email, branch_url, runs)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_run(request):
        from .pkg import generate_run_file
        from .. import state
        run_id = request.match_info['run_id']
        pkg = request.match_info.get('pkg')
        try:
            run = [x async
                   for x in state.iter_runs(run_id=run_id, package=pkg)][0]
        except IndexError:
            raise web.HTTPNotFound(text='No run with id %r' % run_id)
        text = await generate_run_file(logfile_manager, run)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=3600'})

    async def handle_log(request):
        pkg = request.match_info['pkg']
        run_id = request.match_info['run_id']
        filename = request.match_info['log']
        with logfile_manager.get_log(pkg, run_id, filename) as f:
            text = f.read().decode('utf-8', 'replace')
        return web.Response(
            content_type='text/plain', text=text,
            headers={'Cache-Control': 'max-age=3600'})

    async def handle_ready_proposals(suite, request):
        from .pkg import generate_ready_list
        text = await generate_ready_list(suite)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_lintian_fixes_pkg(request):
        from .lintian_fixes import generate_pkg_file
        pkg = request.match_info['pkg']
        try:
            text = await generate_pkg_file(pkg)
        except KeyError:
            raise web.HTTPNotFound()
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_lintian_fixes_tag_list(request):
        from .lintian_fixes import generate_tag_list
        text = await generate_tag_list()
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_lintian_fixes_tag_page(request):
        from .lintian_fixes import generate_tag_page
        text = await generate_tag_page(request.match_info['tag'])
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_new_upstream_pkg(suite, request):
        from .new_upstream import generate_pkg_file
        try:
            text = await generate_pkg_file(request.match_info['pkg'], suite)
        except KeyError:
            raise web.HTTPNotFound()
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[trailing_slash_redirect])
    for path, templatename in [
            ('/', 'index.html'),
            ('/contact', 'contact.html'),
            ('/credentials', 'credentials.html'),
            ('/apt', 'apt.html'),
            ('/cupboard/', 'cupboard.html')]:
        app.router.add_get(
            path, functools.partial(handle_simple, templatename))
    app.router.add_get('/lintian-fixes/', handle_lintian_fixes)
    for suite in ['lintian-fixes', 'fresh-releases', 'fresh-snapshots']:
        app.router.add_get(
            '/%s/merge-proposals' % suite,
            functools.partial(handle_merge_proposals, suite))
        app.router.add_get(
            '/%s/ready' % suite,
            functools.partial(handle_ready_proposals, suite))
        app.router.add_get('/%s/pkg/' % suite, handle_pkg_list)
    app.router.add_get(
        '/lintian-fixes/pkg/{pkg}/', handle_lintian_fixes_pkg)
    app.router.add_get(
        '/lintian-fixes/by-tag/', handle_lintian_fixes_tag_list)
    app.router.add_get(
        '/lintian-fixes/by-tag/{tag}', handle_lintian_fixes_tag_page)
    for suite in ['fresh-releases', 'fresh-snapshots']:
        app.router.add_get(
            '/%s/' % suite, functools.partial(handle_apt_repo, suite))
        app.router.add_get(
            '/%s/pkg/{pkg}/' % suite,
            functools.partial(handle_new_upstream_pkg, suite))
    app.router.add_get('/cupboard/history', handle_history)
    app.router.add_get('/cupboard/queue', handle_queue)
    app.router.add_get('/cupboard/result-codes/', handle_result_codes)
    app.router.add_get('/cupboard/result-codes/{code}', handle_result_codes)
    app.router.add_get('/cupboard/maintainer', handle_maintainer_list)
    app.router.add_get(
        '/cupboard/ready', functools.partial(handle_ready_proposals, None))
    app.router.add_get('/cupboard/pkg/', handle_pkg_list)
    app.router.add_get('/cupboard/pkg/{pkg}/', handle_pkg)
    app.router.add_get('/cupboard/pkg/{pkg}/{run_id}/', handle_run)
    app.router.add_get(
        '/cupboard/pkg/{pkg}/{run_id}/{log:.*\\.log}', handle_log)
    app.router.add_get(
        '/pkg/', handle_pkg_list)
    app.router.add_static(
        '/_static', os.path.join(os.path.dirname(__file__), '_static'))
    from .api import create_app as create_api_app
    with open(args.policy, 'r') as f:
        policy_config = read_policy(f)

    app.add_subapp('/api', create_api_app(args.publisher_url, policy_config))
    web.run_app(app, host=args.host)
