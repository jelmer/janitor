#!/usr/bin/python3
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

from aiohttp import ClientSession, UnixConnector
from aiohttp.web_middlewares import normalize_path_middleware
import asyncio
import json
import os
import re
import sys
import traceback
from typing import List, Tuple, Optional
from urllib.parse import quote

from aiohttp import web
from debian.deb822 import Changes
from debian.changelog import Changelog, Version
from debian.copyright import parse_multiline

from .config import read_config, Config, Suite
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
from .trace import note, warning

suite_check = re.compile('^[a-z0-9-]+$')


async def handle_upload(request):
    directory = request.match_info['directory']
    container = os.path.join(request.app.incoming_dir, directory)
    os.mkdir(container)
    reader = await request.multipart()
    filenames = []
    result = {}
    while True:
        part = await reader.next()
        if part is None:
            break
        path = os.path.join(container, part.filename)
        filenames.append(part.filename)
        with open(path, 'wb') as f:
            f.write(await part.read())
        if path.endswith('.changes'):
            result['changes_filename'] = os.path.basename(path)
            with open(path, 'r') as f:
                changes = Changes(f)
                result['source'] = changes['Source']
                result['version'] = changes['Version']
                result['distribution'] = changes['Distribution']
                result['changes'] = changes['Changes']
    note('Uploaded files: %r', filenames)
    result['filenames'] = filenames
    return web.json_response(result, status=200)


def find_binary_paths_from_changes(incoming_dir, source, version):
    for entry in os.scandir(incoming_dir):
        if not entry.name.endswith('.changes'):
            continue
        binaries = []
        with open(entry.path, 'rb') as f:
            changes = Changes(f)
            if changes['Source'] != source:
                continue
            if changes['Version'] != version:
                continue
            for fd in changes['Files']:
                if not fd['name'].endswith('.deb'):
                    continue
                binaries.append(
                    (fd['name'].split('_')[0],
                     os.path.join(incoming_dir, fd['name'])))
        return binaries


def find_binary_paths_in_pool(
        archive_path: str, source: str,
        version: Version) -> List[Tuple[str, str]]:
    version = Version(str(version))
    version.epoch = None
    ret = []
    pool_dir = os.path.join(archive_path, "pool")
    for component in os.scandir(pool_dir):
        if source.startswith('lib'):
            sd = os.path.join(component.path, source[:4], source)
        else:
            sd = os.path.join(component.path, source[:1], source)
        if not os.path.isdir(sd):
            continue
        for binary in os.scandir(sd):
            (basename, ext) = os.path.splitext(binary.name)
            if ext != '.deb':
                continue
            (name, binary_version, arch) = basename.split('_', 2)
            if binary_version == version:
                ret.append((name, binary.path))
    ret.sort()
    if not ret:
        raise FileNotFoundError(
            'No binary package for source %s/%s found' % (
                source, version))
    return ret


def find_binary_paths(
        incoming_dir: str,
        archive_path: str,
        source: str,
        version: str) -> Optional[List[Tuple[str, str]]]:
    binaries = find_binary_paths_from_changes(incoming_dir, source, version)
    if binaries is not None:
        return binaries
    try:
        return find_binary_paths_in_pool(archive_path, source, version)
    except FileNotFoundError:
        return None


