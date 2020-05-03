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
from aiohttp import ClientSession, MultipartWriter
from contextlib import ExitStack
from datetime import datetime
import os
import sys
from tempfile import TemporaryDirectory
from typing import Any
from urllib.parse import urljoin

from janitor.trace import note
from janitor.worker import WorkerFailure, process_package


async def upload_results(
        session: ClientSession,
        base_url: str, run_id: str, metadata: Any,
        output_directory: str) -> Any:
    with MultipartWriter('mixed') as mpwriter, ExitStack() as es:
        mpwriter.append_json(metadata)
        for entry in os.scandir(output_directory):
            if entry.is_file():
                f = open(entry.path, 'rb')
                es.enter_context(f)
                mpwriter.append(f)
        finish_url = urljoin(
            base_url, 'active-runs/%s/finish' % run_id)
        async with session.post(finish_url, data=mpwriter) as resp:
            if resp.status != 201:
                raise ValueError('Unable to submit result: %r' %
                                 await resp.read())
            return await resp.json()


async def main(argv=None):
    import socket
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
        '--worker-name',
        type=str,
        help='Worker name',
        default=socket.gethostname())

    args = parser.parse_args(argv)

    async with ClientSession() as session:
        assign_url = urljoin(args.base_url, 'active-runs')
        async with session.post(assign_url) as resp:
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

    branch_url = assignment['branch_url']
    subpath = assignment['branch'].get('subpath', '') or ''
    if assignment['resume']:
        resume_result = assignment['resume'].get('result')
        resume_branch_url = assignment['resume'].get('branch_url')
    else:
        resume_result = None
        resume_branch_url = None
    last_build_version = assignment.get('last_build_version')
    cached_branch_url = assignment['branch'].get('cached_branch_url')
    build_distribution = assignment['build']['distribution']
    build_suffix = assignment['build']['suffix']
    command = assignment['command']
    build_command = assignment['build']['command']

    env = dict(os.environ.items())
    env.update(assignment['env'])

    with TemporaryDirectory() as output_directory:
        metadata = {}
        start_time = datetime.now()
        metadata['start_time'] = start_time.isoformat()
        try:
            result = process_package(
                branch_url, env,
                command, output_directory, metadata,
                build_command=build_command,
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
            return 0
        finally:
            finish_time = datetime.now()
            note('Elapsed time: %s', finish_time - start_time)

            async with ClientSession() as session:
                result = await upload_results(
                    session, args.base_url, assignment['id'], metadata,
                    output_directory)
                print(result)


if __name__ == '__main__':
    sys.exit(asyncio.run(main()))
