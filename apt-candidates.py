#!/usr/bin/python3
# Copyright (C) 2020 Jelmer Vernooij <jelmer@jelmer.uk>
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

"""Exporting of candidates without ."""

from debian.deb822 import Sources
from aiohttp import ClientSession
import gzip
from janitor.candidates_pb2 import CandidateList, Candidate


async def iter_sources(url):
    async with ClientSession() as session:
        async with session.get(url) as resp:
            if resp.status != 200:
                raise Exception(
                    'URL %s returned response code %d' % (
                        url, resp.status))
            contents = await resp.read()
            if url.endswith('.gz'):
                contents = gzip.decompress(contents)
            for source in Sources.iter_paragraphs(contents):
                yield source


async def main():
    import apt_pkg
    import argparse
    parser = argparse.ArgumentParser(prog='apt-candidates')
    parser.add_argument("url", nargs='*')
    parser.add_argument(
        '--maintainer', action='append', type=str,
        help='Filter by maintainer email')
    parser.add_argument(
        '--suite', action='append', type=str,
        help='Suite to generate candidate for.')
    args = parser.parse_args()

    for url in args.url:
        async for source in iter_sources(url):
            for suite in args.suite:
                cl = CandidateList()
                candidate = Candidate()
                candidate.suite = suite
                candidate.package = source['Package']
                cl.candidate.append(candidate)
                print(cl)


if __name__ == '__main__':
    import asyncio
    asyncio.run(main())
