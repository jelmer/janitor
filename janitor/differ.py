#!/usr/bin/python3
# Copyright (C) 2019-2020 Jelmer Vernooij <jelmer@jelmer.uk>
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

from aiohttp.web_middlewares import normalize_path_middleware
from aiohttp import ClientSession
import asyncio
from contextlib import ExitStack
import json
import os
import re
import sys
from tempfile import TemporaryDirectory
from yarl import URL

from aiohttp import web, MultipartReader

from . import state
from .artifacts import ArtifactsMissing, get_artifact_manager
from .config import read_config
from .debdiff import (
    run_debdiff,
    DebdiffError,
    filter_boring as filter_debdiff_boring,
    htmlize_debdiff,
    markdownify_debdiff,
    )
from .diffoscope import (
    filter_boring as filter_diffoscope_boring,
    filter_irrelevant as filter_diffoscope_irrelevant,
    run_diffoscope,
    format_diffoscope,
    )
from .prometheus import setup_metrics


suite_check = re.compile('^[a-z0-9-]+$')


async def retrieve_artifacts_from_apt(archiver_url, run, target):
    url = URL(archiver_url + '/download')
    url = url.with_query({
        'source': run.package,
        'version': str(run.build_version),
        'suite': run.suite})
    async with ClientSession() as session:
        async with session.get(url) as resp:
            if resp.status == 404:
                raise ArtifactsMissing(run.id, await resp.text())
            if resp != 200:
                raise Exception('failed to retrieve artifact (%s): %d' % (
                    url, resp.status))
            reader = MultipartReader.from_response(resp)
            while True:
                part = await reader.next()
                if part is None:
                    break
                with open(os.path.join(target, part.filename), 'wb') as f:
                    f.write(await part.read())


async def retrieve_artifacts(artifact_manager, run, target, archiver_url=None):
    try:
        await artifact_manager.retrieve_artifacts(run.id, target)
    except ArtifactsMissing:
        pass
    else:
        return
    if archiver_url is not None:
        await retrieve_artifacts_from_apt(archiver_url, run, target)
    raise ArtifactsMissing(run.id)


def find_binaries(path):
    ret = []
    for entry in os.scandir(path):
        ret.append((entry.name, entry.path))
    return ret


async def handle_debdiff(request):
    old_id = request.match_info['old_id']
    new_id = request.match_info['new_id']

    async with request.app.db.acquire() as conn:
        old_run = await state.get_run(conn, old_id)
        new_run = await state.get_run(conn, new_id)

    if request.app.debdiff_cache_path:
        cache_path = os.path.join(
            request.app.debdiff_cache_path, '%s-%s' % (old_id, new_id))
        try:
            with open(cache_path, 'rb') as f:
                debdiff = f.read()
        except FileNotFoundError:
            debdiff = None
    else:
        cache_path = None
        debdiff = None

    if debdiff is None:
        with ExitStack() as es:
            old_dir = es.enter_context(TemporaryDirectory())
            new_dir = es.enter_context(TemporaryDirectory())

            # TODO: parallelize
            try:
                await retrieve_artifacts(
                    request.app.artifact_manager, old_run, old_dir,
                    request.app.archiver_url)
            except ArtifactsMissing as e:
                raise web.HTTPNotFound(
                    text='No artifacts for run id %s: %r' % (old_run.id, e))
            try:
                await retrieve_artifacts(
                    request.app.artifact_manager, new_run, new_dir,
                    request.app.archiver_url)
            except ArtifactsMissing as e:
                raise web.HTTPNotFound(
                    text='No artifacts for run id %s: %r' % (new_run.id, e))

            old_binaries = find_binaries(old_dir)
            new_binaries = find_binaries(new_dir)

            try:
                debdiff = await run_debdiff(old_binaries, new_binaries)
            except DebdiffError as e:
                return web.Response(status=400, text=e.args[0])

        if cache_path:
            with open(cache_path, 'wb') as f:
                f.write(debdiff)

    if 'filter_boring' in request.query:
        debdiff = filter_debdiff_boring(
            debdiff.decode(), old_run.build_version,
            new_run.build_version).encode()

    for accept in request.headers.get('ACCEPT', '*/*').split(','):
        if accept in ('text/x-diff', 'text/plain', '*/*'):
            return web.Response(
                body=debdiff,
                content_type='text/plain')
        if accept == 'text/markdown':
            return web.Response(
                text=markdownify_debdiff(debdiff.decode('utf-8', 'replace')),
                content_type='text/markdown')
        if accept == 'text/html':
            return web.Response(
                text=htmlize_debdiff(debdiff.decode('utf-8', 'replace')),
                content_type='text/html')
    raise web.HTTPNotAcceptable(
        text='Acceptable content types: '
             'text/html, text/plain, text/markdown')


