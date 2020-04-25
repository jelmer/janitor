#!/usr/bin/python3

from janitor.udd import UDD
from janitor.candidates_pb2 import Candidate, CandidateList


DEFAULT_VALUE_UNCOMMITTED = 60
UNCOMMITTED_NMU_BONUS = 10


async def iter_missing_commits(udd, packages=None):
    args = []
    query = """\
SELECT sources.source, sources.version, vcswatch.url
FROM vcswatch JOIN sources ON sources.source = vcswatch.source
WHERE
vcswatch.status IN ('OLD', 'UNREL') AND
sources.release = 'sid'
"""
    if packages is not None:
        query += " AND sources.source = any($1::text[])"
        args.append(tuple(packages))
    for row in await udd.fetch(query, *args):
        value = DEFAULT_VALUE_UNCOMMITTED
        if 'nmu' in str(row[1]):
            value += UNCOMMITTED_NMU_BONUS
        candidate = Candidate()
        candidate.package = row[0]
        candidate.value = value
        yield candidate


async def main():
    import argparse

    parser = argparse.ArgumentParser(prog='uncommitted-candidates')
    parser.add_argument("packages", nargs='*', default=None)

    args = parser.parse_args()

    udd = await UDD.public_udd_mirror()
    async for candidate in iter_missing_commits(
            udd, args.packages or None):
        cl = CandidateList()
        cl.candidate.append(candidate)
        print(cl)


if __name__ == '__main__':
    import asyncio
    asyncio.run(main())
