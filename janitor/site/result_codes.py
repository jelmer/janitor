#!/usr/bin/python3

import operator

from janitor.site import env


async def generate_result_code_page(code, entries, suite):
    template = env.get_template('result-code.html')
    return await template.render_async(code=code, runs=entries, suite=suite)


async def generate_result_code_index(by_code, never_processed, suite):
    template = env.get_template('result-code-index.html')

    data = sorted(by_code, key=operator.itemgetter(1), reverse=True)
    data.append(('never-processed', never_processed))
    return await template.render_async(result_codes=data, suite=suite)
