#!/usr/bin/python3

from janitor.candidates_pb2 import Candidate, CandidateList
from janitor.udd import UDD


DEFAULT_VALUE_MIA = 70


async def iter_mia_candidates(udd, packages=None):
    from silver_platter.debian.mia import get_candidates
    for package, bug in get_candidates():
        candidate = Candidate()
        if packages is not None and package not in packages:
            continue
        candidate.package = package
        candidate.suite = "mia"
        candidate.context = str(bug)
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
