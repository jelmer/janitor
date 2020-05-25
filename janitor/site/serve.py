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


import asyncio
from aiohttp.web_urldispatcher import (
    PrefixResource,
    ResourceRoute,
    URL,
    UrlMappingMatchInfo,
    )
from . import is_worker


class ForwardedResource(PrefixResource):

    def __init__(self, prefix, location,
                 name=None,
                 expect_handler=None,
                 chunk_size=256 * 1024):
        while prefix.startswith('/'):
            prefix = prefix[1:]
        super().__init__('/' + prefix, name=name)
        self._chunk_size = chunk_size
        self._expect_handler = expect_handler
        self._location = location

        self._routes = {
            method: ResourceRoute(
                method, self._handle, self, expect_handler=expect_handler)
            for method in ['GET', 'POST', 'HEAD']}

    def url_for(self, path):
        while path.startswith('/'):
            path = path[1:]
        path = '/' + path
        return URL.build(path=self._prefix + path)

    def get_info(self):
        return {'location': self._location,
                'prefix': self._prefix}

    def set_options_route(self, handler):
        if 'OPTIONS' in self._routes:
            raise RuntimeError('OPTIONS route was set already')
        self._routes['OPTIONS'] = ResourceRoute(
            'OPTIONS', handler, self,
            expect_handler=self._expect_handler)

    async def resolve(self, request):
        path = request.rel_url.raw_path
        method = request.method
        allowed_methods = set(self._routes)
        if not path.startswith(self._prefix):
            return None, set()

        if method not in allowed_methods:
            return None, allowed_methods

        match_dict = {'path': URL.build(path=path[len(self._prefix)+1:],
                                        encoded=True).path}
        return (UrlMappingMatchInfo(match_dict, self._routes[method]),
                allowed_methods)

    def __len__(self):
        return len(self._routes)

    def __iter__(self):
        return iter(self._routes.values())

    async def _handle(self, request):
        rel_url = request.match_info['path']
        url = '%s/%s' % (self._location, rel_url)
        headers = {}
        for hdr in ['Accept', 'Pragma', 'Accept-Encoding', 'Content-Type']:
            value = request.headers.get(hdr)
            if value:
                headers[hdr] = value
        params = {}
        service = request.query.get('service')
        if service:
            params['service'] = service
        if await is_worker(request.app.database, request):
            params['allow_writes'] = '1'
        async with request.app.http_client_session.request(
                request.method, url, params=params, headers=headers,
                data=request.content) as client_response:
            status = client_response.status

            if status == 404:
                raise web.HTTPNotFound()

            if status == 401:
                raise web.HTTPUnauthorized(headers={
                    'WWW-Authenticate': 'Basic Realm="Debian Janitor"'})

            if status != 200:
                raise web.HTTPBadGateway(
                    text='Upstream server returned %d' % status)

            response = web.StreamResponse(
                status=200,
                reason='OK',
            )

            response.content_type = client_response.content_type

            await response.prepare(request)
            while True:
                chunk = await client_response.content.read(self._chunk_size)
                if not chunk:
                    break
                await response.write(chunk)
        await response.write_eof()
        return response

    def __repr__(self):
        name = "'" + self.name + "'" if self.name is not None else ""
        return "<ForwardedResource {name} {prefix} -> {location!r}>".format(
            name=name, prefix=self._prefix, location=self._location)


@asyncio.coroutine
def debsso_middleware(app, handler):
    @asyncio.coroutine
    def wrapper(request):
        dn = request.headers.get('SSL_CLIENT_S_DN')
        request.debsso_email = None
        if dn and request.headers.get('SSL_CLIENT_VERIFY') == 'SUCCESS':
            m = re.match('.*CN=([^/,]+)', dn)
            if m:
                request.debsso_email = m.group(1)
        response = yield from handler(request)
        if request.debsso_email:
            response.headers['X-DebSSO-User'] = request.debsso_email
        return response
    return wrapper


def setup_debsso(app):
    app.middlewares.insert(0, debsso_middleware)


