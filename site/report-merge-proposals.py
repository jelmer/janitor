#!/usr/bin/python3

import argparse
import asyncio
import sys
import os

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402
from janitor.site import env  # noqa: E402

parser = argparse.ArgumentParser('report-state')
parser.add_argument('name', nargs='?', type=str, default=None)
args = parser.parse_args()

async def write_merge_proposals():
    proposals_by_status = {}
    for url, status, package in await state.iter_all_proposals(
            branch_name=args.name):
        proposals_by_status.setdefault(status, []).append(url)

    template = env.get_template('merge-proposals.html')
    sys.stdout.write(await template.render_async(
            open_proposals=proposals_by_status.get('open', []),
            merged_proposals=proposals_by_status.get('merged', []),
            closed_proposals=proposals_by_status.get('closed', [])))


loop = asyncio.get_event_loop()
loop.run_until_complete(write_merge_proposals())
