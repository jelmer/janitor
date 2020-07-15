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
import asyncio
import json
import os
import re
import sys
from typing import List, Tuple, Optional

from aiohttp import web
from debian.deb822 import Changes
from debian.changelog import Changelog, Version
from debian.copyright import parse_multiline

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


async def handle_publish(request):
    post = await request.post()
    suites_processed = []
    failed_suites = []
    async with request.app.aptly_session.get(
            'http://localhost/api/publish') as resp:
        if resp.status != 200:
            raise Exception('retrieving in progress publish failed')
        publish = {}
        for p in await resp.json():
            publish[(p['Storage'], p['Prefix'], p['Distribution'])] = p

    for suite in request.app.config.suite:
        if post.get('suite') not in (suite.name, None):
            continue
        storage = post.get('storage', '')
        prefix = post.get('prefix', '.')
        loc = "%s:%s" % (storage, prefix)
        if (storage, prefix, suite.name) in publish:
            async with request.app.aptly_session.put(
                    'http://localhost/api/publish/%s/%s' % (loc, suite.name)):
                if resp.status != 200:
                    failed_suites.append(suite.name)
                else:
                    suites_processed.append(suite.name)
        else:
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
            async with request.app.aptly_session.post(
                    'http://localhost/api/publish/%s' % loc,
                    json=params) as resp:
                if resp.status != 200:
                    failed_suites.append(suite.name)
                else:
                    suites_processed.append(suite.name)
    return web.json_response(
        {'suites-processed': suites_processed,
         'suites-failed': failed_suites})


async def run_web_server(listen_addr, port, archive_path, incoming_dir,
                         aptly_session, config):
    app = web.Application()
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
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()


async def upload_directory(aptly_session, directory):
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
    if suite is None:
        warning('No valid changes file found, skipping %s',
                directory)
        return
    async with aptly_session.post(
            'http://localhost/api/repos/%s/include/%s/%s' % (
                suite, os.path.basename(directory), changes_filename),
            data={'acceptUnsigned': 1}) as resp:
        if resp.status != 200:
            warning('Unable to include files %s in %s: %d ',
                    directory, suite, resp.status)
            return
        result = await resp.json()
        note('Result from aptly include: %r', result)


async def update_aptly(aptly_session, incoming_dir):
    for entry in os.scandir(incoming_dir):
        if not entry.is_dir():
            # Weird
            continue
        await upload_directory(aptly_session, entry.path)


async def update_archive_loop(config, aptly_session, incoming_dir):
    # Give aptly some time to start
    await asyncio.sleep(15)
    while True:
        await update_aptly(aptly_session, incoming_dir)
        await asyncio.sleep(30 * 60)


async def sync_aptly_repos(session, suites):
    async with session.get('http://localhost/api/repos') as resp:
        if resp.status != 200:
            raise Exception('failed: %r' % resp.status)
        repos = {repo['Name']: repo for repo in await resp.json()}
        print("current repos: %r" % repos)
    for suite in suites:
        intended_repo = {
            'Name': suite.name,
            'DefaulDistribution': suite.name,
            'DefaultComponent': '',
            'Comment': ''}
        if intended_repo == repos.get(suite.name):
            del repos[suite.name]
            continue
        if suite.name not in repos:
            async with session.post(
                    'http://localhost/api/repos',
                    json=intended_repo) as resp:
                if resp.status != 201:
                    raise Exception('Unable to create repo %s: %d' % (
                                    suite.name, resp.status))
        else:
            async with session.put(
                   'http://localhost/api/repos/%s' % intended_repo.pop('Name'),
                   json=intended_repo) as resp:
                if resp.status != 200:
                    raise Exception('Unable to edit repo %s: %d' % (
                                    suite.name, resp.status))
            del repos[suite.name]
    for suite in repos:
        async with session.delete(
                'http://localhost/api/repos/%s' % suite) as resp:
            if resp.status != 200:
                raise Exception('Error removing repo %s: %d' % (
                    suite, resp.status))


async def sync_aptly(aptly_session, suites):
    # Give aptly some time to start
    await asyncio.sleep(15)
    async with aptly_session.get('http://localhost/api/version') as resp:
        if resp.status != 200:
            raise Exception('failed: %r' % resp.status)
        ret = await resp.json()
        print('aptly version %s connected' % ret['Version'])
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
    from .config import read_config
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
    archive_dir = os.path.join(aptly_root_dir, 'public')

    aptly_socket_path = os.path.abspath(args.aptly_socket_path)
    if os.path.exists(aptly_socket_path):
        os.remove(aptly_socket_path)

    loop = asyncio.get_event_loop()
    aptly_session = ClientSession(
        connector=UnixConnector(path=aptly_socket_path))

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
