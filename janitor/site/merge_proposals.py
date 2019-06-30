#!/usr/bin/python3

import sys
import os

from janitor import state
from janitor.site import env

async def write_merge_proposals():
    proposals_by_status = {}
    for url, status, package in await state.iter_all_proposals(
            branch_name=args.name):
        proposals_by_status.setdefault(status, []).append(url)

    template = env.get_template('merge-proposals.html')
    return await template.render_async(
            open_proposals=proposals_by_status.get('open', []),
            merged_proposals=proposals_by_status.get('merged', []),
            closed_proposals=proposals_by_status.get('closed', []))


if __name__ == '__main__':
    import argparse
    import asyncio
    parser = argparse.ArgumentParser('report-state')
    parser.add_argument('name', nargs='?', type=str, default=None)
    args = parser.parse_args()
    loop = asyncio.get_event_loop()
    sys.stdout.write(loop.run_until_complete(write_merge_proposals()))
