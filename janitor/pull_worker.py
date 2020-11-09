#!/usr/bin/python3
# Copyright (C) 2020 Jelmer Vernooij <jelmer@jelmer.uk>
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

import argparse
import asyncio
from aiohttp import ClientSession, MultipartWriter, BasicAuth, ClientTimeout
from contextlib import contextmanager, ExitStack
from datetime import datetime
import functools
from http.client import IncompleteRead
from io import BytesIO
import json
import os
import socket
import subprocess
import sys
from tempfile import TemporaryDirectory
from typing import Any, Optional, List, Dict
from urllib.parse import urljoin
import yarl

from breezy import urlutils
from breezy.branch import Branch
from breezy.config import (
    credential_store_registry,
    GlobalStack,
    PlainTextCredentialStore,
    )
from breezy.errors import (
    NotBranchError,
    InvalidHttpResponse,
    )
from breezy.controldir import ControlDir
from breezy.transport import Transport

from silver_platter.proposal import enable_tag_pushing

from janitor.trace import note
from janitor.worker import (
    WorkerFailure,
    process_package,
    DEFAULT_BUILD_COMMAND,
    )


DEFAULT_UPLOAD_TIMEOUT = ClientTimeout(30 * 60)


class ResultUploadFailure(Exception):

    def __init__(self, reason: str) -> None:
        self.reason = reason


async def abort_run(
        session: ClientSession,
        base_url: str, run_id: str) -> None:
    finish_url = urljoin(base_url, 'active-runs/%s/abort' % run_id)
    async with session.post(finish_url) as resp:
        if resp.status not in (201, 200):
            raise Exception('Unable to abort run: %r: %d' % (
                await resp.text(), resp.status))


@contextmanager
def bundle_results(metadata: Any, directory: str):
    with ExitStack() as es:
        with MultipartWriter('form-data') as mpwriter:
            mpwriter.append_json(metadata, headers=[
                ('Content-Disposition', 'attachment; filename="result.json"; '
                    'filename*=utf-8\'\'result.json')])  # type: ignore
            for entry in os.scandir(directory):
                if entry.is_file():
                    f = open(entry.path, 'rb')
                    es.enter_context(f)
                    mpwriter.append(BytesIO(f.read()), headers=[
                        ('Content-Disposition', 'attachment; filename="%s"; '
                            'filename*=utf-8\'\'%s' %
                            (entry.name, entry.name))])  # type: ignore
        yield mpwriter


async def upload_results(
        session: ClientSession,
        base_url: str, run_id: str, metadata: Any,
        output_directory: str) -> Any:
    with bundle_results(metadata, output_directory) as mpwriter:
        finish_url = urljoin(
            base_url, 'active-runs/%s/finish' % run_id)
        async with session.post(
                finish_url, data=mpwriter,
                timeout=DEFAULT_UPLOAD_TIMEOUT) as resp:
            if resp.status == 404:
                resp_json = await resp.json()
                raise ResultUploadFailure(resp_json['reason'])
            if resp.status not in (201, 200):
                raise ResultUploadFailure(
                    'Unable to submit result: %r: %d' % (
                        await resp.text(), resp.status))
            return await resp.json()


@contextmanager
def copy_output(output_log: str):
    old_stdout = os.dup(sys.stdout.fileno())
    old_stderr = os.dup(sys.stderr.fileno())
    p = subprocess.Popen(['tee', output_log], stdin=subprocess.PIPE)
    os.dup2(p.stdin.fileno(), sys.stdout.fileno())  # type: ignore
    os.dup2(p.stdin.fileno(), sys.stderr.fileno())  # type: ignore
    yield
    sys.stdout.flush()
    sys.stderr.flush()
    os.dup2(old_stdout, sys.stdout.fileno())
    os.dup2(old_stderr, sys.stderr.fileno())
    p.stdin.close()  # type: ignore


def push_branch(
        source_branch: Branch,
        url: str, vcs_type: str,
        overwrite=False, stop_revision=None,
        possible_transports: Optional[List[Transport]] = None) -> None:
    url, params = urlutils.split_segment_parameters(url)
    branch_name = params.get('branch')
    if branch_name is not None:
        branch_name = urlutils.unquote(branch_name)
    try:
        target = ControlDir.open(
            url, possible_transports=possible_transports)
    except NotBranchError:
        target = ControlDir.create(
            url, format=vcs_type,
            possible_transports=possible_transports)

    target.push_branch(
        source_branch, revision_id=stop_revision,
        overwrite=overwrite, name=branch_name)


