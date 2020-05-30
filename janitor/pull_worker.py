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
from aiohttp import ClientSession, MultipartWriter, BasicAuth
from contextlib import contextmanager, ExitStack
from datetime import datetime
from io import BytesIO
import json
import os
import socket
import subprocess
import sys
from tempfile import TemporaryDirectory
from typing import Any, Optional, List
from urllib.parse import urljoin
import yarl

from breezy import urlutils
from breezy.branch import Branch
from breezy.config import credential_store_registry, PlainTextCredentialStore
from breezy.errors import NotBranchError
from breezy.controldir import ControlDir
from breezy.transport import Transport

from silver_platter.proposal import enable_tag_pushing

from janitor.trace import note
from janitor.worker import (
    WorkerFailure,
    process_package,
    DEFAULT_BUILD_COMMAND,
    )


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
                await resp.read(), resp.status))


async def upload_results(
        session: ClientSession,
        base_url: str, run_id: str, metadata: Any,
        output_directory: str) -> Any:
    with ExitStack() as es:
        with MultipartWriter('form-data') as mpwriter:
            part = mpwriter.append(BytesIO(
                json.dumps(metadata).encode('utf-8')))
            part.set_content_disposition('attachment', filename='result.json')
            for entry in os.scandir(output_directory):
                if entry.is_file():
                    f = open(entry.path, 'rb')
                    es.enter_context(f)
                    part = mpwriter.append(BytesIO(f.read()))
                    part.set_content_disposition(
                        'attachment', filename=entry.name)
        finish_url = urljoin(
            base_url, 'active-runs/%s/finish' % run_id)
        async with session.post(finish_url, data=mpwriter) as resp:
            if resp.status == 404:
                resp_json = await resp.json()
                raise ResultUploadFailure(resp_json['reason'])
            if resp.status not in (201, 200):
                raise ResultUploadFailure(
                    'Unable to submit result: %r: %d' % (
                        await resp.read(), resp.status))
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


async def get_assignment(
        session: ClientSession, base_url: str, node_name: str) -> Any:
    assign_url = urljoin(base_url, 'active-runs')
    build_arch = subprocess.check_output(
        ['dpkg-architecture', '-qDEB_BUILD_ARCH']).decode()
    async with session.post(
            assign_url, json={'node': node_name, 'archs': [build_arch]}
            ) as resp:
        if resp.status != 201:
            raise ValueError('Unable to get assignment: %r' %
                             await resp.read())
        return await resp.json()


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

    node_name = os.environ.get('NODE_NAME')
    if not node_name:
        node_name = socket.gethostname()

    async with ClientSession(auth=auth) as session:
        assignment = await get_assignment(session, args.base_url, node_name)
        if args.debug:
            print(assignment)

        # ws_url = urljoin(
        #  args.base_url, 'active-runs/%s/ws' % assignment['id'])
        # async with session.ws_connect(ws_url) as ws:
        #    # TODO(jelmer): Forward logs to websocket
        #    # TODO(jelmer): Listen for 'abort' message
        #    pass

    if 'WORKSPACE' in os.environ:
        desc_path = os.path.join(os.environ['WORKSPACE'], 'description.txt')
        with open(desc_path, 'w') as f:
            f.write(assignment['description'])

    branch_url = assignment['branch']['url']
    vcs_type = assignment['branch']['vcs_type']
    result_branch_url = assignment['result_branch']['url'].rstrip('/')
    subpath = assignment['branch'].get('subpath', '') or ''
    if assignment['resume']:
        resume_result = assignment['resume'].get('result')
        resume_branch_url = assignment['resume']['branch_url'].rstrip('/')
    else:
        resume_result = None
        resume_branch_url = None
    last_build_version = assignment.get('last_build_version')
    cached_branch_url = assignment['branch'].get('cached_url')
    build_distribution = assignment['build']['distribution']
    build_suffix = assignment['build']['suffix']
    command = assignment['command']
    build_environment = assignment['build'].get('environment', {})

    possible_transports = []

    env = dict(os.environ.items())
    env.update(assignment['env'])
    env.update(build_environment)

    with TemporaryDirectory() as output_directory:
        metadata = {}
        start_time = datetime.now()
        metadata['start_time'] = start_time.isoformat()
        try:
            with copy_output(os.path.join(output_directory, 'worker.log')):
                with process_package(
                        branch_url, subpath, env,
                        command, output_directory, metadata,
                        build_command=args.build_command,
                        pre_check_command=args.pre_check,
                        post_check_command=args.post_check,
                        resume_branch_url=resume_branch_url,
                        cached_branch_url=cached_branch_url,
                        build_distribution=build_distribution,
                        build_suffix=build_suffix,
                        last_build_version=last_build_version,
                        resume_subworker_result=resume_result,
                        possible_transports=possible_transports
                        ) as (ws, result):
                    enable_tag_pushing(ws.local_tree.branch)
                    note('Pushing result branch to %s', result_branch_url)
                    push_branch(
                        ws.local_tree.branch,
                        result_branch_url, overwrite=True,
                        vcs_type=vcs_type.lower(),
                        possible_transports=possible_transports)
                    note('Pushing packaging branch cache to %s',
                         cached_branch_url)
                    push_branch(
                        ws.local_tree.branch,
                        cached_branch_url, vcs_type=vcs_type.lower(),
                        possible_transports=possible_transports,
                        stop_revision=ws.main_branch.last_revision(),
                        overwrite=True)
        except WorkerFailure as e:
            metadata['code'] = e.code
            metadata['description'] = e.description
            note('Worker failed (%s): %s', e.code, e.description)
            return 1
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

            async with ClientSession(auth=auth) as session:
                try:
                    result = await upload_results(
                        session, args.base_url, assignment['id'], metadata,
                        output_directory)
                except ResultUploadFailure as e:
                    sys.stderr.write(str(e))
                    sys.exit(1)
                if args.debug:
                    print(result)


if __name__ == '__main__':
    sys.exit(asyncio.run(main()))
