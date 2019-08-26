#!/usr/bin/python3

import argparse
import asyncio
import re
from janitor import state

parser = argparse.ArgumentParser('reschedule')
parser.add_argument('result_code', type=str)
parser.add_argument('description_re', type=str, nargs='?')
args = parser.parse_args()


async def main(result_code):
    packages = {}
    for package in await state.iter_packages():
        if package.removed:
            continue
        packages[package.name] = package

    async for (package, suite, command, id, description, start_time,
            duration, branch_url) in state.iter_last_runs(result_code):
        if package not in packages:
            continue
        if packages[package].branch_url is None:
            continue
        if (args.description_re and
                not re.match(args.description_re, description, re.S)):
            continue
        print('Rescheduling %s, %s' % (package, suite))
        await state.add_to_queue(
            packages[package].branch_url,
            package, command.split(' '), suite,
            estimated_duration=duration)


asyncio.run(main(args.result_code))
