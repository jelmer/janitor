#!/usr/bin/python3

from debian.changelog import Version
from janitor.udd import UDD
from janitor.candidates_pb2 import Candidate, CandidateList


DEFAULT_VALUE_NEW_UPSTREAM = 30
INVALID_VERSION_DOWNGRADE = 5


async def iter_fresh_releases_candidates(udd, packages=None):
    args = []
    query = """\
SELECT DISTINCT ON (sources.source)
sources.source, upstream.upstream_version FROM upstream \
INNER JOIN sources ON upstream.version = sources.version \
AND upstream.source = sources.source where \
status = 'newer package available' AND \
sources.vcs_url != '' AND \
sources.release = 'sid'
"""
    if packages is not None:
        query += " AND upstream.source = any($1::text[])"
        args.append(tuple(packages))
    query += " ORDER BY sources.source, sources.version DESC"
    for row in await udd.fetch(query, *args):
        candidate = Candidate()
        candidate.package = row[0]
        candidate.context = row[1]
        candidate.value = DEFAULT_VALUE_NEW_UPSTREAM
        try:
            Version(row[1])
        except ValueError:
            candidate.value -= INVALID_VERSION_DOWNGRADE
        yield candidate


async def main():
    import argparse

    parser = argparse.ArgumentParser(prog='fresh-releases-candidates')
    parser.add_argument("packages", nargs='*', default=None)

    args = parser.parse_args()

    udd = await UDD.public_udd_mirror()
    async for candidate in iter_fresh_releases_candidates(
            udd, args.packages or None):
        cl = CandidateList()
        cl.candidate.append(candidate)
        print(cl)


if __name__ == '__main__':
    import asyncio
    asyncio.run(main())
