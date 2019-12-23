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
import json
import os
import re
import sys
import tempfile
from urllib.parse import urljoin

from aiohttp import web

from .aptly import Aptly, AptlyError
from .debdiff import run_debdiff, filter_boring
from .prometheus import setup_metrics

suite_check = re.compile('^[a-z0-9-]+$')


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

    old_changes_path = os.path.join(
        archive_path, old_suite, old_changes_filename)
    new_changes_path = os.path.join(
        archive_path, new_suite, new_changes_filename)

    if (not os.path.exists(old_changes_path) or
            not os.path.exists(new_changes_path)):
        return web.Response(
            status=400, text='One or both changes files do not exist.')

    debdiff = await run_debdiff(old_changes_path, new_changes_path)
    if 'ignore_boring' in post:
        debdiff = filter_boring(debdiff)

    return web.Response(body=debdiff, content_type='text/diff')


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


async def run_web_server(listen_addr, port, archive_path):
    app = web.Application()
    app.archive_path = archive_path
    setup_metrics(app)
    app.router.add_post('/debdiff', handle_debdiff, name='debdiff')
    app.router.add_get(
        '/archive'
        '/{suite:[a-z0-9-]+}'
        '/{filename}', handle_archive_file, name='file')
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()


async def update_archive(config, archive_dir):
    with tempfile.NamedTemporaryFile(mode='w') as f:
        with open('mini-dinstall.conf', 'r') as t:
            f.write(t.read() % {'archive_dir': archive_dir})
        for suite in config.suite:
            f.write('[%s]\n' % suite.name)
            f.write('experimental_release = 1\n')
            f.write('release_label = %s\n' % suite.archive_description)
            f.write('\n')

        f.flush()

        args = ['mini-dinstall', '-c', f.name]
        proc = await asyncio.create_subprocess_exec(*args)
        await proc.wait()


async def update_archive_loop(config, archive_dir):
    while True:
        await update_archive(config, archive_dir)
        await asyncio.sleep(30 * 60)


async def update_aptly(config, aptly):
    for suite in config.suite:
        await aptly.publish_update(':.', suite.name)


async def update_aptly_loop(config, aptly_url):
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
                    ':.', suite.name, not_automatic=True, distribution=suite.name,
                    architectures=['all', 'amd64'])
            except AptlyError as e:
                # 400 indicates it's already published
                if e.status != 400:
                    raise
        for suite_name in existing_by_name:
            await aptly.repos_delete(suite_name)

        while True:
            await update_aptly(config, aptly)
            await asyncio.sleep(30 * 60)


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

    args = parser.parse_args()

    with open(args.config, 'r') as f:
        config = read_config(f)

    loop = asyncio.get_event_loop()
    loop.run_until_complete(asyncio.gather(
        loop.create_task(run_web_server(
            args.listen_address, args.port, args.archive)),
        loop.create_task(update_archive_loop(config, args.archive)),
        loop.create_task(update_aptly_loop(config, args.aptly_url))))
    loop.run_forever()


if __name__ == '__main__':
    sys.exit(main(sys.argv))
