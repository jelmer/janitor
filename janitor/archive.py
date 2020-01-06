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

from aiohttp import ClientSession
import asyncio
from contextlib import ExitStack
import os
import re
import sys
import tempfile
import uuid

from aiohttp import web
from debian.deb822 import Changes

from .aptly import Aptly, AptlyError
from .debdiff import (
    run_debdiff,
    filter_boring,
    htmlize_debdiff,
    markdownify_debdiff,
    )
from .diffoscope import run_diffoscope
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


def find_changes_path(incoming_dir, archive_dir, suite, changes_filename):
    for path in [
            os.path.join(incoming_dir, changes_filename),
            os.path.join(archive_dir, suite, changes_filename)]:
        if os.path.exists(path):
            return path
    else:
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

    try:
        old_changes_filename = post['old_changes_filename']
    except KeyError:
        return web.Response(
            status=400, text='Missing argument: old_changes_filename')

    if '/' in old_changes_filename:
        return web.Response(
            status=400,
            text='Invalid changes filename: %s' % old_changes_filename)

    try:
        new_changes_filename = post['new_changes_filename']
    except KeyError:
        return web.Response(
            status=400, text='Missing argument: new_changes_filename')

    if '/' in new_changes_filename:
        return web.Response(
            status=400,
            text='Invalid changes filename: %s' % new_changes_filename)

    archive_path = request.app.archive_path

    old_changes_path = find_changes_path(
            request.app.incoming_dir, archive_path, old_suite,
            old_changes_filename)

    if old_changes_path is None:
        return web.Response(
            status=404, text='Old changes file %s does not exist.' % (
                old_changes_filename))

    new_changes_path = find_changes_path(
            request.app.incoming_dir, archive_path, new_suite,
            new_changes_filename)

    if new_changes_path is None:
        return web.Response(
            status=404, text='New changes file %s does not exist.' % (
                new_changes_filename))

    with open(old_changes_path, 'r') as f:
        old_changes = Changes(f)

    with open(new_changes_path, 'r') as f:
        new_changes = Changes(f)

    debdiff = await run_debdiff(old_changes_path, new_changes_path)
    if 'filter_boring' in post:
        debdiff = filter_boring(
            debdiff.decode(), old_changes['Version'],
            new_changes['Version']).encode()

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

    try:
        old_changes_filename = post['old_changes_filename']
    except KeyError:
        return web.Response(
            status=400, text='Missing argument: old_changes_filename')

    if '/' in old_changes_filename:
        return web.Response(
            status=400,
            text='Invalid changes filename: %s' % old_changes_filename)

    try:
        new_changes_filename = post['new_changes_filename']
    except KeyError:
        return web.Response(
            status=400, text='Missing argument: new_changes_filename')

    if '/' in new_changes_filename:
        return web.Response(
            status=400,
            text='Invalid changes filename: %s' % new_changes_filename)

    archive_path = request.app.archive_path

    old_changes_path = find_changes_path(
            request.app.incoming_dir, archive_path, old_suite,
            old_changes_filename)

    if old_changes_path is None:
        return web.Response(
            status=404, text='Old changes file %s does not exist.' % (
                old_changes_filename))

    new_changes_path = find_changes_path(
            request.app.incoming_dir, archive_path, new_suite,
            new_changes_filename)

    if new_changes_path is None:
        return web.Response(
            status=404, text='New changes file %s does not exist.' % (
                new_changes_filename))

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

    debdiff = await run_diffoscope(
        old_changes_path, new_changes_path, content_type=content_type)

    return web.Response(body=debdiff, content_type=content_type)


async def handle_archive_file(request):
    filename = request.match_info['filename']
    if '/' in filename:
        return web.Response(
            text='Invalid filename %s' % request.match_info['filename'],
            status=400)

    full_path = os.path.join(
        request.app.archive_path,
        request.match_info['suite'],
        filename)

    if os.path.exists(full_path):
        return web.FileResponse(full_path)
    else:
        return web.Response(
            text='No such changes file : %s' % filename, status=404)


