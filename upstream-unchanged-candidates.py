#!/usr/bin/python3

from janitor.candidates_pb2 import Candidate, CandidateList
from janitor.config import read_config
from janitor import state


DEFAULT_VALUE_DEBIANIZE = 25


async def iter_debianize_candidates(db, packages=None):
    async with db.acquire() as conn:
        for (source,) in await conn.fetch("SELECT name FROM package WHERE name LIKE '%-upstream'"):
            if packages is not None and source not in packages:
                continue
            candidate = Candidate()
            candidate.package = source
            # TODO(jelmer): Set context
            # candidate.context = None
            candidate.suite = "upstream-unchanged"
            candidate.value = DEFAULT_VALUE_DEBIANIZE
            yield candidate


async def main():
    import argparse

    parser = argparse.ArgumentParser(prog="upstream-unchanged-candidates")
    parser.add_argument("packages", nargs="*", default=None)

    parser.add_argument(
        "--config", type=str, default="janitor.conf", help="Path to configuration."
    )

    args = parser.parse_args()

    with open(args.config, "r") as f:
        config = read_config(f)

    db = state.Database(config.database_location)
    async for candidate in iter_debianize_candidates(db, args.packages or None):
        cl = CandidateList()
        cl.candidate.append(candidate)
        print(cl)


if __name__ == "__main__":
    import asyncio

    asyncio.run(main())
