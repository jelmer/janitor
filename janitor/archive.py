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

from aiohttp import ClientSession, UnixConnector, ClientTimeout
from aiohttp.web_middlewares import normalize_path_middleware
import asyncio
import json
import os
import re
import sys
import traceback
from typing import List
from urllib.parse import quote

from aiohttp import web
from debian.deb822 import Changes
from debian.changelog import Changelog
from debian.copyright import parse_multiline

from .config import read_config, Config, Suite
from .prometheus import setup_metrics
from prometheus_client import (
    Gauge,
    )

from .trace import note, warning

suite_check = re.compile('^[a-z0-9-]+$')

last_suite_publish_success = Gauge(
    'last_suite_publish_success',
    'Last time publishing a suite succeeded',
    labelnames=('suite', ))
last_local_publish_success = Gauge(
    'last_local_publish_success',
    'Last time regular publishing to local succeeded')


class UploadError(Exception):

    def __init__(self, failed_files, details):
        self.failed_files = failed_files
        self.details = details


async def handle_upload(request):
    aptly_session = request.app.aptly_session
    directory = request.match_info['directory']
    container = os.path.join(request.app.incoming_dir, directory)
    os.mkdir(container)
    reader = await request.multipart()
    result = {'filenames': []}
    while True:
        part = await reader.next()
        if part is None:
            break
        path = os.path.join(container, part.filename)
        result['filenames'].append(part.filename)
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
    if 'changes_filename' not in result:
        note('No changes file in uploaded directory: %r',
             result['filenames'])
        return web.json_response(result, status=200)
    try:
        report = await upload_directory(aptly_session, container)
    except UploadError as e:
        return web.json_response(
            {'msg': str(e), 'failed_files': e.failed_files},
            status=400)
    result['report'] = report
    note('Uploaded files: %r', result['filenames'])
    return web.json_response(result, status=200)


class AptlyError(Exception):

    def __init__(self, message):
        self.message = message


async def aptly_call(aptly_session, method, path, json=None, params=None,
                     timeout=None):
    async with aptly_session.request(
            method=method, url='http://localhost/api/' + path, json=json,
            params=params, timeout=timeout) as resp:
        if resp.status // 100 != 2:
            raise AptlyError(
                'error %d in /api/%s: %s' % (
                    resp.status, path, await resp.read()))
        return await resp.json()


async def do_publish(
        aptly_session, suite, storage, prefix, label, origin=None):
    publish = {}
    for p in await aptly_call(aptly_session, 'GET', 'publish'):
        publish[(p['Storage'], p['Prefix'], p['Distribution'])] = p

    note('Publishing %s to %s:%s', suite, storage, prefix)

    loc = "%s:%s" % (storage, prefix)
    if (storage, prefix, suite) in publish:
        params = {}
        try:
            await aptly_call(
                aptly_session, 'PUT',
                'publish/%s/%s' % (loc, suite), json=params,
                timeout=ClientTimeout(30 * 60))
        except asyncio.exceptions.TimeoutError:
            raise AptlyError('timeout while publishing %s' % suite)
    else:
        params = {
            'SourceKind': 'local',
            'Sources': [{'Name': suite}],
            'Distribution': suite,
            'Label': label,
            'NotAutomatic': 'yes',
            'ButAutomaticUpgrades': 'yes',
            }
        if origin:
            params['Origin'] = origin

        await aptly_call(
            aptly_session, 'POST', 'publish/%s' % loc, json=params)

    last_suite_publish_success.labels(suite=suite).set_to_current_time()


async def loop_local_publish(aptly_session, config):
    await asyncio.sleep(15 * 60)
    while True:
        for suite in config.suite:
            try:
                await do_publish(
                    aptly_session, suite.name,
                    storage='', prefix='.',
                    label=suite.archive_description,
                    origin=config.origin)
            except AptlyError as e:
                warning('Error while publishing %s: %s',
                        suite.name, e)
        last_local_publish_success.set_to_current_time()
        await asyncio.sleep(30 * 60)


async def handle_publish(request):
    post = await request.post()
    storage = post.get('storage', '')
    prefix = post.get('prefix', '.')
    suites_processed = []
    failed_suites = []

    for suite in request.app.config.suite:
        if post.get('suite') not in (suite.name, None):
            continue
        try:
            await asyncio.shield(do_publish(
                request.app.aptly_session, suite.name, storage, prefix,
                label=suite.archive_description,
                origin=request.app.config.origin))
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
        path = 'files/%s' % dirname
    json = await aptly_call(request.app.aptly_session, 'GET', path)
    return web.json_response(json)


async def handle_update(request):
    # TODO: Find the latest version of each package in db for suite
    # TODO: Find out latest version of each package in aptly for suite
    # TODO: Figure out which packages are outdated in aptly
    # TODO: Upload those packages to aptly
    pass


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
    app.router.add_static('/dists', os.path.join(archive_path, 'dists'))
    app.router.add_static('/pool', os.path.join(archive_path, 'pool'))
    app.router.add_post('/update', handle_update, name='update')
    app.router.add_post('/publish', handle_publish, name='publish')
    app.router.add_get('/pending/', handle_pending, name='pending')
    app.router.add_get('/pending/{subdir}/', handle_pending,
                       name='pending-subdir')
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
        raise UploadError(os.listdir(directory), 'No valid changes file')
    try:
        result = await aptly_call(
            aptly_session, 'POST', 'repos/%s/include/%s/%s' % (
                    suite, quote(os.path.basename(directory)),
                    quote(changes_filename)),
            params={'acceptUnsigned': 1})
    except AptlyError as e:
        raise UploadError(os.listdir(directory), e)
    else:
        failed_files = result['FailedFiles']
        report = result['Report']
        for w in report['Warnings']:
            warning('Aptly warning: %s', w)
        if failed_files:
            raise UploadError(failed_files, report['Warnings'])
        return report


async def process_incoming(aptly_session: ClientSession, incoming_dir: str):
    for entry in os.scandir(incoming_dir):
        if not entry.is_dir():
            # Weird
            continue
        try:
            await upload_directory(aptly_session, entry.path)
        except UploadError as e:
            warning('Failed to upload files (%r) to aptly: %s',
                    e.failed_files, e.details)


async def loop_process_incoming(
        config: Config, aptly_session: ClientSession, incoming_dir: str):
    # Give aptly some time to start
    await asyncio.sleep(25)
    while True:
        await process_incoming(aptly_session, incoming_dir)
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
        loop.create_task(loop_local_publish(aptly_session, config)),
        loop.create_task(loop_process_incoming(
            config, aptly_session, incoming_dir))))
    loop.run_forever()


if __name__ == '__main__':
    sys.exit(main(sys.argv))