async def run_web_server(listen_addr, port, archive_path, incoming_dir):
    app = web.Application()
    app.archive_path = archive_path
    app.incoming_dir = incoming_dir
    setup_metrics(app)
    app.router.add_post('/', handle_upload, name='upload')
    app.router.add_post('/debdiff', handle_debdiff, name='debdiff')
    app.router.add_post('/diffoscope', handle_diffoscope, name='diffoscope')
    app.router.add_get(
        '/archive'
        '/{suite:[a-z0-9-]+}'
        '/{filename}', handle_archive_file, name='file')
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()


async def update_mini_dinstall(config, archive_dir):
    with tempfile.NamedTemporaryFile(mode='w') as f:
        with open('mini-dinstall.conf', 'r') as t:
            f.write(t.read() % {'archive_dir': archive_dir})
        for suite in config.suite:
            f.write('[%s]\n' % suite.name)
            f.write('experimental_release = 1\n')
            f.write('release_label = %s\n' % suite.archive_description)
            f.write('\n')

        f.flush()

        args = ['mini-dinstall', '-b', '-c', f.name]
        proc = await asyncio.create_subprocess_exec(*args)
        await proc.wait()


async def update_archive_loop(config, archive_dir, incoming_dir, aptly_url):
    while True:
        async with ClientSession() as session:
            aptly = Aptly(session, aptly_url)
            await update_aptly(incoming_dir, aptly)
            for suite in config.suite:
                await aptly.publish_update(':.', suite.name)
        await update_mini_dinstall(config, archive_dir)
        await asyncio.sleep(30 * 60)


async def upload_to_aptly(changes_path, aptly):
    dirname = str(uuid.uuid4())
    files = []
    uploaded = []
    with open(changes_path, 'r') as f, ExitStack() as es:
        changes = Changes(f)
        f.seek(0)
        files.append(f)
        distro = changes["Distribution"]
        uploaded.append(os.path.basename(changes_path))
        for file_details in changes['files']:
            path = os.path.join(
                os.path.dirname(changes_path), file_details['name'])
            uploaded.append(file_details['name'])
            f = open(path, 'rb')
            files.append(f)
            es.enter_context(f)
        await aptly.upload_files(dirname, files)
    await aptly.repos_include(distro, dirname)
    return uploaded


async def update_aptly(incoming_dir, aptly):
    for filename in os.listdir(incoming_dir):
        if filename.endswith('.changes'):
            print('uploading %s' % filename)
            await upload_to_aptly(os.path.join(incoming_dir, filename), aptly)


async def sync_aptly_metadata(config, aptly_url):
    async with ClientSession() as session:
        aptly = Aptly(session, aptly_url)
        existing_repos = await aptly.repos_list()
        existing_by_name = {r['Name']: r for r in existing_repos}
        for suite in config.suite:
            if suite.name in existing_by_name:
                del existing_by_name[suite.name]
            else:
                await aptly.repos_create(suite.name)
            try:
                await aptly.publish(
                    ':.', suite.name, not_automatic=True,
                    distribution=suite.name, architectures=['all', 'amd64'])
            except AptlyError as e:
                # 400 indicates it's already published
                if e.status != 400:
                    raise
        for suite_name in existing_by_name:
            await aptly.repos_delete(suite_name)


def main(argv=None):
    import argparse
    from .config import read_config
    parser = argparse.ArgumentParser(prog='janitor.runner')
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
        '--aptly-url', type=str, default='http://localhost:9915/api/',
        help='URL for aptly API.')
    parser.add_argument(
        '--incoming', type=str,
        help='Path to incoming directory.')

    args = parser.parse_args()

    with open(args.config, 'r') as f:
        config = read_config(f)

    loop = asyncio.get_event_loop()
    loop.run_until_complete(asyncio.gather(
        loop.create_task(run_web_server(
            args.listen_address, args.port, args.archive, args.incoming)),
        loop.create_task(update_archive_loop(
            config, args.archive, args.incoming, args.aptly_url)),
        loop.create_task(sync_aptly_metadata(config, args.aptly_url))))
    loop.run_forever()


if __name__ == '__main__':
    sys.exit(main(sys.argv))
