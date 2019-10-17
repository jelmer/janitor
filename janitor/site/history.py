#!/usr/bin/python3

from janitor import state
from janitor.site import env


async def write_history(limit=None):
    template = env.get_template('history.html')
    async with state.get_connection() as conn:
        return await template.render_async(
            count=limit,
            history=state.iter_runs(conn, limit=limit))


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