async def handle_debdiff(request):
    post = await request.post()

    old_suite = post.get('old_suite', 'unchanged')
    if not suite_check.match(old_suite):
        return web.Response(
            status=400, text='Invalid old suite %s' % old_suite)

    try:
        new_suite = post['new_suite']
    except KeyError:
        return web.Response(
            status=400, text='Missing argument: new_suite')

    if not suite_check.match(new_suite):
        return web.Response(
            status=400, text='Invalid new suite %s' % new_suite)

    source = post.get('source')
    if not source:
        return web.Response(status=400, text='No source package specified')

    old_version = post['old_version']
    new_version = post['new_version']

    archive_path = request.app.archive_path

    old_binaries = find_binary_paths(
            request.app.incoming_dir, archive_path,
            source, old_version)

    if old_binaries is None:
        return web.Response(
            status=404, text='Old source %s/%s does not exist.' % (
                source, old_version))

    new_binaries = find_binary_paths(
            request.app.incoming_dir, archive_path,
            source, new_version)

    if new_binaries is None:
        return web.Response(
            status=404, text='New source %s/%s does not exist.' % (
                source, new_version))

    try:
        debdiff = await run_debdiff(old_binaries, new_binaries)
    except DebdiffError as e:
        return web.Response(status=400, text=e.args[0])

    if 'filter_boring' in post:
        debdiff = filter_debdiff_boring(
            debdiff.decode(), old_version,
            new_version).encode()

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

    old_suite = post.get('old_suite', 'unchanged')
    if not suite_check.match(old_suite):
        return web.Response(
            status=400, text='Invalid old suite %s' % old_suite)

    try:
        new_suite = post['new_suite']
    except KeyError:
        return web.Response(
            status=400, text='Missing argument: new_suite')

    if not suite_check.match(new_suite):
        return web.Response(
            status=400, text='Invalid new suite %s' % new_suite)

    source = post['source']
    old_version = post['old_version']
    new_version = post['new_version']

    old_binaries = find_binary_paths(
            request.app.incoming_dir, request.app.archive_path,
            source, old_version)

    if old_binaries is None:
        return web.Response(
            status=404, text='Old source %s/%s does not exist.' % (
                source, old_version))

    new_binaries = find_binary_paths(
            request.app.incoming_dir, request.app.archive_path,
            source, new_version)

    if new_binaries is None:
        return web.Response(
            status=404, text='New source %s/%s does not exist.' % (
                source, new_version))

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

    def set_limits():
        import resource
        # Limit to 200Mb
        resource.setrlimit(
            resource.RLIMIT_AS, (200 * 1024 * 1024, 200 * 1024 * 1024))

    try:
        diffoscope_diff = await run_diffoscope(
            old_binaries, new_binaries,
            set_limits)
    except MemoryError:
        raise web.HTTPServiceUnavailable(
            text='diffoscope used too much memory')

    diffoscope_diff['source1'] = '%s version %s (%s)' % (
        source, old_version, old_suite)
    diffoscope_diff['source2'] = '%s version %s (%s)' % (
        source, new_version, new_suite)

    filter_diffoscope_irrelevant(diffoscope_diff)

    title = 'diffoscope for %s applied to %s' % (new_suite, source)

    if 'filter_boring' in post:
        filter_diffoscope_boring(
            diffoscope_diff, old_version,
            new_version, old_suite, new_suite)
        title += ' (filtered)'

    debdiff = await format_diffoscope(
        diffoscope_diff, content_type,
        title=title, jquery_url=post.get('jquery_url'),
        css_url=post.get('css_url'))

    return web.Response(text=debdiff, content_type=content_type)


class AptlyError(Exception):

    def __init__(self, message):
        self.message = message


async def aptly_call(aptly_session, method, path, json=None, params=None):
    async with aptly_session.request(
            method=method, url='http://localhost/api/' + path, json=json,
            params=params) as resp:
        if resp.status // 100 != 2:
            raise AptlyError(
                'error %d in /api/%s: %s' % (
                    resp.status, path, await resp.read()))
        return await resp.json()


async def handle_publish(request):
    post = await request.post()
    suites_processed = []
    failed_suites = []
    publish = {}
    for p in await aptly_call(request.app.aptly_session, 'GET', 'publish'):
        publish[(p['Storage'], p['Prefix'], p['Distribution'])] = p

    for suite in request.app.config.suite:
        if post.get('suite') not in (suite.name, None):
            continue
        storage = post.get('storage', '')
        prefix = post.get('prefix', '.')
        loc = "%s:%s" % (storage, prefix)
        params = {
            'SourceKind': 'local',
            'Sources': [suite.name],
            'Distribution': suite.name,
            'Label': suite.archive_description,
            'NotAutomatic': 'yes',
            'ButAutomaticUpgrades': 'yes',
            }
        if request.app.config.origin:
            params['Origin'] = request.app.config.origin

        if (storage, prefix, suite.name) in publish:
            try:
                await aptly_call(
                    request.app.aptly_session, 'PUT',
                    'publish/%s/%s' % (loc, suite.name),
                    json=params)
            except AptlyError:
                traceback.print_exc()
                failed_suites.append(suite.name)
            else:
                suites_processed.append(suite.name)
        else:
            try:
                await aptly_call(
                    request.app.aptly_session, 'POST', 'publish/%s' % loc,
                    json=params)
            except AptlyError:
                traceback.print_exc()
                failed_suites.append(suite.name)
            else:
                suites_processed.append(suite.name)
    return web.json_response(
        {'suites-processed': suites_processed,
         'suites-failed': failed_suites})


async def handle_pending(request):
    try:
        dirname = request.match_info['subdir']
    except KeyError:
        path = 'files'
    else:
        path = 'files/%s' % % dirname
    json = await aptly_call(request.app.aptly_session, 'GET', path)
    return web.json_response(json)


async def run_web_server(listen_addr, port, archive_path, incoming_dir,
                         aptly_session, config):
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[trailing_slash_redirect])
    app.archive_path = archive_path
    app.incoming_dir = incoming_dir
    app.aptly_session = aptly_session
    app.config = config
    setup_metrics(app)
    app.router.add_post('/upload/{directory}', handle_upload, name='upload')
    app.router.add_post('/debdiff', handle_debdiff, name='debdiff')
    app.router.add_post('/diffoscope', handle_diffoscope, name='diffoscope')
    app.router.add_static('/dists', os.path.join(archive_path, 'dists'))
    app.router.add_static('/pool', os.path.join(archive_path, 'pool'))
    app.router.add_post('/publish', handle_publish, name='publish')
    app.router.add_get('/pending/', handle_pending, name='pending')
    app.router.add_get('/pending/{subdir}', handle_pending, name='pending')
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()


