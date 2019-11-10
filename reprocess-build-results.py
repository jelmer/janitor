#!/usr/bin/python3

import argparse
import asyncio
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))

import silver_platter  # noqa: E402, F401
from janitor import state  # noqa: E402
from janitor.config import read_config  # noqa: E402
from janitor.logs import get_log_manager  # noqa: E402
from janitor.sbuild_log import worker_failure_from_sbuild_log  # noqa: E402
from janitor.trace import note  # noqa: E402


loop = asyncio.get_event_loop()

parser = argparse.ArgumentParser()
parser.add_argument(
    '--config', type=str, default='janitor.conf',
    help='Path to configuration.')
args = parser.parse_args()


with open(args.config, 'r') as f:
    config = read_config(f)


logfile_manager = get_log_manager(config.logs_location)


async def reprocess_run(db, package, log_id, result_code, description):
    try:
        build_logf = await logfile_manager.get_log(
            package, log_id, 'build.log')
    except FileNotFoundError:
        return
    failure = worker_failure_from_sbuild_log(build_logf)
    if failure.error:
        if failure.stage:
            new_code = '%s-%s' % (failure.stage, failure.error.kind)
        else:
            new_code = failure.error.kind
    elif failure.stage:
        new_code = 'build-failed-stage-%s' % failure.stage
    else:
        new_code = 'build-failed'
    if new_code != result_code or description != failure.description:
        async with db.acquire() as conn:
            await state.update_run_result(
                conn, log_id, new_code, failure.description)
        note('Updated %r, %r => %r, %r', result_code, description,
             new_code, failure.description)


async def process_all_build_failures(db):
    todo = []
    async with db.acquire() as conn:
        async for package, log_id, result_code, description in (
                state.iter_build_failures(conn)):
            todo.append(
                reprocess_run(db, package, log_id, result_code, description))
    for i in range(0, len(todo), 100):
        await asyncio.wait(set(todo[i:i+100]))


db = state.Database(config.database_location)
loop.run_until_complete(process_all_build_failures(db))
