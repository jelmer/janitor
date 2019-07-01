#!/usr/bin/python3

import os

from janitor import state
from janitor.site import env, format_duration

async def get_history(limit):
    data = []
    async for (run_id, times, command, description, package, proposal_url,
            changes_filename, build_distro, result_code,
            branch_name) in state.iter_runs(limit=limit):
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


async def write_history(limit=None):
    template = env.get_template('history.html')
    return await template.render_async(
        count=limit,
        history=await get_history(limit),
        format_duration=format_duration)


if __name__ == '__main__':
    import argparse
    import asyncio
    import sys
    parser = argparse.ArgumentParser('report-history')
    parser.add_argument('--limit', type=int, help='Number of entries to display',
                        default=100)
    args = parser.parse_args()
    loop = asyncio.get_event_loop()
    sys.stdout.write(loop.run_until_complete(write_history(limit=args.limit)))
