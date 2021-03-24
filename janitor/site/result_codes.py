#!/usr/bin/python3

import asyncpg
import operator


async def stats_by_result_codes(conn: asyncpg.Connection, suite=None):
    query = """\
select (
    case when result_code = 'nothing-new-to-do' then 'success'
    else result_code end), count(result_code) from last_runs
"""
    args = []
    if suite:
        args.append(suite)
        query += " WHERE suite = $1"
    query += " group by 1 order by 2 desc"
    return await conn.fetch(query, *args)


async def generate_result_code_index(by_code, never_processed, suite, all_suites):

    data = [
        [code, count]
        for (code, count) in sorted(by_code, key=operator.itemgetter(1), reverse=True)
    ]
    data.append(("never-processed", never_processed))
    return {"result_codes": data, "suite": suite, "all_suites": all_suites}
