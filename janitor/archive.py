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

import asyncio
from io import BytesIO
import os
import sys

from aiohttp import web

from . import SUITES
from .prometheus import setup_metrics


async def handle_debdiff(request):
    post = await request.post()

    old_suite = post.get('old_suite', 'unchanged')
    if old_suite not in SUITES:
        return web.Response(
            status=400, text='Invalid old suite %s' % old_suite)

    try:
        new_suite = post['new_suite']
    except KeyError:
        return web.Response(
            status=400, text='Missing argument: new_suite')

    if new_suite not in SUITES:
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

    return web.Response(
        body=await run_debdiff(old_changes_path, new_changes_path),
        content_type='text/diff')


async def run_debdiff(old_changes, new_changes):
    args = ['debdiff', old_changes, new_changes]
    stdout = BytesIO()
    p = await asyncio.create_subprocess_exec(
        *args, stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE)
    stdout, stderr = await p.communicate()
    return stdout


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
    app.router.add_post('/debdiff', handle_debdiff)
    app.router.add_get(
        '/archive'
        '/{suite:' + '|'.join(SUITES) + '}'
        '/{filename}', handle_archive_file)
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()


def main(argv=None):
    import argparse
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
    args = parser.parse_args()
    loop = asyncio.get_event_loop()
    loop.run_until_complete(asyncio.gather(
        loop.create_task(run_web_server(
            args.listen_address, args.port, args.archive)),
        ))
    loop.run_forever()


if __name__ == '__main__':
    sys.exit(main(sys.argv))
