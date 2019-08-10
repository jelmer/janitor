#!/usr/bin/python3

import argparse
import asyncio
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))

import silver_platter  # noqa: E402, F401
from janitor import state  # noqa: E402
from janitor.logs import get_log_manager  # noqa: E402
from janitor.sbuild_log import worker_failure_from_sbuild_log  # noqa: E402
from janitor.trace import note  # noqa: E402


loop = asyncio.get_event_loop()

parser = argparse.ArgumentParser()
parser.add_argument(
    '--log-dir', help='Directory to store logs in.',
    type=str, default='https://s3.nl-ams.scw.cloud')
args = parser.parse_args()


logfile_manager = get_log_manager(args.log_dir)


async def reprocess_run(package, log_id, result_code, description):
    try:
        build_logf = await logfile_manager.get_log(
            package, log_id, 'build.log')
    except FileNotFoundError:
        return
    failure = worker_failure_from_sbuild_log(build_logf)
    if failure.error:
        new_code = '%s-%s' % (failure.stage, failure.error.kind)
    elif failure.stage:
        new_code = 'build-failed-stage-%s' % failure.stage
    else:
        new_code = 'build-failed'
    if new_code != result_code or description != failure.description:
        await state.update_run_result(log_id, new_code, failure.description)
        note('Updated %r, %r => %r, %r', result_code, description,
             new_code, failure.description)


loop.run_until_complete(asyncio.gather(*[
    reprocess_run(package, log_id, result_code, description)
    for package, log_id, result_code, description in loop.run_until_complete(
        state.iter_build_failures())]))
