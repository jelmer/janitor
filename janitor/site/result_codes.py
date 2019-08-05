#!/usr/bin/python3

import operator
import os

from janitor import state
from janitor.site import env


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
