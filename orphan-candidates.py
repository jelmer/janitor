#!/usr/bin/python3

from janitor.candidates_pb2 import Candidate, CandidateList
from janitor.udd import UDD


DEFAULT_VALUE_ORPHAN = 60


async def iter_orphan_candidates(udd, packages=None):
    args = []
    query = """\
SELECT DISTINCT ON (sources.source) sources.source, now() - orphaned_time, bug
FROM sources
JOIN orphaned_packages ON orphaned_packages.source = sources.source
WHERE sources.vcs_url != '' AND sources.release = 'sid' AND
orphaned_packages.type in ('O') AND
(sources.uploaders != '' OR
sources.maintainer != 'Debian QA Group <packages@qa.debian.org>')
"""
    if packages is not None:
        query += " AND sources.source = any($1::text[])"
        args.append(tuple(packages))
    for row in await udd.fetch(query, *args):
        candidate = Candidate()
        candidate.package = row[0]
        candidate.suite = 'orphan'
        candidate.context = str(row[2])
        candidate.value = DEFAULT_VALUE_ORPHAN
        yield candidate


async def main():
    import argparse

    parser = argparse.ArgumentParser(prog='orphan-candidates')
    parser.add_argument("packages", nargs='*', default=None)

    args = parser.parse_args()

    udd = await UDD.public_udd_mirror()
    async for candidate in iter_orphan_candidates(
            udd, args.packages or None):
        cl = CandidateList()
        cl.candidate.append(candidate)
        print(cl)


if __name__ == '__main__':
    import asyncio
    asyncio.run(main())
