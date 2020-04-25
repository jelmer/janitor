#!/usr/bin/python3

from janitor.candidates_pb2 import Candidate, CandidateList
from janitor.udd import UDD


DEFAULT_VALUE_UNCHANGED = 20


async def iter_unchanged_candidates(udd, packages=None):
    args = []
    query = """\
SELECT DISTINCT ON (sources.source) \
sources.source FROM sources WHERE \
sources.vcs_url != '' AND \
sources.release = 'sid'
"""
    if packages is not None:
        query += " AND sources.source = any($1::text[])"
        args.append(tuple(packages))
    for row in await udd.fetch(query, *args):
        candidate = Candidate()
        candidate.package = row[0]
        candidate.value = DEFAULT_VALUE_UNCHANGED
        yield candidate


async def main():
    import argparse

    parser = argparse.ArgumentParser(prog='unchanged-candidates')
    parser.add_argument("packages", nargs='*', default=None)

    args = parser.parse_args()

    udd = await UDD.public_udd_mirror()
    async for candidate in iter_unchanged_candidates(
            udd, args.packages or None):
        cl = CandidateList()
        cl.candidate.append(candidate)
        print(cl)


if __name__ == '__main__':
    import asyncio
    asyncio.run(main())
