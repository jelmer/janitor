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


async def reprocess_run(db, package, suite, log_id, command, duration, result_code, description, dry_run=False, reschedule=False):
    if result_code.startswith('dist-'):
        logname = 'dist.log'
    else:
        logname = 'build.log'
    try:
        logf = await logfile_manager.get_log(
            package, log_id, logname, timeout=args.log_timeout
        )
    except FileNotFoundError:
        return

    if logname == 'build.log':
        failure = worker_failure_from_sbuild_log(logf)
        if failure.error:
            if failure.stage and not failure.error.is_global:
                new_code = "%s-%s" % (failure.stage, failure.error.kind)
            else:
                new_code = failure.error.kind
        elif failure.stage:
            new_code = "build-failed-stage-%s" % failure.stage
        else:
            new_code = "build-failed"
        new_description = failure.description
        new_phase = failure.phase,
    elif logname == 'dist.log':
        lines = [line.decode('utf-8', 'replace') for line in logf]
        problem = find_build_failure_description(lines)[1]
        if problem is None:
            if result_code == 'dist-no-tarball':
                new_code = result_code
                new_description = description
            else:
                new_code = 'dist-command-failed'
                new_description = description
        else:
            new_code = 'dist-' + problem.kind
            new_description = str(problem)
        new_phase = None

    if new_code != result_code or description != new_description:
        logging.info(
            "%s/%s: Updated %r, %r => %r, %r %r",
            package,
            log_id,
            result_code,
            description,
            new_code,
            new_description,
            new_phase
        )
        if not dry_run:
            async with db.acquire() as conn:
                await state.update_run_result(conn, log_id, new_code, new_description)
                if reschedule and new_code != result_code:
                    await do_schedule(
                        conn,
                        package,
                        suite,
                        command=command.split(" "),
                        estimated_duration=duration,
                        requestor="reprocess-build-results",
                        bucket="reschedule",
                    )


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
  description
FROM run
WHERE
  (result_code = 'build-failed' OR
   result_code LIKE 'build-failed-stage-%' OR
   result_code LIKE 'autopkgtest-%' OR
   result_code LIKE 'build-%' OR
   result_code LIKE 'dist-%' OR
   result_code LIKE 'create-session-%')
   """
        async for package, suite, log_id, command, duration, result_code, description in (conn.cursor(query)):
            todo.append(reprocess_run(db, package, suite, log_id, command, duration, result_code, description, dry_run=dry_run, reschedule=reschedule))
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
  description
FROM run
WHERE
  id = ANY($1::text[])
"""
        for package, suite, log_id, command, duration, result_code, description in await conn.fetch(
            query, run_ids
        ):
            todo.append(reprocess_run(db, package, suite, log_id, command, duration, result_code, description, dry_run=dry_run, reschedule=reschedule))
    if todo:
        await asyncio.wait(todo)


db = state.Database(config.database_location)
if args.run_id:
    loop.run_until_complete(process_builds(db, args.run_id, dry_run=args.dry_run, reschedule=args.reschedule))
else:
    loop.run_until_complete(process_all_build_failures(db, dry_run=args.dry_run, reschedule=args.reschedule))