async def upload_directory(aptly_session: ClientSession, directory: str):
    suite = None
    changes_filename = None
    for entry in os.scandir(directory):
        if not entry.name.endswith('.changes'):
            continue
        with open(entry.path, 'r') as f:
            changes = Changes(f)
            changes_filename = entry.name
            cl = Changelog(parse_multiline(changes['Changes']))
            suite = cl.distributions
    if suite is None or changes_filename is None:
        warning('No valid changes file found, skipping %s',
                directory)
        return
    try:
        result = await aptly_call(
            aptly_session, 'POST', 'repos/%s/include/%s/%s' % (
                    suite, quote(os.path.basename(directory)),
                    quote(changes_filename)),
            params={'acceptUnsigned': 1})
    except AptlyError as e:
        warning('Unable to include files %s in %s: %s ',
                directory, suite, e)
    else:
        note('Result from aptly include: %r', result)


async def update_aptly(aptly_session: ClientSession, incoming_dir: str):
    for entry in os.scandir(incoming_dir):
        if not entry.is_dir():
            # Weird
            continue
        await upload_directory(aptly_session, entry.path)


async def update_archive_loop(
        config: Config, aptly_session: ClientSession, incoming_dir: str):
    # Give aptly some time to start
    await asyncio.sleep(25)
    while True:
        await update_aptly(aptly_session, incoming_dir)
        await asyncio.sleep(30 * 60)


async def sync_aptly_repos(session: ClientSession, suites: List[Suite]):
    repos = {repo['Name']: repo for repo in await aptly_call(
        session, 'GET', 'repos')}
    for suite in suites:
        intended_repo = {
            'Name': suite.name,
            'DefaulDistribution': suite.name,
            'DefaultComponent': '',
            'Comment': ''}
        actual_repo = repos.get(suite.name)
        if intended_repo == actual_repo:
            del repos[suite.name]
            continue
        if not actual_repo:
            await aptly_call(session, 'POST', 'repos', json=intended_repo)
        else:
            await aptly_call(
                session, 'PUT', 'repos/%s' % intended_repo.pop('Name'),
                json=intended_repo)
            del repos[suite.name]
    for suite in repos:
        await aptly_call(session, 'DELETE', 'repos/%s' % suite)


async def sync_aptly(aptly_session: ClientSession, suites: List[Suite]):
    # Give aptly some time to start
    await asyncio.sleep(15)
    ret = await aptly_call(aptly_session, 'GET', 'version')
    note('aptly version %s connected' % ret['Version'])
    await sync_aptly_repos(aptly_session, suites)


async def run_aptly(sock_path, config_path):
    args = [
        '/usr/bin/aptly', 'api', 'serve',
        '-listen=unix://%s' % sock_path,
        '-config=%s' % config_path]
    proc = await asyncio.create_subprocess_exec(*args)
    ret = await proc.wait()
    raise Exception('aptly finished with exit code %r' % ret)


def main(argv=None):
    import argparse
    parser = argparse.ArgumentParser(prog='janitor.archive')
    parser.add_argument(
        '--listen-address', type=str,
        help='Listen address', default='localhost')
    parser.add_argument(
        '--port', type=int,
        help='Listen port', default=9914)
    parser.add_argument(
        '--config', type=str, default='janitor.conf',
        help='Path to configuration.')
    parser.add_argument(
        '--aptly-config-path', type=str, default='~/.aptly.conf',
        help='Path to aptly configuration')
    parser.add_argument(
        '--aptly-socket-path', type=str,
        default='aptly.sock',
        help='Path to aptly socket')

    args = parser.parse_args()

    with open(args.config, 'r') as f:
        config = read_config(f)

    aptly_config_path = os.path.expanduser(args.aptly_config_path)
    with open(aptly_config_path, 'r') as f:
        aptly_config = json.load(f)

    aptly_root_dir = aptly_config['rootDir']

    incoming_dir = os.path.join(aptly_root_dir, 'upload')
    if not os.path.exists(incoming_dir):
        os.mkdir(incoming_dir)
    archive_dir = os.path.join(aptly_root_dir, 'public')

    aptly_socket_path = os.path.abspath(args.aptly_socket_path)
    if os.path.exists(aptly_socket_path):
        os.remove(aptly_socket_path)

    loop = asyncio.get_event_loop()
    aptly_session = ClientSession(
        connector=UnixConnector(path=aptly_socket_path))

    for name in ['dists', 'pool']:
        try:
            os.makedirs(os.path.join(archive_dir, name))
        except FileExistsError:
            pass

    loop.run_until_complete(asyncio.gather(
        loop.create_task(run_aptly(aptly_socket_path, aptly_config_path)),
        loop.create_task(sync_aptly(aptly_session, config.suite)),
        loop.create_task(run_web_server(
            args.listen_address, args.port, archive_dir,
            incoming_dir, aptly_session, config)),
        loop.create_task(update_archive_loop(
            config, aptly_session, incoming_dir))))
    loop.run_forever()


if __name__ == '__main__':
    sys.exit(main(sys.argv))
