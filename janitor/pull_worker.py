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
import os
import socket
import subprocess
import sys
from tempfile import TemporaryDirectory
from typing import Any
from urllib.parse import urljoin
import yarl

from janitor.trace import note
from janitor.worker import WorkerFailure, process_package


class ResultUploadFailure(Exception):

    def __init__(self, reason):
        self.reason = reason


async def upload_results(
        session: ClientSession,
        base_url: str, run_id: str, metadata: Any,
        output_directory: str) -> Any:
    with MultipartWriter('mixed') as mpwriter, ExitStack() as es:
        part = mpwriter.append_json(metadata)
        part.set_content_disposition('attachment', filename='result.json')
        for entry in os.scandir(output_directory):
            if entry.is_file():
                f = open(entry.path, 'rb')
                es.enter_context(f)
                mpwriter.append(f)
        finish_url = urljoin(
            base_url, 'active-runs/%s/finish' % run_id)
        async with session.post(finish_url, data=mpwriter) as resp:
            if resp.status == 404:
                json = await resp.json()
                raise ResultUploadFailure(json['reason'])
            if resp.status not in (201, 200):
                raise ResultUploadFailure(
                    'Unable to submit result: %r: %d' % (
                        await resp.read(), resp.status))
            return await resp.json()


@contextmanager
def copy_output(output_log):
    old_stdout = sys.stdout
    old_stderr = sys.stderr
    p = subprocess.Popen(['tee', output_log], stdin=subprocess.PIPE)
    os.dup2(p.stdin.fileno(), sys.stdout.fileno())
    os.dup2(p.stdin.fileno(), sys.stderr.fileno())
    yield
    sys.stdout = old_stdout
    sys.stderr = old_stderr
    p.stdout.close()
    p.std.close()
    p.wait()


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
        '--node-name',
        type=str,
        help='Node name',
        default=socket.gethostname())

    args = parser.parse_args(argv)

    auth = BasicAuth.from_url(yarl.URL(args.base_url))

    async with ClientSession(auth=auth) as session:
        assign_url = urljoin(args.base_url, 'active-runs')
        async with session.post(
                assign_url, json={'node': args.node_name}) as resp:
            if resp.status != 201:
                raise ValueError('Unable to get assignment: %r' %
                                 await resp.read())
            assignment = await resp.json()

        # ws_url = urljoin(
        #  args.base_url, 'active-runs/%s/ws' % assignment['id'])
        # async with session.ws_connect(ws_url) as ws:
        #    # TODO(jelmer): Forward logs to websocket
        #    # TODO(jelmer): Listen for 'abort' message
        #    pass

    branch_url = assignment['branch']['url']
    result_branch_url = assignment['result_branch']['url']  # noqa: F841
    subpath = assignment['branch'].get('subpath', '') or ''
    if assignment['resume']:
        resume_result = assignment['resume'].get('result')
        resume_branch_url = assignment['resume'].get('branch_url')
    else:
        resume_result = None
        resume_branch_url = None
    last_build_version = assignment.get('last_build_version')
    cached_branch_url = assignment['branch'].get('cached_url')
    build_distribution = assignment['build']['distribution']
    build_suffix = assignment['build']['suffix']
    command = assignment['command']
    build_environment = assignment['build'].get('environment', {})

    env = dict(os.environ.items())
    env.update(assignment['env'])
    env.update(build_environment)

    with TemporaryDirectory() as output_directory:
        metadata = {}
        start_time = datetime.now()
        metadata['start_time'] = start_time.isoformat()
        try:
            with copy_output(os.path.join(output_directory, 'worker.log')):
                result = process_package(
                    branch_url, env,
                    command, output_directory, metadata,
                    build_command=args.build_command,
                    pre_check_command=args.pre_check,
                    post_check_command=args.post_check,
                    resume_branch_url=resume_branch_url,
                    cached_branch_url=cached_branch_url,
                    subpath=subpath, build_distribution=build_distribution,
                    build_suffix=build_suffix,
                    last_build_version=last_build_version,
                    resume_subworker_result=resume_result)
        except WorkerFailure as e:
            metadata['code'] = e.code
            metadata['description'] = e.description
            note('Worker failed: %s', e.description)
            return 0
        except BaseException as e:
            metadata['code'] = 'worker-exception'
            metadata['description'] = str(e)
            raise
        else:
            metadata['code'] = None
            metadata['description'] = result.description
            note('%s', result.description)
            if result.changes_filename is not None:
                note('Built %s.', result.changes_filename)

            # TODO(jelmer): Push to target_branch_url
            return 0
        finally:
            finish_time = datetime.now()
            note('Elapsed time: %s', finish_time - start_time)

            # TODO(jelmer): Push to cached_branch_url
            # TODO(jelmer): Push to result_branch_url

            async with ClientSession(auth=auth) as session:
                try:
                    result = await upload_results(
                        session, args.base_url, assignment['id'], metadata,
                        output_directory)
                except ResultUploadFailure as e:
                    sys.stderr.write(str(e))
                    sys.exit(1)
                print(result)


if __name__ == '__main__':
    sys.exit(asyncio.run(main()))
