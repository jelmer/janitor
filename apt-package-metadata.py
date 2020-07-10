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

"""Exporting of upstream metadata from an apt repository."""

from debian.deb822 import Sources
from aiohttp import ClientSession
import gzip
from janitor.package_metadata_pb2 import PackageList, PackageMetadata
from typing import List, Optional
from email.utils import parseaddr
from breezy.plugins.debian.directory import source_package_vcs


def extract_uploader_emails(uploaders: Optional[str]) -> List[str]:
    if not uploaders:
        return []
    ret = []
    for uploader in uploaders.split(','):
        if not uploader:
            continue
        email = parseaddr(uploader)[1]
        if not email:
            continue
        ret.append(email)
    return ret


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
    parser = argparse.ArgumentParser(prog='apt-package-metadata')
    parser.add_argument("url", nargs='*')
    args = parser.parse_args()

    for url in args.url:
        async for source in iter_sources(url):
            pl = PackageList()
            package = PackageMetadata()
            package.name = source['Package']
            package.maintainer_email = parseaddr(source['Maintainer'])[1]
            package.uploader_email.extend(
                extract_uploader_emails(source.get('Uploaders')))
            try:
                (vcs_type, vcs_url) = source_package_vcs(source)
            except KeyError:
                pass
            else:
                package.vcs_type = vcs_type
                package.vcs_url = vcs_url
            if 'Vcs-Browser' in source:
                package.vcs_browser = source['Vcs-Browser']
            package.archive_version = source['Version']
            package.removed = False
            pl.package.append(package)
            print(pl)


if __name__ == '__main__':
    import asyncio
    asyncio.run(main())