def run_worker(branch_url, subpath, vcs_type, env,
               command, output_directory, metadata,
               build_command=None,
               pre_check_command=None,
               post_check_command=None,
               resume_branch_url=None,
               cached_branch_url=None,
               last_build_version=None,
               resume_subworker_result=None,
               result_branch_url=None,
               possible_transports=None):
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    with copy_output(os.path.join(output_directory, 'worker.log')):
        with process_package(
               branch_url, subpath, env,
               command, output_directory, metadata,
               build_command=build_command,
               pre_check_command=pre_check_command,
               post_check_command=post_check_command,
               resume_branch_url=resume_branch_url,
               cached_branch_url=cached_branch_url,
               last_build_version=last_build_version,
               resume_subworker_result=resume_subworker_result,
               possible_transports=possible_transports) as (ws, result):
            enable_tag_pushing(ws.local_tree.branch)
            note('Pushing result branch to %s', result_branch_url)

            try:
                push_branch(
                    ws.local_tree.branch,
                    result_branch_url, overwrite=True,
                    vcs_type=vcs_type.lower(),
                    possible_transports=possible_transports)
            except (InvalidHttpResponse, IncompleteRead) as e:
                # TODO(jelmer): Retry if this was a server error (5xx) of
                # some  sort?
                raise WorkerFailure(
                    'result-push-failed',
                    "Failed to push result branch: %s" % e)
            note('Pushing packaging branch cache to %s',
                 cached_branch_url)
            push_branch(
                ws.local_tree.branch,
                cached_branch_url, vcs_type=vcs_type.lower(),
                possible_transports=possible_transports,
                stop_revision=ws.main_branch.last_revision(),
                overwrite=True)
            return result


async def get_assignment(
        session: ClientSession, base_url: str, node_name: str,
        jenkins_metadata: Optional[Dict[str, str]]) -> Any:
    assign_url = urljoin(base_url, 'active-runs')
    build_arch = subprocess.check_output(
        ['dpkg-architecture', '-qDEB_BUILD_ARCH']).decode()
    json: Any = {'node': node_name, 'archs': [build_arch]}
    if jenkins_metadata:
        json['jenkins'] = jenkins_metadata
    async with session.post(assign_url, json=json) as resp:
        if resp.status != 201:
            raise ValueError('Unable to get assignment: %r' %
                             await resp.text())
        return await resp.json()


async def send_keepalives(ws):
    while True:
        await asyncio.sleep(60)
        await ws.send_bytes(b'keepalive')


async def forward_logs(ws, directory):
    import aionotify
    offsets = {}
    watcher = aionotify.Watcher()
    watcher.watch(path=directory, flags=aionotify.Flags.CREATE)
    await watcher.setup(asyncio.get_event_loop())
    try:
        while True:
            event = await watcher.get_event()
            if (event.flags & aionotify.Flags.CREATE and
                    event.name.endswith('.log')):
                path = os.path.join(directory, event.name)
                watcher.watch(path, flags=aionotify.Flags.MODIFY)
                offsets[path] = 0
            elif (event.alias.endswith('.log') and
                    event.flags & aionotify.Flags.MODIFY):
                path = event.alias
            else:
                continue
            with open(path, 'rb') as f:
                f.seek(offsets[path])
                data = f.read()
            offsets[path] += len(data)
            await ws.send_bytes(
                b'\0'.join([
                    b'log',
                    os.path.basename(path).encode('utf-8'),
                    data]))
    finally:
        watcher.close()


