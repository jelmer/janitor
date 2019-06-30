#!/usr/bin/python3

import argparse
import asyncio
import os
import sys

from jinja2 import Environment, FileSystemLoader, select_autoescape

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402
from janitor.site import env, format_duration  # noqa: E402

parser = argparse.ArgumentParser('report-history')
parser.add_argument('--limit', type=int, help='Number of entries to display',
                    default=100)
args = parser.parse_args()

loop = asyncio.get_event_loop()

async def get_history():
    data = []
    async for (run_id, times, command, description, package, proposal_url,
            changes_filename, build_distro, result_code,
            branch_name) in state.iter_runs(limit=args.limit):
        row = [
            package,
            command,
            times[1] - times[0],
            run_id,
            result_code,
            proposal_url,
            ]
        data.append(row)
    return data


async def write_history():
    template = env.get_template('history.html')
    sys.stdout.write(await template.render_async(
        count=args.limit,
        history=await get_history(),
        format_duration=format_duration))


loop.run_until_complete(write_history())
