#!/usr/bin/python3

import operator
import os

from janitor import state
from janitor.site import env


async def get_results_by_code(code):
    by_code = {}
    for (source, command, result_code, log_id,
         description, start_time, duration) in await state.iter_last_runs():
        by_code.setdefault(result_code, []).append(
            (source, command, log_id, description))
    return by_code.get(code, [])


async def generate_result_code_page(code, entries):
    template = env.get_template('result-code.html')
    return await template.render_async(code=code, entries=entries)


async def write_result_code_page(path, code, items):
    with open(os.path.join(path, '%s.html' % code), 'w') as f:
        f.write(await generate_result_code_page(code, items))


async def generate_result_code_index(by_code):
    template = env.get_template('result-code-index.html')

    data = sorted(by_code, key=operator.itemgetter(1), reverse=True)
    return await template.render_async(result_codes=data)


async def write_result_code_index(path, by_code):
    with open(os.path.join(args.path, 'index.html'), 'w') as f:
        f.write(await generate_result_code_index(by_code))


if __name__ == '__main__':
    import argparse
    import asyncio
    parser = argparse.ArgumentParser(prog='report-result-codes')
    parser.add_argument(
        'path', type=str, default='result-codes', help='Output path')
    args = parser.parse_args()

    loop = asyncio.get_event_loop()
    by_code = loop.run_until_complete(get_results_by_code())
    jobs = [write_result_code_index(args.path, by_code)]
    for code, items in by_code.items():
        jobs.append(write_result_code_page(args.path, code, items))
    loop.run_until_complete(asyncio.gather(*jobs))
