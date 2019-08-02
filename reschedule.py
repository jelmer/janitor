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
        packages[package.name] = package

    async with state.get_connection() as conn:
        results = await conn.fetch("""SELECT * FROM (
SELECT DISTINCT ON (package, suite) package, command, suite, result_code
FROM run) AS f WHERE result_code = $1""", result_code)

        print('%d items to reschedule.' % len(results))

        for result in results:
            print('Rescheduling %s, %s' % (result[0], result[2]))
            await state.add_to_queue(
                packages[result[0]].branch_url,
                result[0], result[1].split(' '), result[2])


asyncio.run(main(args.result_code))
