#!/usr/bin/python3

import argparse
import asyncio
from janitor import state

parser = argparse.ArgumentParser('reschedule')
parser.add_argument('result_code', type=str)
args = parser.parse_args()


async def main(result_code):
    packages = {}
    for package in await state.iter_packages():
        if package.removed:
            continue
        packages[package.name] = package

    results = await state.iter_last_runs(result_code)
    print('%d items to reschedule.' % len(results))

    for (package, suite, command, id, description, start_time,
            duration, branch_url) in results:
        if branch_url is None:
            continue
        print('Rescheduling %s, %s' % (package, suite))
        await state.add_to_queue(
            branch_url,
            package, command.split(' '), suite)


asyncio.run(main(args.result_code))
