#!/usr/bin/python3

from janitor import state
from janitor.site import env


async def get_history(limit):
    return [run async for run in state.iter_runs(limit=limit)]


async def write_history(limit=None):
    template = env.get_template('history.html')
    return await template.render_async(
        count=limit,
        history=await get_history(limit))


if __name__ == '__main__':
    import argparse
    import asyncio
    import sys
    parser = argparse.ArgumentParser('report-history')
    parser.add_argument(
        '--limit', type=int,
        help='Number of entries to display',
        default=100)
    args = parser.parse_args()
    loop = asyncio.get_event_loop()
    sys.stdout.write(loop.run_until_complete(write_history(limit=args.limit)))
