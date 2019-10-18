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
args = parser.parse_args()
with open(args.config, 'r') as f:
    config = read_config(f)


async def main(db, result_code):
    packages = {}
    async with db.acquire() as conn1, db.acquire() as conn2:
        for package in await state.iter_packages(conn1):
            if package.removed:
                continue
            packages[package.name] = package

        async for (package, suite, command, id, description, start_time,
                   duration, branch_url) in state.iter_last_runs(
                       conn1, result_code):
            if package not in packages:
                continue
            if packages[package].branch_url is None:
                continue
            if (args.description_re and
                    not re.match(args.description_re, description, re.S)):
                continue
            print('Rescheduling %s, %s' % (package, suite))
            await state.add_to_queue(
                conn2, packages[package].branch_url,
                package, command.split(' '), suite,
                estimated_duration=duration, requestor='reschedule',
                refresh=args.refresh)


db = state.Database(config.database_location)
asyncio.run(main(db, args.result_code))