def iter_accept(request):
    return [
        h.strip() for h in request.headers.get('Accept', '*/*').split(',')]


if __name__ == '__main__':
    import argparse
    import functools
    import os
    import re
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
        '--archiver-url', type=str,
        default='http://localhost:9914/',
        help='URL for runner.')
    parser.add_argument(
        "--policy",
        help="Policy file to read.", type=str,
        default=os.path.join(
            os.path.dirname(__file__), '..', '..', 'policy.conf'))
    parser.add_argument(
        '--config', type=str, default='janitor.conf',
        help='Path to configuration.')
    parser.add_argument(
        '--debug', action='store_true',
        help='Enable debugging mode. For example, avoid minified JS.')

    args = parser.parse_args()

    if args.debug:
        minified = ''
    else:
        minified = 'min.'

    with open(args.config, 'r') as f:
        config = read_config(f)

    logfile_manager = get_log_manager(config.logs_location)

    async def handle_simple(templatename, request):
        from .generate import render_simple
        return web.Response(
            content_type='text/html', text=await render_simple(templatename),
            headers={'Cache-Control': 'max-age=3600'})

    async def handle_lintian_fixes(request):
        from .lintian_fixes import render_start
        return web.Response(
            content_type='text/html', text=await render_start(),
            headers={'Cache-Control': 'max-age=3600'})

    async def handle_multiarch_fixes(request):
        from .multiarch_hints import render_start
        return web.Response(
            content_type='text/html', text=await render_start(),
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
                text=await write_history(
                    conn, limit=limit, is_admin=is_admin(request)),
                headers={'Cache-Control': 'max-age=10'})

    async def handle_queue(request):
        limit = int(request.query.get('limit', '100'))
        from .queue import write_queue
        async with request.app.database.acquire() as conn:
            return web.Response(
                content_type='text/html', text=await write_queue(
                    request.app.http_client_session, conn,
                    is_admin=is_admin(request),
                    queue_status=app.runner_status, limit=limit),
                headers={'Cache-Control': 'max-age=10'})

    async def handle_cupboard_stats(request):
        from .stats import write_stats
        async with request.app.database.acquire() as conn:
            return web.Response(
                content_type='text/html', text=await write_stats(
                    conn), headers={'Cache-Control': 'max-age=60'})

    async def handle_cupboard_maintainer_stats(request):
        from .stats import write_maintainer_stats
        async with request.app.database.acquire() as conn:
            return web.Response(
                content_type='text/html', text=await write_maintainer_stats(
                    conn), headers={'Cache-Control': 'max-age=60'})

    async def handle_result_codes(request):
        from .result_codes import (
            generate_result_code_index,
            generate_result_code_page)
        from .. import state
        suite = request.query.get('suite')
        if suite == '_all':
            suite = None
        code = request.match_info.get('code')
        all_suites = [s.name for s in config.suite]
        async with request.app.database.acquire() as conn:
            if not code:
                stats = await state.stats_by_result_codes(conn, suite=suite)
                if suite:
                    suites = [suite]
                else:
                    suites = None
                never_processed = sum(dict(
                    await state.get_never_processed(conn, suites)).values())
                text = await generate_result_code_index(
                    stats, never_processed, suite, all_suites=all_suites)
            else:
                runs = [run async for run in state.iter_last_runs(
                    conn, code, suite=suite)]
                text = await generate_result_code_page(
                    code, runs, suite, all_suites=all_suites)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_login(request):
        return web.Response(
            content_type='text/plain',
            text=repr(request.debsso_email))

    async def handle_static_file(path, request):
        return web.FileResponse(path)

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
                request.app.database, request.app.config,
                package, merge_proposals, runs)
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

    async def handle_vcs_regressions(request):
        template = request.app.jinja_env.get_template('vcs-regressions.html')
        async with request.app.database.acquire() as conn:
            regressions = await state.iter_vcs_regressions(conn)
        text = await template.render_async(regressions=regressions)
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
            request.app.archiver_url,
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
        # TODO(jelmer): Handle Accept: text/diff
        pkg = request.match_info['pkg']
        run_id = request.match_info.get('run_id')
        try:
            text = await generate_pkg_file(
                request.app.database,
                request.app.policy,
                request.app.http_client_session,
                request.app.archiver_url,
                request.app.publisher_url, pkg, run_id)
        except KeyError:
            raise web.HTTPNotFound()
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_multiarch_fixes_pkg(request):
        from .multiarch_hints import generate_pkg_file
        # TODO(jelmer): Handle Accept: text/diff
        pkg = request.match_info['pkg']
        run_id = request.match_info.get('run_id')
        try:
            text = await generate_pkg_file(
                request.app.database,
                request.app.policy,
                request.app.http_client_session,
                request.app.archiver_url,
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

    async def handle_multiarch_fixes_hint_list(request):
        from .multiarch_hints import generate_hint_list
        async with request.app.database.acquire() as conn:
            text = await generate_hint_list(conn)
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

    async def handle_multiarch_fixes_hint_page(request):
        from .multiarch_hints import generate_hint_page
        text = await generate_hint_page(
            request.app.database, request.match_info['hint'])
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_lintian_fixes_developer_table_page(request):
        from .lintian_fixes import generate_developer_table_page
        try:
            developer = request.match_info['developer']
        except KeyError:
            developer = request.query.get('developer')
        if developer and '@' not in developer:
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
                request.app.database, request.app.http_client_session,
                request.app.archiver_url,
                pkg, suite, run_id)
        except KeyError:
            raise web.HTTPNotFound()
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_lintian_fixes_candidates(request):
        from .lintian_fixes import generate_candidates
        text = await generate_candidates(
            request.app.database)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_multiarch_fixes_candidates(request):
        from .multiarch_hints import generate_candidates
        text = await generate_candidates(
            request.app.database)
        return web.Response(
            content_type='text/html', text=text,
            headers={'Cache-Control': 'max-age=600'})

    async def handle_lintian_fixes_stats(request):
        from .lintian_fixes import generate_stats
        text = await generate_stats(request.app.database)
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

    async def handle_rejected(request):
        from .review import generate_rejected
        suite = request.query.get('suite')
        async with request.app.database.acquire() as conn:
            text = await generate_rejected(conn, suite=suite)
        return web.Response(content_type='text/html', text=text)

    async def handle_review_post(request):
        from .review import generate_review
        post = await request.post()
        async with request.app.database.acquire() as conn:
            run = await state.get_run(conn, post['run_id'])
            review_status = post['review_status'].lower()
            if review_status == 'reschedule':
                review_status = 'rejected'
                from ..schedule import do_schedule
                await do_schedule(
                    conn, run.package, run.suite,
                    refresh=True, requestor='reviewer')
            await state.set_run_review_status(
                conn, post['run_id'], review_status)
            text = await generate_review(
                conn, request.app.http_client_session,
                request.app.archiver_url, request.app.publisher_url,
                suites=[run.suite])
        return web.Response(content_type='text/html', text=text)

    async def handle_review(request):
        from .review import generate_review
        suites = request.query.getall('suite', None)
        async with request.app.database.acquire() as conn:
            text = await generate_review(
                conn, request.app.http_client_session,
                request.app.archiver_url, request.app.publisher_url,
                suites=suites)
        return web.Response(content_type='text/html', text=text)

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
    app.router.add_get(
        '/multiarch-fixes/', handle_multiarch_fixes,
        name='multiarch-fixes-start')
    app.router.add_get(
        '/multiarch-fixes/by-hint/', handle_multiarch_fixes_hint_list,
        name='multiarch-fixes-hint-list')
    app.router.add_get(
        '/multiarch-fixes/by-hint/{hint}', handle_multiarch_fixes_hint_page,
        name='multiarch-fixes-hint')
    app.router.add_get(
        '/multiarch-fixes/candidates', handle_multiarch_fixes_candidates,
        name='multiarch-fixes-candidates')
    for suite in ['lintian-fixes', 'fresh-snapshots', 'fresh-releases',
                  'multiarch-fixes']:
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
    apt_location = config.apt_location or args.archiver_url
    app.router.register_resource(
        ForwardedResource('dists', apt_location.rstrip('/') + '/dists'))
    app.router.register_resource(
        ForwardedResource('pool', apt_location.rstrip('/') + '/pool'))
    app.router.register_resource(
        ForwardedResource(
            'bzr', args.publisher_url.rstrip('/') + '/bzr'))
    app.router.register_resource(
        ForwardedResource(
            'git', args.publisher_url.rstrip('/') + '/git'))
    app.router.add_get(
        '/multiarch-fixes/pkg/{pkg}/', handle_multiarch_fixes_pkg,
        name='multiarch-fixes-package')
    app.router.add_get(
        '/multiarch-fixes/pkg/{pkg}/{run_id}', handle_multiarch_fixes_pkg,
        name='multiarch-fixes-package-run')
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
    app.router.add_get(
        '/lintian-fixes/stats', handle_lintian_fixes_stats,
        name='lintian-fixes-stats')
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
        '/cupboard/stats', handle_cupboard_stats, name='cupboard-stats')
    app.router.add_get(
        '/cupboard/maintainer-stats', handle_cupboard_maintainer_stats,
        name='cupboard-maintainer-stats')
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
        '/cupboard/review', handle_review,
        name='cupboard-review')
    app.router.add_get(
        '/cupboard/rejected', handle_rejected,
        name='cupboard-rejected')
    app.router.add_post(
        '/cupboard/review', handle_review_post,
        name='cupboard-review-post')
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
        '/cupboard/vcs-regressions/',
        handle_vcs_regressions,
        name='vcs-regressions')
    app.router.add_get(
        '/login', handle_login,
        name='login')
    for entry in os.scandir(
            os.path.join(os.path.dirname(__file__), '_static')):
        app.router.add_get(
            '/_static/%s' % entry.name,
            functools.partial(handle_static_file, entry.path))
    app.router.add_get(
        '/janitor.asc', functools.partial(
            handle_static_file,
            os.path.join(
                os.path.dirname(__file__), '..', '..', 'janitor.asc')),
        name='gpg-key')
    app.router.add_get(
        '/_static/chart.js', functools.partial(
            handle_static_file,
            '/usr/share/javascript/chart.js/Chart.%sjs' % minified))
    app.router.add_get(
        '/_static/chart.css', functools.partial(
            handle_static_file,
            '/usr/share/javascript/chart.js/Chart.%scss' % minified))
    app.router.add_get(
        '/_static/jquery.js', functools.partial(
            handle_static_file,
            '/usr/share/javascript/jquery/jquery.%sjs' % minified))
    app.router.add_get(
        '/_static/moment.js', functools.partial(
            handle_static_file,
            '/usr/share/javascript/moment/moment.%sjs' % minified))
    from .api import create_app as create_api_app
    with open(args.policy, 'r') as f:
        policy_config = read_policy(f)

    app.http_client_session = ClientSession()
    app.topic_notifications = Topic()
    app.runner_url = args.runner_url
    app.archiver_url = args.archiver_url
    app.policy = policy_config
    app.publisher_url = args.publisher_url
    app.on_startup.append(start_pubsub_forwarder)
    app.database = state.Database(config.database_location)
    app.config = config
    from janitor.site import env, is_admin
    app.jinja_env = env
    setup_debsso(app)
    setup_metrics(app)
    app.router.add_get(
        '/ws/notifications',
        functools.partial(pubsub_handler,
                          app.topic_notifications),  # type: ignore
        name='ws-notifications')
    app.add_subapp(
        '/api', create_api_app(
            app.database, args.publisher_url, args.runner_url,  # type: ignore
            args.archiver_url, policy_config))
    web.run_app(app, host=args.host, port=args.port)
