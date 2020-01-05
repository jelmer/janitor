#!/usr/bin/python3

import argparse
import asyncio
import re
from janitor import state
from janitor.config import read_config

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
args = parser.parse_args()
with open(args.config, 'r') as f:
    config = read_config(f)


async def main(db, result_code, rejected):
    packages = {}
    async with db.acquire() as conn1, db.acquire() as conn2:
        for package in await state.iter_packages(conn1):
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
            if (args.description_re and
                    not re.match(args.description_re, run.description, re.S)):
                continue
            print('Rescheduling %s, %s' % (run.package, run.suite))
            await state.add_to_queue(
                conn2, run.package, run.command.split(' '), run.suite,
                estimated_duration=run.duration, requestor='reschedule',
                refresh=args.refresh, offset=args.offset)


db = state.Database(config.database_location)
asyncio.run(main(db, args.result_code, args.rejected))
