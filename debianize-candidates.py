#!/usr/bin/python3

from janitor.candidates_pb2 import Candidate, CandidateList


async def iter_debianize_candidates(packages=None):
    # TODO
    for source, entries in bysource.items():
        if packages is not None and source not in packages:
            continue
        candidate = Candidate()
        candidate.package = source
        # TODO(jelmer): Set context
        candidate.context = None
        # TODO(jelmer): Set value
        candidate.value = None
        yield candidate


async def main():
    import argparse

    parser = argparse.ArgumentParser(prog='debianize-candidates')
    parser.add_argument("packages", nargs='*', default=None)

    args = parser.parse_args()

    async for candidate in iter_debianize_candidates(
            args.packages or None):
        cl = CandidateList()
        cl.candidate.append(candidate)
        print(cl)


if __name__ == '__main__':
    import asyncio
    asyncio.run(main())
