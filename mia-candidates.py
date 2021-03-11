#!/usr/bin/python3

from janitor.candidates_pb2 import Candidate, CandidateList
from janitor.udd import UDD


DEFAULT_VALUE_MIA = 70


async def iter_mia_candidates(udd, packages=None):
    from silver_platter.debian.mia import MIA_EMAIL, MIA_TEAMMAINT_USERTAG
    query = """\
SELECT source, id from bugs
WHERE
  id IN (select id from bugs_usertags where email = $1 and tag = $2) AND
  status = 'pending'
"""
    args = [MIA_EMAIL, MIA_TEAMMAINT_USERTAG]
    if packages is not None:
        query += " AND sources.source = any($3::text[])"
        args.append(tuple(packages))
    for row in await udd.fetch(query, *args):
        candidate = Candidate()
        candidate.package = row[0]
        candidate.suite = "mia"
        candidate.context = str(row[1])
        candidate.value = DEFAULT_VALUE_MIA
        yield candidate


async def main():
    import argparse

    parser = argparse.ArgumentParser(prog="mia-candidates")
    parser.add_argument("packages", nargs="*", default=None)

    args = parser.parse_args()

    udd = await UDD.public_udd_mirror()
    async for candidate in iter_mia_candidates(udd, args.packages or None):
        cl = CandidateList()
        cl.candidate.append(candidate)
        print(cl)


if __name__ == "__main__":
    import asyncio

    asyncio.run(main())
