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
    from aiohttp import web
    from aiohttp.web_middlewares import normalize_path_middleware
    parser = argparse.ArgumentParser()
    parser.add_argument('--host', type=str, help='Host to listen on')
    parser.add_argument('--logdirectory', type=str, help='Logs directory path.', default='site/pkg')
    args = parser.parse_args()

    async def handle_simple(templatename, request):
        from .generate import render_simple
        return web.Response(
            content_type='text/html', text=await render_simple(templatename))

    async def handle_lintian_fixes(request):
        from .generate import render_lintian_fixes
        return web.Response(
            content_type='text/html', text=await render_lintian_fixes())

    async def handle_merge_proposals(suite, request):
        from .merge_proposals import write_merge_proposals
        return web.Response(
            content_type='text/html', text=await write_merge_proposals(suite))

    async def handle_apt_repo(suite, request):
        from .apt_repo import write_apt_repo
        return web.Response(
            content_type='text/html', text=await write_apt_repo(suite))

    async def handle_history(request):
        limit = int(request.query.get('limit', '100'))
        from .history import write_history
        return web.Response(
            content_type='text/html', text=await write_history(limit=limit))

    async def handle_queue(request):
        limit = int(request.query.get('limit', '100'))
        from .queue import write_queue
        return web.Response(
            content_type='text/html', text=await write_queue(limit=limit))

    async def handle_result_codes(request):
        from .result_codes import get_results_by_code, generate_result_code_index, generate_result_code_page
        code = request.match_info.get('code')
        by_code = await get_results_by_code()
        if not code:
            text = await generate_result_code_index(by_code)
        else:
            text = await generate_result_code_page(code, by_code[code])
        return web.Response(content_type='text/html', text=text)

    async def handle_pkg_list(request):
        from .pkg import generate_pkg_list
        from .. import state
        packages = [item[0] for item in await state.iter_packages()]
        text = await generate_pkg_list(packages)
        return web.Response(content_type='text/html', text=text)

    async def handle_pkg(request):
        from .pkg import generate_pkg_file
        from .. import state
        name, maintainer_email, branch_url = list(await state.iter_packages(request.match_info['pkg']))[0]
        merge_proposals = []
        for package, url, status in await state.iter_proposals(package=name):
            merge_proposals.append((url, status))
        runs = [x async for x in state.iter_runs(package=name)]
        text = await generate_pkg_file(name, merge_proposals, maintainer_email, branch_url, runs)
        return web.Response(content_type='text/html', text=text)

    async def handle_run(request):
        from .pkg import generate_run_file
        from .. import state
        run_id = request.match_info['run_id']
        pkg = request.match_info.get('pkg')
        run = [x async for x in state.iter_runs(run_id=run_id, package=pkg)][0]
        text = await generate_run_file(args.logdirectory, *run)
        return web.Response(content_type='text/html', text=text)

    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[trailing_slash_redirect])
    for path, templatename in [
        ('/', 'index.html'),
        ('/contact', 'contact.html'),
        ('/credentials', 'credentials.html'),
        ('/apt', 'apt.html'),
        ('/cupboard', 'cupboard.html')]:
        app.router.add_get(path, functools.partial(handle_simple, templatename))
    app.router.add_get('/lintian-fixes/', handle_lintian_fixes)
    for suite in ['lintian-fixes', 'fresh-releases', 'fresh-snapshots']:
        app.router.add_get('/%s/merge-proposals' % suite, functools.partial(handle_merge_proposals, suite))
    for suite in ['fresh-releases/', 'fresh-snapshots/']:
        app.router.add_get('/%s' % suite, functools.partial(handle_apt_repo, suite))
    app.router.add_get('/cupboard/history', handle_history)
    app.router.add_get('/cupboard/queue', handle_queue)
    app.router.add_get('/cupboard/result-codes/', handle_result_codes)
    app.router.add_get('/cupboard/result-codes/{code}', handle_result_codes)
    app.router.add_get('/pkg/', handle_pkg_list)
    app.router.add_get('/pkg/{pkg}/', handle_pkg)
    app.router.add_get('/pkg/{pkg}/{run_id}/', handle_run)
    from janitor.api import app as api_app
    app.add_subapp('/api', api_app)
    web.run_app(app, host=args.host)
