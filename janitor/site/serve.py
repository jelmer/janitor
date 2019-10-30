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


async def read_apt_file_from_s3(
        request, session, s3_location, suite, filename, max_age):
    headers = {'Cache-Control': 'max-age=%d' % max_age}
    url = '%s/%s/%s' % (s3_location, suite, filename)
    async with session.get(url) as client_response:
        status = client_response.status

        if status == 404:
            raise web.HTTPNotFound()

        if status != 200:
            raise web.HTTPBadRequest()

        response = web.StreamResponse(
            status=200,
            reason='OK',
            headers=headers
        )
        await response.prepare(request)
        S3_READ_CHUNK_SIZE = 65536
        while True:
            chunk = await client_response.content.read(S3_READ_CHUNK_SIZE)
            if not chunk:
                break
            await response.write(chunk)
    await response.write_eof()
    return response


async def read_apt_file_from_fs(suite, filename, max_age):
    headers = {'Cache-Control': 'max-age=%d' % max_age}
    path = os.path.join(
            os.path.dirname(__file__), '..', '..',
            "public_html", suite, filename)
    if not os.path.exists(path):
        raise web.HTTPNotFound()
    return web.FileResponse(path, headers=headers)


if __name__ == '__main__':
    import argparse
    import functools
    import os
    import re
    from janitor import SUITES
    from janitor import state
    from janitor.config import read_config
    from janitor.logs import get_log_manager
    from janitor.policy import read_policy
    from janitor.prometheus import setup_metrics
    from aiohttp import web, ClientSession
    from aiohttp.web_middlewares import normalize_path_middleware
    from ..pubsub import pubsub_reader, pubsub_handler, Topic
    import urllib.parse
    parser = argparse.ArgumentParser()
    parser.add_argument('--host', type=str, help='Host to listen on')
    parser.add_argument(
        '--port',
        type=int, help='Port to listen on', default=8080)
    parser.add_argument(
        '--publisher-url', type=str,
        default='http://localhost:9912/',
        help='URL for publisher.')
    parser.add_argument(
        '--runner-url', type=str,
        default='http://localhost:9911/',
        help='URL for runner.')
    parser.add_argument(
        "--policy",
        help="Policy file to read.", type=str,
        default=os.path.join(
            os.path.dirname(__file__), '..', '..', 'policy.conf'))
    parser.add_argument(
        '--config', type=str, default='janitor.conf',
        help='Path to configuration.')

    args = parser.parse_args()

    with open(args.config, 'r') as f:
        config = read_config(f)

    state.DEFAULT_URL = config.database_location
    logfile_manager = get_log_manager(config.logs_location)

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
            content_type='text/html', text=await write_merge_proposals(
                request.app.database, suite),
            headers={'Cache-Control': 'max-age=60'})

    async def handle_apt_repo(suite, request):
        from .apt_repo import write_apt_repo
        async with request.app.database.acquire() as conn:
            return web.Response(
                content_type='text/html',
                text=await write_apt_repo(conn, suite),
                headers={'Cache-Control': 'max-age=60'})

    async def handle_history(request):
        limit = int(request.query.get('limit', '100'))
        from .history import write_history
        async with request.app.database.acquire() as conn:
            return web.Response(
                content_type='text/html',
                text=await write_history(conn, limit=limit),
                headers={'Cache-Control': 'max-age=60'})

    async def handle_publish_history(request):
        limit = int(request.query.get('limit', '100'))
        from .publish import write_history
        async with request.app.database.acquire() as conn:
            return web.Response(
                content_type='text/html',
                text=await write_history(conn, limit=limit),
                headers={'Cache-Control': 'max-age=10'})

    async def handle_queue(request):
        limit = int(request.query.get('limit', '100'))
        from .queue import write_queue
        async with request.app.database.acquire() as conn:
            return web.Response(
                content_type='text/html', text=await write_queue(
                    request.app.http_client_session, conn,
                    queue_status=app.runner_status, limit=limit),
                headers={'Cache-Control': 'max-age=10'})

    async def handle_result_codes(request):
        from .result_codes import (
            generate_result_code_index,
            generate_result_code_page)
        from .. import state
        code = request.match_info.get('code')
        async with request.app.database.acquire() as conn:
            if not code:
                stats = await state.stats_by_result_codes(conn)
                never_processed = sum(dict(
                    await state.get_never_processed(conn)).values())
                text = await generate_result_code_index(stats, never_processed)
            else:
                runs = [run async for run in state.iter_last_runs(conn, code)]
                text = await generate_result_code_page(code, runs)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_login(request):
        return web.Response(
            content_type='text/plain',
            text=repr(request.headers))

    async def handle_pkg_list(request):
        # TODO(jelmer): The javascript plugin thingy should just redirect to
        # the right URL, not rely on query parameters here.
        pkg = request.query.get('package')
        if pkg:
            return web.HTTPFound(pkg)
        from .pkg import generate_pkg_list
        from .. import state
        async with request.app.database.acquire() as conn:
            packages = [
                (item.name, item.maintainer_email)
                for item in await state.iter_packages(conn)
                if not item.removed]
        text = await generate_pkg_list(packages)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_maintainer_list(request):
        from .pkg import generate_maintainer_list
        from .. import state
        async with request.app.database.acquire() as conn:
            packages = [
                (item.name, item.maintainer_email)
                for item in await state.iter_packages(conn)
                if not item.removed]
        text = await generate_maintainer_list(packages)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_pkg(request):
        from .pkg import generate_pkg_file
        from .. import state
        package_name = request.match_info['pkg']
        async with request.app.database.acquire() as conn:
            package = await state.get_package(conn, package_name)
            if package is None:
                raise web.HTTPNotFound(
                    text='No package with name %s' % package_name)
            merge_proposals = []
            async for (run, url, status) in state.iter_proposals_with_run(
                    conn, package=package.name):
                merge_proposals.append((url, status, run))
            runs = state.iter_runs(conn, package=package.name)
            text = await generate_pkg_file(
                request.app.database, package, merge_proposals, runs)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_failed_lintian_brush_fixers(request):
        from .lintian_fixes import (
            generate_failing_fixers_list, generate_failing_fixer)
        fixer = request.match_info.get('fixer')
        if fixer:
            text = await generate_failing_fixer(
                request.app.database, fixer)
        else:
            text = await generate_failing_fixers_list(
                request.app.database)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_lintian_brush_regressions(request):
        from .lintian_fixes import generate_regressions_list
        text = await generate_regressions_list(request.app.database)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_run(request):
        from .pkg import generate_run_file
        from .. import state
        run_id = request.match_info['run_id']
        pkg = request.match_info.get('pkg')
        async with request.app.database.acquire() as conn:
            run = await state.get_run(conn, run_id, pkg)
            if run is None:
                raise web.HTTPNotFound(text='No run with id %r' % run_id)
        text = await generate_run_file(
            request.app.database,
            request.app.http_client_session,
            logfile_manager, run, request.app.publisher_url)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=3600'})

    async def handle_log(request):
        pkg = request.match_info['pkg']
        filename = request.match_info['log']
        run_id = request.match_info['run_id']
        if not re.match('^[a-z0-9+-\\.]+$', pkg) or len(pkg) < 2:
            raise web.HTTPNotFound(
                text='No log file %s for run %s' % (filename, run_id))
        if not re.match('^[a-z0-9-]+$', run_id) or len(run_id) < 5:
            raise web.HTTPNotFound(
                text='No log file %s for run %s' % (filename, run_id))
        if not re.match('^[a-z0-9\\.]+$', filename) or len(filename) < 3:
            raise web.HTTPNotFound(
                text='No log file %s for run %s' % (filename, run_id))
        try:
            logfile = await logfile_manager.get_log(pkg, run_id, filename)
        except FileNotFoundError:
            raise web.HTTPNotFound(
                text='No log file %s for run %s' % (filename, run_id))
        else:
            with logfile as f:
                text = f.read().decode('utf-8', 'replace')
        return web.Response(
            content_type='text/plain', text=text,
            headers={'Cache-Control': 'max-age=3600'})

    async def handle_ready_proposals(suite, request):
        from .pkg import generate_ready_list
        review_status = request.query.get('review_status')
        text = await generate_ready_list(
            request.app.database, suite, review_status)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_lintian_fixes_pkg(request):
        from .lintian_fixes import generate_pkg_file
        pkg = request.match_info['pkg']
        run_id = request.match_info.get('run_id')
        try:
            text = await generate_pkg_file(
                request.app.database,
                request.app.policy,
                request.app.http_client_session,
                request.app.publisher_url, pkg, run_id)
        except KeyError:
            raise web.HTTPNotFound()
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_lintian_fixes_tag_list(request):
        from .lintian_fixes import generate_tag_list
        async with request.app.database.acquire() as conn:
            text = await generate_tag_list(conn)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_lintian_fixes_tag_page(request):
        from .lintian_fixes import generate_tag_page
        text = await generate_tag_page(
            request.app.database, request.match_info['tag'])
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_lintian_fixes_developer_table_page(request):
        from .lintian_fixes import generate_developer_table_page
        try:
            developer = request.match_info['developer']
        except KeyError:
            developer = request.query.get('developer')
        if '@' not in developer:
            developer = '%s@debian.org' % developer
        text = await generate_developer_table_page(
            request.app.database, developer)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=30'})

    async def handle_new_upstream_pkg(suite, request):
        from .new_upstream import generate_pkg_file
        pkg = request.match_info['pkg']
        run_id = request.match_info.get('run_id')
        try:
            text = await generate_pkg_file(
                request.app.database, pkg, suite, run_id)
        except KeyError:
            raise web.HTTPNotFound()
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_apt_file(request):
        suite = request.match_info['suite']
        file = request.match_info['file']

        if (file.endswith('.deb') or
                file.endswith('.buildinfo') or
                file.endswith('.changes')):
            # One week
            max_age = 60 * 60 * 24 * 7
        else:
            # 1 Minute
            max_age = 60

        if config.apt_location.startswith('http'):
            return await read_apt_file_from_s3(
                request, app.http_client_session, config.apt_location, suite,
                file, max_age)
        else:
            return await read_apt_file_from_fs(suite, file, max_age)

    async def handle_lintian_fixes_candidates(request):
        from .lintian_fixes import generate_candidates
        text = await generate_candidates(
            request.app.database)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_new_upstream_candidates(suite, request):
        from .new_upstream import generate_candidates
        text = await generate_candidates(
            request.app.database, suite)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def start_pubsub_forwarder(app):

        async def listen_to_publisher_publish(app):
            url = urllib.parse.urljoin(app.publisher_url, 'ws/publish')
            async for msg in pubsub_reader(app.http_client_session, url):
                app.topic_notifications.publish(['publish', msg])

        async def listen_to_publisher_mp(app):
            url = urllib.parse.urljoin(app.publisher_url, 'ws/merge-proposal')
            async for msg in pubsub_reader(app.http_client_session, url):
                app.topic_notifications.publish(['merge-proposal', msg])

        app.runner_status = None
        async def listen_to_runner(app):
            url = urllib.parse.urljoin(app.runner_url, 'ws/queue')
            async for msg in pubsub_reader(app.http_client_session, url):
                app.runner_status = msg
                app.topic_notifications.publish(['queue', msg])

        for cb in [listen_to_publisher_publish, listen_to_publisher_mp,
                   listen_to_runner]:
            listener = app.loop.create_task(cb(app))
            async def stop_listener(app):
                listener.cancel()
                await listener
            app.on_cleanup.append(stop_listener)

    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[trailing_slash_redirect])
    for path, templatename in [
            ('/', 'index'),
            ('/contact', 'contact'),
            ('/about', 'about'),
            ('/credentials', 'credentials'),
            ('/apt', 'apt'),
            ('/cupboard/', 'cupboard')]:
        app.router.add_get(
            path, functools.partial(handle_simple, templatename + '.html'),
            name=templatename)
    app.router.add_get(
        '/lintian-fixes/', handle_lintian_fixes,
        name='lintian-fixes-start')
    for suite in SUITES:
        app.router.add_get(
            '/%s/merge-proposals' % suite,
            functools.partial(handle_merge_proposals, suite),
            name='%s-merge-proposals' % suite)
        app.router.add_get(
            '/%s/ready' % suite,
            functools.partial(handle_ready_proposals, suite),
            name='%s-ready' % suite)
        app.router.add_get(
            '/%s/maintainer' % suite, handle_maintainer_list,
            name='%s-maintainer-list' % suite)
        app.router.add_get(
            '/%s/pkg/' % suite, handle_pkg_list,
            name='%s-package-list' % suite)
    app.router.add_get(
        '/{suite:' + '|'.join(SUITES) + '}'
        '/{file:Contents-.*|InRelease|Packages.*|Release.*|'
        '.*.(changes|deb|buildinfo)}',
        handle_apt_file, name='apt-file')
    app.router.add_get(
        '/unchanged', functools.partial(handle_apt_repo, 'unchanged'),
        name='unchanged-start')
    app.router.add_get(
        '/lintian-fixes/pkg/{pkg}/', handle_lintian_fixes_pkg,
        name='lintian-fixes-package')
    app.router.add_get(
        '/lintian-fixes/pkg/{pkg}/{run_id}', handle_lintian_fixes_pkg,
        name='lintian-fixes-package-run')
    app.router.add_get(
        '/lintian-fixes/by-tag/', handle_lintian_fixes_tag_list,
        name='lintian-fixes-tag-list')
    app.router.add_get(
        '/lintian-fixes/by-tag/{tag}', handle_lintian_fixes_tag_page,
        name='lintian-fixes-tag')
    app.router.add_get(
        '/lintian-fixes/by-developer',
        handle_lintian_fixes_developer_table_page,
        name='lintian-fixes-developer-list')
    app.router.add_get(
        '/lintian-fixes/by-developer/{developer}',
        handle_lintian_fixes_developer_table_page,
        name='lintian-fixes-developer')
    app.router.add_get(
        '/lintian-fixes/candidates', handle_lintian_fixes_candidates,
        name='lintian-fixes-candidates')
    for suite in ['fresh-releases', 'fresh-snapshots']:
        app.router.add_get(
            '/%s/' % suite, functools.partial(handle_apt_repo, suite),
            name='%s-start' % suite)
        app.router.add_get(
            '/%s/pkg/{pkg}/' % suite,
            functools.partial(handle_new_upstream_pkg, suite),
            name='%s-package' % suite)
        app.router.add_get(
            '/%s/pkg/{pkg}/{run_id}' % suite,
            functools.partial(handle_new_upstream_pkg, suite),
            name='%s-run' % suite)
        app.router.add_get(
            '/%s/candidates' % suite,
            functools.partial(handle_new_upstream_candidates, suite),
            name='%s-candidates' % suite)

    app.router.add_get('/cupboard/history', handle_history, name='history')
    app.router.add_get('/cupboard/queue', handle_queue, name='queue')
    app.router.add_get('/cupboard/result-codes/', handle_result_codes,
                       name='result-code-list')
    app.router.add_get('/cupboard/result-codes/{code}', handle_result_codes, 
                       name='result-code')
    app.router.add_get(
        '/cupboard/maintainer', handle_maintainer_list, name='maintainer-list')
    app.router.add_get(
        '/cupboard/publish', handle_publish_history, name='publish-history')
    app.router.add_get(
        '/cupboard/ready', functools.partial(handle_ready_proposals, None),
        name='cupboard-ready')
    app.router.add_get(
        '/cupboard/pkg/', handle_pkg_list, name='package-list')
    app.router.add_get(
        '/cupboard/pkg/{pkg}/', handle_pkg, name='cupboard-package')
    app.router.add_get(
        '/cupboard/pkg/{pkg}/{run_id}/', handle_run,
        name='cupboard-run')
    app.router.add_get(
        '/cupboard/failed-lintian-brush-fixers/',
        handle_failed_lintian_brush_fixers,
        name='failed-lintian-brush-fixer-list')
    app.router.add_get(
        '/cupboard/failed-lintian-brush-fixers/{fixer}',
        handle_failed_lintian_brush_fixers,
        name='failed-lintian-brush-fixer')
    app.router.add_get(
        '/cupboard/lintian-brush-regressions/',
        handle_lintian_brush_regressions,
        name='lintian-brush-regressions')
    app.router.add_get(
        '/cupboard/pkg/{pkg}/{run_id}/{log:.*\\.log(\\.[0-9]+)?}', handle_log,
        name='logfile')
    app.router.add_get(
        '/login', handle_login,
        name='login')
    app.router.add_static(
        '/_static', os.path.join(os.path.dirname(__file__), '_static'))
    from .api import create_app as create_api_app
    with open(args.policy, 'r') as f:
        policy_config = read_policy(f)

    app.http_client_session = ClientSession()
    app.topic_notifications = Topic()
    app.runner_url = args.runner_url
    app.policy = policy_config
    app.publisher_url = args.publisher_url
    app.on_startup.append(start_pubsub_forwarder)
    app.database = state.Database(config.database_location)
    setup_metrics(app)
    app.router.add_get(
        '/ws/notifications',
        functools.partial(pubsub_handler, app.topic_notifications),
        name='ws-notifications')
    app.add_subapp(
        '/api', create_api_app(
            app.database, args.publisher_url, args.runner_url, policy_config))
    web.run_app(app, host=args.host, port=args.port)
