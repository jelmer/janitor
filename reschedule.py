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
from janitor.debian import state as debian_state
from janitor.config import read_config
from janitor.schedule import do_schedule

parser = argparse.ArgumentParser('reschedule')
parser.add_argument('result_code', type=str)
parser.add_argument('description_re', type=str, nargs='?')
parser.add_argument(
    '--config', type=str, default='janitor.conf',
    help='Path to configuration.')
parser.add_argument(
    '--refresh', action='store_true',
    help='Force run from scratch.')
parser.add_argument(
    '--offset', type=int, default=0,
    help='Schedule offset.')
parser.add_argument(
    '--rejected', action='store_true',
    help='Process rejected runs only.')
parser.add_argument(
    '--suite', type=str,
    help='Suite to process.')
parser.add_argument(
    '--min-age', type=int, default=0,
    help='Only reschedule runs older than N days.')
args = parser.parse_args()
with open(args.config, 'r') as f:
    config = read_config(f)


async def main(db, result_code, rejected, min_age=0):
    packages = {}
    async with db.acquire() as conn1, db.acquire() as conn2:
        for package in await debian_state.iter_packages(conn1):
            if package.removed:
                continue
            packages[package.name] = package

        async for run in state.iter_last_runs(
                conn1, result_code, suite=args.suite):
            if run.package not in packages:
                continue
            if rejected and run.review_status != 'rejected':
                continue
            if packages[run.package].branch_url is None:
                continue
            if datetime.now() < run.times[1] + timedelta(days=min_age):
                continue
            if (args.description_re and
                    not re.match(args.description_re, run.description, re.S)):
                continue
            print('Rescheduling %s, %s' % (run.package, run.suite))
            await do_schedule(
                conn2, run.package, run.suite, command=run.command.split(' '),
                estimated_duration=run.duration, requestor='reschedule',
                refresh=args.refresh, offset=args.offset,
                bucket='reschedule')


db = state.Database(config.database_location)
asyncio.run(main(db, args.result_code, args.rejected, args.min_age))