async def main(argv=None):
    parser = argparse.ArgumentParser(
        prog='janitor-pull-worker',
        formatter_class=argparse.ArgumentDefaultsHelpFormatter)
    parser.add_argument(
        '--base-url', type=str, help='Base URL',
        default='https://janitor.debian.net/api/')
    parser.add_argument(
        '--output-directory', type=str,
        help='Output directory', default='.')
    parser.add_argument(
        '--pre-check',
        help='Command to run to check whether to process package.',
        type=str)
    parser.add_argument(
        '--post-check',
        help='Command to run to check package before pushing.',
        type=str, default=None)
    parser.add_argument(
        '--build-command',
        help='Build package to verify it.', type=str,
        default=DEFAULT_BUILD_COMMAND)
    parser.add_argument(
        '--credentials',
        help='Path to credentials file (JSON).', type=str,
        default=None)
    parser.add_argument(
        '--debug',
        help='Print out API communication', action='store_true',
        default=False)

    args = parser.parse_args(argv)

    global_config = GlobalStack()
    global_config.set('branch.fetch_tags', True)

    base_url = yarl.URL(args.base_url)

    auth = BasicAuth.from_url(base_url)
    if args.credentials:
        with open(args.credentials) as f:
            creds = json.load(f)
        auth = BasicAuth(login=creds['login'], password=creds['password'])

        class WorkerCredentialStore(PlainTextCredentialStore):

            def get_credentials(
                    self, protocol, host, port=None, user=None, path=None,
                    realm=None):
                if host == base_url.host:
                    return {
                        'user': creds['login'],
                        'password': creds['password'],
                        'protocol': protocol,
                        'port': port,
                        'host': host,
                        'realm': realm,
                        'verify_certificates': True,
                        }
                return None

        credential_store_registry.register(
            'janitor-worker', WorkerCredentialStore, fallback=True)

    if 'JENKINS_URL' in os.environ:
        jenkins_metadata = {
            'build_url': os.environ.get('BUILD_URL'),
            'executor_number': os.environ.get('EXECUTOR_NUMBER'),
            'build_id': os.environ.get('BUILD_ID'),
            'build_number': os.environ.get('BUILD_NUMBER'),
            }
    else:
        jenkins_metadata = None

    node_name = os.environ.get('NODE_NAME')
    if not node_name:
        node_name = socket.gethostname()

    async with ClientSession(auth=auth) as session:
        assignment = await get_assignment(
            session, args.base_url, node_name,
            jenkins_metadata=jenkins_metadata)
        if args.debug:
            print(assignment)

        ws_url = urljoin(
            args.base_url, 'active-runs/%s/progress' % assignment['id'])
        ws = await session.ws_connect(ws_url)

        watchdog_petter = asyncio.create_task(send_keepalives(ws))

        if 'WORKSPACE' in os.environ:
            desc_path = os.path.join(
                os.environ['WORKSPACE'], 'description.txt')
            with open(desc_path, 'w') as f:
                f.write(assignment['description'])

        branch_url = assignment['branch']['url']
        vcs_type = assignment['branch']['vcs_type']
        result_branch_url = assignment['result_branch']['url']
        if result_branch_url is not None:
            result_branch_url = result_branch_url.rstrip('/')
        subpath = assignment['branch'].get('subpath', '') or ''
        if assignment['resume']:
            resume_result = assignment['resume'].get('result')
            resume_branch_url = assignment['resume']['branch_url'].rstrip('/')
        else:
            resume_result = None
            resume_branch_url = None
        last_build_version = assignment.get('last_build_version')
        cached_branch_url = assignment['branch'].get('cached_url')
        command = assignment['command']
        build_environment = assignment['build'].get('environment', {})

        possible_transports = []

        os.environ.update(assignment['env'])
        os.environ.update(build_environment)

        metadata = {}
        if jenkins_metadata:
            metadata['jenkins'] = jenkins_metadata

        with TemporaryDirectory() as output_directory:
            loop = asyncio.get_running_loop()
            try:
                import aionotify  # noqa: F401
            except ImportError:
                log_forwarder = None
            else:
                log_forwarder = asyncio.create_task(
                    forward_logs(ws, output_directory))

            metadata = {}
            start_time = datetime.now()
            metadata['start_time'] = start_time.isoformat()
            try:
                result = await loop.run_in_executor(None, functools.partial(
                    run_worker, branch_url, subpath, vcs_type, os.environ,
                    command, output_directory, metadata,
                    build_command=args.build_command,
                    pre_check_command=args.pre_check,
                    post_check_command=args.post_check,
                    resume_branch_url=resume_branch_url,
                    cached_branch_url=cached_branch_url,
                    last_build_version=last_build_version,
                    resume_subworker_result=resume_result,
                    result_branch_url=result_branch_url,
                    possible_transports=possible_transports))
            except WorkerFailure as e:
                metadata['code'] = e.code
                metadata['description'] = e.description
                note('Worker failed (%s): %s', e.code, e.description)
                # This is a failure for the worker, but returning 0 will cause
                # jenkins to mark the job having failed, which is not really
                # true.  We're happy if we get to successfully POST to /finish
                return 0
            except BaseException as e:
                metadata['code'] = 'worker-exception'
                metadata['description'] = str(e)
                raise
            else:
                metadata['code'] = None
                metadata['value'] = result.value
                metadata['description'] = result.description
                note('%s', result.description)

                return 0
            finally:
                finish_time = datetime.now()
                note('Elapsed time: %s', finish_time - start_time)

                try:
                    result = await upload_results(
                        session, args.base_url, assignment['id'], metadata,
                        output_directory)
                except ResultUploadFailure as e:
                    sys.stderr.write(str(e))
                    sys.exit(1)

                watchdog_petter.cancel()
                if log_forwarder is not None:
                    log_forwarder.cancel()
                if args.debug:
                    print(result)


if __name__ == '__main__':
    sys.exit(asyncio.run(main()))
