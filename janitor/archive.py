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
import os
import re
import subprocess
import sys

from aiohttp import web
from debian.deb822 import Changes

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
from .trace import note

suite_check = re.compile('^[a-z0-9-]+$')


async def handle_upload(request):
    reader = await request.multipart()
    filenames = []
    while True:
        part = await reader.next()
        if part is None:
            break
        path = os.path.join(request.app.incoming_dir, part.filename)
        filenames.append(part.filename)
        with open(path, 'wb') as f:
            f.write(await part.read(decode=False))
    note('Uploaded files: %r', filenames)
    return web.Response(status=200, text='Uploaded files: %r.' % filenames)


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
            for f in changes['Files']:
                if not f['name'].endswith('.deb'):
                    continue
                binaries.append(
                    (f['name'].split('_')[0],
                     os.path.join(incoming_dir, f['name'])))
        return binaries


def find_binary_paths_in_pool(
        archive_path, source, version):
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


async def find_binary_paths(
        incoming_dir, archive_path, source, version):
    binaries = find_binary_paths_from_changes(incoming_dir, source, version)
    if binaries is not None:
        return binaries
    try:
        return find_binary_paths_in_pool(
            archive_path, source, version)
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

    old_binaries = await find_binary_paths(
            request.app.incoming_dir, archive_path,
            source, old_version)

    if old_binaries is None:
        return web.Response(
            status=404, text='Old source %s/%s does not exist.' % (
                source, old_version))

    new_binaries = await find_binary_paths(
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

    old_binaries = await find_binary_paths(
            request.app.incoming_dir, request.app.archive_path,
            source, old_version)

    if old_binaries is None:
        return web.Response(
            status=404, text='Old source %s/%s does not exist.' % (
                source, old_version))

    new_binaries = await find_binary_paths(
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
        title=title)

    return web.Response(text=debdiff, content_type=content_type)


async def handle_publish(request):
    post = await request.post()
    conn = UnixConnector(path=request.app.aptly_socket_path)
    suites_processed = []
    failed_suites = []
    # First, retrierve
    async with ClientSession(connector=conn) as session:
        async with session.get('http://localhost/api/publish') as resp:
            if resp.status != 200:
                raise Exception('retrieving in progress publish failed')
            publish = {}
            for p in await resp.json():
                publish[(p['Storage'], p['Prefix'], p['Distribution'])] = p

        for suite in request.app.config.suite:
            if post.get('suite') not in (suite.name, None):
                continue
            storage = ''
            prefix = '.'
            loc = "%s:%s" % (storage, prefix)
            if (storage, prefix, suite.name) in publish:
                async with session.put('http://localhost/api/publish/%s/%s'
                                       % (loc, suite.name)):
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
                    'Origin': 'janitor.debian.net',
                    'NotAutomatic': 'yes',
                    'ButAutomaticUpgrades': 'yes',
                    }
                async with session.post(
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
                         aptly_socket_path, config):
    app = web.Application()
    app.archive_path = archive_path
    app.incoming_dir = incoming_dir
    app.aptly_socket_path = aptly_socket_path
    app.config = config
    setup_metrics(app)
    app.router.add_post('/', handle_upload, name='upload')
    app.router.add_post('/debdiff', handle_debdiff, name='debdiff')
    app.router.add_post('/diffoscope', handle_diffoscope, name='diffoscope')
    app.router.add_static('/dists', os.path.join(archive_path, 'dists'))
    app.router.add_static('/pool', os.path.join(archive_path, 'pool'))
    app.router.add_post('/publish', handle_publish, name='publish')
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()


async def update_aptly(incoming_dir):
    args = [
        "/usr/bin/aptly", "repo", "include",
        "-keyring=/home/janitor/debian-janitor/janitor.gpg"]
    args.append(incoming_dir)
    proc = await asyncio.create_subprocess_exec(*args)
    await proc.wait()


async def update_archive_loop(config, incoming_dir):
    while True:
        await update_aptly(incoming_dir)
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
    # TODO(jelmer): Drop/return repositories not referenced?


async def sync_aptly(socket_path, suites):
    # Give aptly some time to start
    await asyncio.sleep(15)
    conn = UnixConnector(path=socket_path)
    async with ClientSession(connector=conn) as session:
        async with session.get('http://localhost/api/version') as resp:
            if resp.status != 200:
                raise Exception('failed: %r' % resp.status)
            ret = await resp.json()
            print('aptly version %s connected' % ret['Version'])
        await sync_aptly_repos(session, suites)

    # TODO(jelmer): Use API
    for suite in suites:
        subprocess.call(
            ['/usr/bin/aptly', 'publish', 'repo', '-notautomatic=yes',
             '-butautomaticupgrades=yes', '-origin=janitor.debian.net',
             '-label=%s' % suite.archive_description,
             '-distribution=%s' % suite.name,
             suite.name])


async def run_aptly(sock_path):
    args = [
        '/usr/bin/aptly', 'api', 'serve', '-listen=unix://%s' % sock_path,
        '-no-lock']
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
        '--archive', type=str,
        help='Path to the apt archive.')
    parser.add_argument(
        '--config', type=str, default='janitor.conf',
        help='Path to configuration.')
    parser.add_argument(
        '--incoming', type=str,
        help='Path to incoming directory.')
    parser.add_argument(
        '--aptly-socket-path', type=str,
        default='aptly.sock',
        help='Path to aptly socket')

    args = parser.parse_args()

    with open(args.config, 'r') as f:
        config = read_config(f)

    aptly_socket_path = os.path.abspath(args.aptly_socket_path)
    if os.path.exists(aptly_socket_path):
        os.remove(aptly_socket_path)

    loop = asyncio.get_event_loop()
    loop.run_until_complete(asyncio.gather(
        loop.create_task(run_aptly(aptly_socket_path)),
        loop.create_task(sync_aptly(aptly_socket_path, config.suite)),
        loop.create_task(run_web_server(
            args.listen_address, args.port, args.archive,
            args.incoming, args.aptly_socket_path, config)),
        loop.create_task(update_archive_loop(
            config, args.incoming))))
    loop.run_forever()


if __name__ == '__main__':
    sys.exit(main(sys.argv))
