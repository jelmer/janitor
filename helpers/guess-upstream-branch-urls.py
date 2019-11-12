#!/usr/bin/python3

import argparse
import asyncio
import urllib.error

from janitor import state
from janitor.config import read_config

from lintian_brush.upstream_metadata import guess_from_launchpad


async def main(db):
    async with db.acquire() as conn:
        async for pkg, version in state.iter_missing_upstream_branch_packages(conn):
            metadata = dict(guess_from_launchpad(pkg))
            try:
                repo_url = metadata['Repository']
            except KeyError:
                continue
            print('Setting %s to %s' % (pkg, repo_url))
            await state.set_upstream_branch_url(conn, pkg, repo_url)

parser = argparse.ArgumentParser('reschedule')
parser.add_argument(
    '--config', type=str, default='janitor.conf',
    help='Path to configuration.')

args = parser.parse_args()
with open(args.config, 'r') as f:
    config = read_config(f)

db = state.Database(config.database_location)
asyncio.run(main(db))
