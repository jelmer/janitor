#!/usr/bin/python3

from janitor.udd import UDD
from janitor.candidates_pb2 import CandidateList, Candidate


DEFAULT_VALUE_NEW_UPSTREAM_SNAPSHOTS = 20


async def iter_fresh_snapshots_candidates(udd, packages):
    args = []
    query = """\
SELECT DISTINCT ON (sources.source)
sources.source, exists (
select from upstream_metadata where
key = 'Repository' and source = sources.source)
from sources
where sources.vcs_url != '' and position('-' in sources.version) > 0 AND
sources.release = 'sid'
"""
    if packages is not None:
        query += " AND sources.source = any($1::text[])"
        args.append(tuple(packages))
    query += " ORDER BY sources.source, sources.version DESC"
    for row in await udd.fetch(query, *args):
        candidate = Candidate()
        candidate.package = row[0]
        candidate.suite = 'fresh-snapshots'
        candidate.value = DEFAULT_VALUE_NEW_UPSTREAM_SNAPSHOTS
        candidate.success_chance = 1.0 if row[1] else 0.1
        yield candidate


async def main():
    import argparse

    parser = argparse.ArgumentParser(prog='fresh-snapshots-candidates')
    parser.add_argument("packages", nargs='*', default=None)

    args = parser.parse_args()

    udd = await UDD.public_udd_mirror()
    async for candidate in iter_fresh_snapshots_candidates(
            udd, args.packages or None):
        cl = CandidateList()
        cl.candidate.append(candidate)
        print(cl)


if __name__ == '__main__':
    import asyncio
    asyncio.run(main())
