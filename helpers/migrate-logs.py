#!/usr/bin/python3

import argparse
import asyncio
import os
import tempfile

from ognibuild.debian.build import BUILD_LOG_FILENAME

import silver_platter  # noqa: E402, F401
from janitor import state  # noqa: E402
from janitor.config import read_config  # noqa: E402
from janitor.logs import get_log_manager  # noqa: E402


loop = asyncio.get_event_loop()

parser = argparse.ArgumentParser()
parser.add_argument(
    '--config', type=str, default='janitor.conf',
    help='Path to configuration.')
parser.add_argument('from_location', type=str, nargs=1)
parser.add_argument('to_location', type=str, nargs=1)
args = parser.parse_args()

with open(args.config) as f:
    config = read_config(f)

from_manager = get_log_manager(args.from_location)
to_manager = get_log_manager(args.to_location)


async def reprocess_run(pool, package, log_id, logfilenames):
    if logfilenames is None:
        logfilenames = []
        if await from_manager.has_log(package, log_id, 'worker.log'):
            logfilenames.append('worker.log')
        if await from_manager.has_log(package, log_id, BUILD_LOG_FILENAME):
            logfilenames.append(BUILD_LOG_FILENAME)
        i = 1
        while await from_manager.has_log(
                package, log_id, '%s.%d' % (BUILD_LOG_FILENAME, i)):
            log_name = '%s.%d' % (BUILD_LOG_FILENAME, i)
            logfilenames.append(log_name)
            i += 1

        async with pool.acquire() as conn:
            await conn.execute(
                'UPDATE run SET logfilenames = $1 WHERE id = $2', logfilenames,
                log_id)

    print('Processing {} ({!r})'.format(log_id, logfilenames))
    with tempfile.TemporaryDirectory() as d:
        for name in logfilenames:
            try:
                log = await from_manager.get_log(package, log_id, name)
            except FileNotFoundError:
                continue
            path = os.path.join(d, name)
            with open(path, 'wb') as f:
                f.write(log.read())
            await to_manager.import_log(package, log_id, path)
            await from_manager.delete_log(package, log_id, name)


async def process_all_build_failures(db_location):
    todo = []
    async with state.create_pool(db_location) as pool, pool.acquire() as conn:
        async with conn.transaction():
            async for row in conn.cursor(
                    "SELECT package, id, logfilenames FROM run"):
                todo.append(reprocess_run(pool, row[0], row[1], row[2]))
    for i in range(0, len(todo), 100):
        await asyncio.gather(*todo[i:i + 100])


loop.run_until_complete(process_all_build_failures(config.database_location))
