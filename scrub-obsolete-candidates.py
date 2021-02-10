#!/usr/bin/python3

# Copyright (C) 2018-2020 Jelmer Vernooij <jelmer@jelmer.uk>
#
# This program is free software; you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation; either version 2 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program; if not, write to the Free Software
# Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA


from janitor.candidates_pb2 import Candidate, CandidateList
from janitor.udd import UDD


DEFAULT_VALUE_SCRUB_OBSOLETE = 50


async def iter_scrub_obsolete_candidates(udd, packages=None):
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
        candidate.value = DEFAULT_VALUE_SCRUB_OBSOLETE
        candidate.suite = "scrub-obsolete"
        yield candidate


async def main():
    import argparse

    parser = argparse.ArgumentParser(prog="scrub-obsolete-candidates")
    parser.add_argument("packages", nargs="*", default=None)

    args = parser.parse_args()

    udd = await UDD.public_udd_mirror()
    async for candidate in iter_scrub_obsolete_candidates(udd, args.packages or None):
        cl = CandidateList()
        cl.candidate.append(candidate)
        print(cl)


if __name__ == "__main__":
    import asyncio

    asyncio.run(main())
