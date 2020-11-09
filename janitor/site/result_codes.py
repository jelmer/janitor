#!/usr/bin/python3

import operator


async def generate_result_code_index(
        by_code, never_processed, suite, all_suites):

    data = [[code, count]
            for (code, count) in
            sorted(by_code, key=operator.itemgetter(1), reverse=True)]
    data.append(('never-processed', never_processed))
    return {'result_codes': data, 'suite': suite, 'all_suites': all_suites}
