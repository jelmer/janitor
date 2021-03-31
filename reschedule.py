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
from datetime import datetime, timedelta
import re
from janitor import state
from janitor.config import read_config
from janitor.schedule import do_schedule

parser = argparse.ArgumentParser("reschedule")
parser.add_argument("result_code", type=str)
parser.add_argument("description_re", type=str, nargs="?")
parser.add_argument(
    "--config", type=str, default="janitor.conf", help="Path to configuration."
)
parser.add_argument("--refresh", action="store_true", help="Force run from scratch.")
parser.add_argument("--offset", type=int, default=0, help="Schedule offset.")
parser.add_argument(
    "--rejected", action="store_true", help="Process rejected runs only."
)
parser.add_argument("--suite", type=str, help="Suite to process.")
parser.add_argument(
    "--min-age", type=int, default=0, help="Only reschedule runs older than N days."
)
args = parser.parse_args()
with open(args.config, "r") as f:
    config = read_config(f)


async def main(db, result_code, suite, description_re, rejected, min_age=0):
    async with db.acquire() as conn1:
        query = """
SELECT
  package,
  suite,
  command,
  finish_time - start_time AS duration
FROM last_runs
WHERE
    branch_url IS NOT NULL AND
    package IN (SELECT name FROM package WHERE NOT removed) AND
"""
        where = []
        params = []
        if result_code is not None:
            params.append(result_code)
            where.append("result_code = $%d" % len(params))
        if suite:
            params.append(suite)
            where.append("suite = $%d" % len(params))
        if rejected:
            where.append("run.review_status = 'rejected'")
        if description_re:
            params.append(description_re)
            where.append("description ~ $%d" % len(params))
        if min_age:
            params.append(datetime.utcnow() - timedelta(days=min_age))
            where.append("finish_time < $%d" % len(params))
        query += " AND ".join(where)
        for run in await conn1.fetch(query, *params):
            print("Rescheduling %s, %s" % (run['package'], run['suite']))
            await do_schedule(
                conn1,
                run['package'],
                run['suite'],
                command=run['command'],
                estimated_duration=run['duration'],
                requestor="reschedule",
                refresh=args.refresh,
                offset=args.offset,
                bucket="reschedule",
            )


db = state.Database(config.database_location)
asyncio.run(main(db, args.result_code, args.suite, args.description_re, args.rejected, args.min_age))
