#!/usr/bin/python3

import argparse
import asyncio
import logging
import os
import sys
from janitor.publish_one import template_env
from janitor.config import read_config

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402
from janitor.debian.debdiff import markdownify_debdiff, debdiff_is_empty

loop = asyncio.get_event_loop()

parser = argparse.ArgumentParser()
parser.add_argument(
    "--config", type=str, default="janitor.conf", help="Path to configuration."
)
parser.add_argument(
    "-r", "--run-id", type=str, help="Run id to process"
)
parser.add_argument(
        "--role", type=str, help="Role", default="main"
)
parser.add_argument(
        '--format', type=str, choices=['md', 'txt'], default='md')

args = parser.parse_args()

logging.basicConfig(level=logging.INFO, format='%(message)s')


with open(args.config, "r") as f:
    config = read_config(f)


async def process_build(db, run_id, role, format):
    todo = []
    async with db.acquire() as conn:
        query = """
SELECT
  package,
  suite,
  id AS log_id,
  result AS _result
FROM run
WHERE
  id = $1
"""
        row = await conn.fetchrow(query, run_id)
        vs = {}
        vs.update(row)
        vs.update(row['_result'])
        vs['external_url'] = 'https://janitor.debian.net/'
        vs['markdownify_debdiff'] = markdownify_debdiff
        vs['debdiff_is_empty'] = debdiff_is_empty
        print(template_env.get_template(vs['suite'] + '.' + format).render(vs))


db = state.Database(config.database_location)
loop.run_until_complete(process_build(db, args.run_id, args.role, args.format))
