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
from buildlog_consultant.sbuild import worker_failure_from_sbuild_log  # noqa: E402
from janitor import state  # noqa: E402
from janitor.config import read_config  # noqa: E402
from janitor.logs import get_log_manager  # noqa: E402
from janitor.trace import note  # noqa: E402


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
args = parser.parse_args()


with open(args.config, "r") as f:
    config = read_config(f)


logfile_manager = get_log_manager(config.logs_location)


async def reprocess_run(db, package, log_id, result_code, description):
    try:
        build_logf = await logfile_manager.get_log(
            package, log_id, "build.log", timeout=args.log_timeout
        )
    except FileNotFoundError:
        return
    failure = worker_failure_from_sbuild_log(build_logf)
    if failure.error:
        if failure.stage and not failure.error.is_global:
            new_code = "%s-%s" % (failure.stage, failure.error.kind)
        else:
            new_code = failure.error.kind
    elif failure.stage:
        new_code = "build-failed-stage-%s" % failure.stage
    else:
        new_code = "build-failed"
    if new_code != result_code or description != failure.description:
        async with db.acquire() as conn:
            await state.update_run_result(conn, log_id, new_code, failure.description)
        note(
            "%s/%s: Updated %r, %r => %r, %r %r",
            package,
            log_id,
            result_code,
            description,
            new_code,
            failure.description,
            failure.context,
        )


async def process_all_build_failures(db):
    todo = []
    async with db.acquire() as conn, conn.transaction():
        query = """
SELECT
  package,
  id,
  result_code,
  description
FROM run
WHERE
  (result_code = 'build-failed' OR
   result_code LIKE 'build-failed-stage-%' OR
   result_code LIKE 'autopkgtest-%' OR
   result_code LIKE 'build-%' OR
   result_code LIKE 'create-session-%')
   """
        async for package, log_id, result_code, description in (conn.cursor(query)):
            todo.append(reprocess_run(db, package, log_id, result_code, description))
    for i in range(0, len(todo), 100):
        await asyncio.wait(set(todo[i : i + 100]))


async def process_builds(db, run_ids):
    todo = []
    async with db.acquire() as conn:
        query = """
SELECT
  package,
  id,
  result_code,
  description
FROM run
WHERE
  id = ANY($1::text[])
"""
        for package, log_id, result_code, description in await conn.fetch(
            query, run_ids
        ):
            todo.append(reprocess_run(db, package, log_id, result_code, description))
    if todo:
        await asyncio.wait(todo)


db = state.Database(config.database_location)
if args.run_id:
    loop.run_until_complete(process_builds(db, args.run_id))
else:
    loop.run_until_complete(process_all_build_failures(db))