async def handle_diffoscope(request):
    post = await request.post()

    for accept in request.headers.get('ACCEPT', '*/*').split(','):
        if accept in ('text/plain', '*/*'):
            content_type = 'text/plain'
            break
        elif accept in ('text/html', ):
            content_type = 'text/html'
            break
        elif accept in ('application/json', ):
            content_type = 'application/json'
            break
        elif accept in ('text/markdown', ):
            content_type = 'text/markdown'
            break
    else:
        raise web.HTTPNotAcceptable(
            text='Acceptable content types: '
                 'text/html, text/plain, application/json, '
                 'application/markdown')

    old_id = request.match_info['old_id']
    new_id = request.match_info['new_id']

    async with request.app.db.acquire() as conn:
        old_run = await state.get_run(conn, old_id)
        new_run = await state.get_run(conn, new_id)

    if request.app.diffoscope_cache_path:
        cache_path = os.path.join(
            request.app.diffoscope_cache_path,
            '%s-%s.json' % (old_run.id, new_run.id))
        try:
            with open(cache_path, 'rb') as f:
                diffoscope_diff = json.load(f)
        except FileNotFoundError:
            diffoscope_diff = None
    else:
        cache_path = None
        diffoscope_diff = None

    if diffoscope_diff is None:
        with ExitStack() as es:
            old_dir = es.enter_context(TemporaryDirectory())
            new_dir = es.enter_context(TemporaryDirectory())

            # TODO: parallelize
            try:
                await retrieve_artifacts(
                    request.app.artifact_manager, old_run, old_dir,
                    request.app.archiver_url)
            except ArtifactsMissing as e:
                raise web.HTTPNotFound(
                    text='No artifacts for run id %s: %r' % (old_run.id, e))
            try:
                await retrieve_artifacts(
                    request.app.artifact_manager, new_run, new_dir,
                    request.app.archiver_url)
            except ArtifactsMissing as e:
                raise web.HTTPNotFound(
                    text='No artifacts for run id %s: %r' % (new_run.id, e))

            old_binaries = find_binaries(old_dir)
            new_binaries = find_binaries(new_dir)

            def set_limits():
                import resource
                # Limit to 2Gb
                resource.setrlimit(
                    resource.RLIMIT_AS, (1800 * 1024 * 1024, 2000 * 1024 * 1024))

            try:
                diffoscope_diff = await asyncio.wait_for(
                        run_diffoscope(
                            old_binaries, new_binaries,
                            set_limits), 60.0)
            except MemoryError:
                raise web.HTTPServiceUnavailable(
                     'diffoscope used too much memory')
            except asyncio.TimeoutError:
                raise web.HTTPServiceUnavailable('diffoscope timed out')
        
        if cache_path is not None:
            with open(cache_path, 'w') as f:
                json.dump(diffoscope_diff, f)

    diffoscope_diff['source1'] = '%s version %s (%s)' % (
        old_run.package, old_run.build_version, old_run.suite)
    diffoscope_diff['source2'] = '%s version %s (%s)' % (
        new_run.package, new_run.build_version, new_run.suite)

    filter_diffoscope_irrelevant(diffoscope_diff)

    title = 'diffoscope for %s applied to %s' % (
        new_run.suite, new_run.package)

    if 'filter_boring' in post:
        filter_diffoscope_boring(
            diffoscope_diff, old_run.build_version,
            new_run.build_version, old_run.suite, new_run.suite)
        title += ' (filtered)'

    debdiff = await format_diffoscope(
        diffoscope_diff, content_type,
        title=title, jquery_url=post.get('jquery_url'),
        css_url=post.get('css_url'))

    return web.Response(text=debdiff, content_type=content_type)


async def run_web_server(listen_addr, port, config, artifact_manager,
                         db, cache_path, archiver_url):
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[trailing_slash_redirect])
    app.db = db
    app.config = config
    app.cache_path = cache_path
    app.artifact_manager = artifact_manager
    app.archiver_url = archiver_url
    if cache_path:
        app.diffoscope_cache_path = os.path.join(cache_path, 'diffoscope')
        if not os.path.isdir(app.diffoscope_cache_path):
            os.mkdir(app.diffoscope_cache_path)
        app.debdiff_cache_path = os.path.join(cache_path, 'debdiff')
        if not os.path.isdir(app.debdiff_cache_path):
            os.mkdir(app.debdiff_cache_path)
    else:
        app.diffoscope_cache_path = None
        app.debdiff_cache_path = None
    setup_metrics(app)
    app.router.add_get(
        '/debdiff/{old_id}/{new_id}',
        handle_debdiff, name='debdiff')
    app.router.add_get(
        '/diffoscope/{old_id}/{new_id}',
        handle_diffoscope, name='diffoscope')
    async def connect_artifact_manager(app):
        await app.artifact_manager.__aenter__()
    app.on_startup.append(connect_artifact_manager)
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()


def main(argv=None):
    import argparse
    parser = argparse.ArgumentParser(prog='janitor.differ')
    parser.add_argument(
        '--listen-address', type=str,
        help='Listen address', default='localhost')
    parser.add_argument(
        '--port', type=int,
        help='Listen port', default=9920)
    parser.add_argument(
        '--config', type=str, default='janitor.conf',
        help='Path to configuration.')
    parser.add_argument(
        '--cache-path', type=str, default=None,
        help='Path to cache.')
    parser.add_argument(
        '--archiver-url', type=str, default=None,
        help='Archiver URL')

    args = parser.parse_args()

    with open(args.config, 'r') as f:
        config = read_config(f)

    artifact_manager = get_artifact_manager(config.artifact_location)

    db = state.Database(config.database_location)
    loop = asyncio.get_event_loop()

    loop.run_until_complete(run_web_server(
            args.listen_address, args.port, config, artifact_manager,
            db, args.cache_path, args.archiver_url))
    loop.run_forever()


if __name__ == '__main__':
    sys.exit(main(sys.argv))
