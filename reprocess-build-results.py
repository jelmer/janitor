#!/usr/bin/python3

# Copyright (C) 2019-2020 Jelmer Vernooij <jelmer@jelmer.uk>
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
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))

import silver_platter  # noqa: E402, F401
from janitor import state  # noqa: E402
from janitor.config import read_config  # noqa: E402
from janitor.logs import get_log_manager  # noqa: E402
from janitor.sbuild_log import worker_failure_from_sbuild_log  # noqa: E402
from janitor.trace import note  # noqa: E402


loop = asyncio.get_event_loop()

parser = argparse.ArgumentParser()
parser.add_argument(
    '--config', type=str, default='janitor.conf',
    help='Path to configuration.')
args = parser.parse_args()


with open(args.config, 'r') as f:
    config = read_config(f)


logfile_manager = get_log_manager(config.logs_location)


async def reprocess_run(db, package, log_id, result_code, description):
    try:
        build_logf = await logfile_manager.get_log(
            package, log_id, 'build.log')
    except FileNotFoundError:
        return
    failure = worker_failure_from_sbuild_log(build_logf)
    if failure.error:
        if failure.stage:
            new_code = '%s-%s' % (failure.stage, failure.error.kind)
        else:
            new_code = failure.error.kind
    elif failure.stage:
        new_code = 'build-failed-stage-%s' % failure.stage
    else:
        new_code = 'build-failed'
    if new_code != result_code or description != failure.description:
        async with db.acquire() as conn:
            await state.update_run_result(
                conn, log_id, new_code, failure.description)
        note('Updated %r, %r => %r, %r %r', result_code, description,
             new_code, failure.description, failure.context)


async def process_all_build_failures(db):
    todo = []
    async with db.acquire() as conn:
        async for package, log_id, result_code, description in (
                state.iter_build_failures(conn)):
            todo.append(
                reprocess_run(db, package, log_id, result_code, description))
    for i in range(0, len(todo), 100):
        await asyncio.wait(set(todo[i:i+100]))


db = state.Database(config.database_location)
loop.run_until_complete(process_all_build_failures(db))
