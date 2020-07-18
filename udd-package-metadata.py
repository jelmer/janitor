#!/usr/bin/python3
# Copyright (C) 2018 Jelmer Vernooij <jelmer@jelmer.uk>
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

"""Exporting of upstream metadata from UDD."""

from debian.changelog import Version
from email.utils import parseaddr
from google.protobuf import text_format  # type: ignore
from typing import List, Optional, Iterator, AsyncIterator, Tuple

from janitor.package_metadata_pb2 import PackageList, PackageMetadata, Removal
from janitor.udd import UDD


def extract_uploader_emails(uploaders: str) -> List[str]:
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


async def iter_packages_with_metadata(
        udd: UDD, packages: Optional[List[str]] = None
        ) -> AsyncIterator[Tuple[
            str, str, str, int, str, str,
            str, str, str, str, Version, Version]]:
    args = []
    query = """
select distinct on (sources.source) sources.source,
sources.maintainer_email, sources.uploaders, popcon_src.insts,
coalesce(vcswatch.vcs, sources.vcs_type),
coalesce(vcswatch.url, sources.vcs_url),
vcswatch.branch,
coalesce(vcswatch.browser, sources.vcs_browser),
commit_id,
status as vcswatch_status,
sources.version,
vcswatch.version
from sources left join popcon_src on sources.source = popcon_src.source
left join vcswatch on vcswatch.source = sources.source
where sources.release = 'sid'
"""
    if packages:
        query += " and sources.source = ANY($1::text[])"
        args.append(packages)
    query += " order by sources.source, sources.version desc"
    for row in await udd.fetch(query, *args):
        yield row


async def iter_removals(
        udd: UDD, packages: Optional[List[str]] = None) -> Iterator:
    query = """\
select name, version from package_removal where 'source' = any(arch_array)
"""
    args = []
    if packages:
        query += " and name = ANY($1::text[])"
        args.append(packages)
    return await udd.fetch(query, *args)


async def main():
    import argparse
    parser = argparse.ArgumentParser(prog='candidates')
    parser.add_argument("packages", nargs='*')
    args = parser.parse_args()

    udd = await UDD.public_udd_mirror()

    removals = {}
    for name, version in await iter_removals(
            udd, packages=args.packages):
        if name not in removals:
            removals[name] = Version(version)
        else:
            removals[name] = max(Version(version), removals[name])

    for name, version in removals.items():
        pl = PackageList()
        removal = pl.removal.add()
        removal.name = name
        removal.version = str(version)
        print(pl)

    async for (name, maintainer_email, uploaders, insts, vcs_type, vcs_url,
         vcs_branch, vcs_browser, commit_id, vcswatch_status, sid_version,
         vcswatch_version) in iter_packages_with_metadata(
                    udd, args.packages):
        pl = PackageList()
        package = pl.package.add()
        package.name = name
        package.maintainer_email = maintainer_email
        package.uploader_email.extend(extract_uploader_emails(uploaders))
        if insts is not None:
            package.insts = insts
        if vcs_type:
            package.vcs_type = vcs_type
        if vcs_url:
            package.vcs_url = vcs_url
        if vcs_branch:
            package.vcs_branch = vcs_branch
        if vcs_browser:
            package.vcs_browser = vcs_browser
        if commit_id:
            package.commit_id = commit_id
        if vcswatch_status:
            package.vcswatch_status = vcswatch_status
        package.archive_version = sid_version
        if vcswatch_version:
            package.vcswatch_version = vcswatch_version
        if name not in removals:
            package.removed = False
        else:
            package.removed = Version(sid_version) <= removals[name]
        print(pl)

if __name__ == '__main__':
    import asyncio
    asyncio.run(main())
