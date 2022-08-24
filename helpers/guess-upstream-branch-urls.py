#!/usr/bin/python3

import argparse
import asyncio
import logging
import sys
import traceback

import asyncpg
from google.protobuf import text_format

from janitor import state
from janitor.config import read_config
from janitor import package_overrides_pb2

from upstream_ontologist.guess import (
    guess_from_launchpad,
    guess_from_aur,
    guess_from_pecl,
    )


async def iter_missing_upstream_branch_packages(conn: asyncpg.Connection):
    query = """\
select
  package.name,
  package.archive_version
from
  last_runs
inner join package on last_runs.package = package.name
left outer join upstream on upstream.name = package.name
where
  result_code = 'upstream-branch-unknown' and
  upstream.upstream_branch_url is null
order by package.name asc
"""
    for row in await conn.fetch(query):
        yield row[0], row[1]


async def main(db_location, start=None):
    async with state.create_pool(db_location) as pool:
        async for pkg, version in iter_missing_upstream_branch_packages(pool):
            if start and pkg < start:
                continue
            logging.info('Package: %s' % pkg)
            urls = []
            for name, guesser in [
                    ('aur', guess_from_aur),
                    ('lp', guess_from_launchpad),
                    ('pecl', guess_from_pecl)]:
                try:
                    metadata = dict(guesser(pkg))
                except Exception:
                    traceback.print_exc()
                    continue
                try:
                    repo_url = metadata['Repository']
                except KeyError:
                    continue
                else:
                    urls.append((name, repo_url))
            if not urls:
                continue
            if len(urls) > 1:
                print('# Note: Conflicting URLs for %s: %r' % (pkg, urls))
            config = package_overrides_pb2.OverrideConfig()
            override = config.package.add()
            override.name = pkg
            override.upstream_branch_url = urls[0][1]
            print("# From %s" % urls[0][0])
            text_format.PrintMessage(config, sys.stdout)

parser = argparse.ArgumentParser('guess-upstream-branch-urls')
parser.add_argument(
    '--config', type=str, default='janitor.conf',
    help='Path to configuration.')
parser.add_argument(
    '--start', type=str, default='',
    help='Only process package with names after this one.')

args = parser.parse_args()
with open(args.config, 'r') as f:
    config = read_config(f)

logging.basicConfig(level=logging.INFO)

asyncio.run(main(config.database_location, args.start))
