#!/usr/bin/python3

import argparse
import asyncio
import operator
import os
import sys

from jinja2 import Environment, FileSystemLoader, select_autoescape

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402
from janitor.site import env  # noqa: E402

parser = argparse.ArgumentParser(prog='report-result-codes')
parser.add_argument(
    'path', type=str, default='result-codes', help='Output path')
args = parser.parse_args()

loop = asyncio.get_event_loop()

by_code = {}

for (source, command, result_code, log_id,
     description, duration) in loop.run_until_complete(
         state.iter_last_runs()):
    by_code.setdefault(result_code, []).append(
        (source, command, log_id, description))

async def write_result_code_page(code, items):
    template = env.get_template('result-code.html')
    data = []
    for (source, command, log_id, description) in items:
        data.append((
            source,
            log_id,
            command,
            description))
    with open(os.path.join(args.path, '%s.html' % code), 'w') as f:
        f.write(await template.render_async(code=code, entries=data))

async def write_result_code_index(by_code):
    with open(os.path.join(args.path, 'index.html'), 'w') as f:
        template = env.get_template('result-code-index.html')

        data = sorted(
            [[name, len(by_code[name])] for name in by_code],
            key=operator.itemgetter(1), reverse=True)
        f.write(await template.render_async(result_codes=data))


jobs = [write_result_code_index(by_code)]
for code, items in by_code.items():
    jobs.append(write_result_code_page(code, items))
loop.run_until_complete(asyncio.gather(*jobs))
