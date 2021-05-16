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
import logging
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))

import silver_platter  # noqa: E402, F401
from buildlog_consultant.common import find_build_failure_description  # noqa: E402
from buildlog_consultant.sbuild import worker_failure_from_sbuild_log  # noqa: E402
from janitor import state  # noqa: E402
from janitor.config import read_config  # noqa: E402
from janitor.logs import get_log_manager  # noqa: E402
from janitor.schedule import do_schedule  # noqa: E402
from janitor.reprocess_logs import reprocess_run_logs  # noqa: E402


loop = asyncio.get_event_loop()

parser = argparse.ArgumentParser()
parser.add_argument(
    "--config", type=str, default="janitor.conf", help="Path to configuration."
)
parser.add_argument(
    "--log-timeout",
    type=int,
    default=60,
    help="Default timeout when retrieving log files.",
)
parser.add_argument(
    "-r", "--run-id", type=str, action="append", help="Run id to process"
)
parser.add_argument(
    '--reschedule', action='store_true',
    help='Schedule rebuilds for runs for which result code has changed.')
parser.add_argument('--dry-run', action='store_true')
args = parser.parse_args()

logging.basicConfig(level=logging.INFO, format='%(message)s')


with open(args.config, "r") as f:
    config = read_config(f)


logfile_manager = get_log_manager(config.logs_location)


async def process_all_build_failures(db, dry_run=False, reschedule=False):
    todo = []
    async with db.acquire() as conn, conn.transaction():
        query = """
SELECT
  package,
  suite,
  id,
  command,
  finish_time - start_time,
  result_code,
  description,
  failure_details
FROM run
WHERE
  (result_code = 'build-failed' OR
   result_code LIKE 'build-failed-stage-%' OR
   result_code LIKE 'autopkgtest-%' OR
   result_code LIKE 'build-%' OR
   result_code LIKE 'dist-%' OR
   result_code LIKE 'unpack-%s' OR
   result_code LIKE 'create-session-%' OR
   result_code LIKE 'missing-%')
   """
        async for package, suite, log_id, command, duration, result_code, description, failure_details in (conn.cursor(query)):
            todo.append(reprocess_run_logs(db, logfile_manager, package, suite, log_id, command, duration, result_code, description, failure_details, dry_run=dry_run, reschedule=reschedule, log_timeout=args.log_timeout))
    for i in range(0, len(todo), 100):
        await asyncio.wait(set(todo[i : i + 100]))


async def process_builds(db, run_ids, dry_run=False, reschedule=False):
    todo = []
    async with db.acquire() as conn:
        query = """
SELECT
  package,
  suite,
  id,
  command,
  finish_time - start_time,
  result_code,
  description,
  failure_details
FROM run
WHERE
  id = ANY($1::text[])
"""
        for package, suite, log_id, command, duration, result_code, description, failure_details in await conn.fetch(
            query, run_ids
        ):
            todo.append(reprocess_run_logs(db, logfile_manager, package, suite, log_id, command, duration, result_code, description, failure_details, dry_run=dry_run, reschedule=reschedule, log_timeout=args.log_timeout))
    if todo:
        await asyncio.wait(todo)


db = state.Database(config.database_location)
if args.run_id:
    loop.run_until_complete(process_builds(db, args.run_id, dry_run=args.dry_run, reschedule=args.reschedule))
else:
    loop.run_until_complete(process_all_build_failures(db, dry_run=args.dry_run, reschedule=args.reschedule))
