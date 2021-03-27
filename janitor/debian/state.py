#!/usr/bin/python
# Copyright (C) 2018-2021 Jelmer Vernooij <jelmer@jelmer.uk>
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

from typing import Optional, Dict, List, Tuple, Any


import asyncpg
from breezy import urlutils
from debian.changelog import Version


class Package(object):

    name: str
    branch_url: str
    maintainer_email: str
    uploader_emails: List[str]
    subpath: Optional[str]
    archive_version: Optional[Version]
    vcs_type: Optional[str]
    vcs_url: Optional[str]
    vcs_browse: Optional[str]
    popcon_inst: Optional[int]
    removed: bool
    vcswatch_status: str
    vcswatch_version: str
    upstream_branch_url: Optional[str]

    def __init__(
        self,
        name,
        maintainer_email,
        uploader_emails,
        branch_url,
        vcs_type,
        vcs_url,
        vcs_browse,
        removed,
        vcswatch_status,
        vcswatch_version,
        in_base
    ):
        self.name = name
        self.maintainer_email = maintainer_email
        self.uploader_emails = uploader_emails
        self.branch_url = branch_url
        self.vcs_type = vcs_type
        self.vcs_url = vcs_url
        self.vcs_browse = vcs_browse
        self.removed = removed
        self.vcswatch_status = vcswatch_status
        self.vcswatch_version = vcswatch_version
        self.in_base = in_base

    field_names = ["name", "maintainer_email", "uploader_emails", "branch_url", "vcs_type", "vcs_url", "vcs_browse", "removed", "vcswatch_status", "vcswatch_version", "in_base"]

    @classmethod
    def from_row(cls, row) -> "Package":
        return cls(*row)

    def __lt__(self, other) -> bool:
        if not isinstance(other, type(self)):
            raise TypeError(other)
        return self.__tuple__ < other.__tuple__()

    def __tuple__(self):
        return (
            self.name,
            self.maintainer_email,
            self.uploader_emails,
            self.branch_url,
            self.vcs_type,
            self.vcs_url,
            self.vcs_browse,
            self.removed,
            self.vcswatch_status,
            self.vcswatch_version,
            self.in_base
        )


async def iter_packages(conn: asyncpg.Connection, package: Optional[str] = None):
    query = """
SELECT
""" + ','.join(Package.field_names) + """
FROM
  package
"""
    args = []
    if package:
        query += " WHERE name = $1"
        args.append(package)
    query += " ORDER BY name ASC"
    return [Package.from_row(row) for row in await conn.fetch(query, *args)]


async def get_package(conn: asyncpg.Connection, name):
    try:
        return list(await iter_packages(conn, package=name))[0]
    except IndexError:
        return None


async def iter_candidates(
    conn: asyncpg.Connection,
    packages: Optional[List[str]] = None,
    suite: Optional[str] = None,
) -> List[Tuple[Package, str, Optional[str], Optional[int], Optional[float]]]:
    query = """
SELECT
""" + ','.join(['package.%s' % field for field in Package.field_names]) + """,
  candidate.suite,
  candidate.context,
  candidate.value,
  candidate.success_chance
FROM candidate
INNER JOIN package on package.name = candidate.package
WHERE NOT package.removed
"""
    args = []
    if suite is not None and packages is not None:
        query += " AND package.name = ANY($1::text[]) AND suite = $2"
        args.extend([packages, suite])
    elif suite is not None:
        query += " AND suite = $1"
        args.append(suite)
    elif packages is not None:
        query += " AND package.name = ANY($1::text[])"
        args.append(packages)
    return [
        tuple([Package.from_row(row[:len(Package.field_names)])] + list(row[len(Package.field_names):]))  # type: ignore
        for row in await conn.fetch(query, *args)
    ]


async def version_available(
    conn: asyncpg.Connection,
    package: str,
    suite: str,
    version: Optional[Tuple[str, Version]] = None,
) -> List[Tuple[str, str, Version]]:
    query = """\
SELECT
  package,
  suite,
  debian_build.version
FROM
  run
LEFT JOIN debian_build ON run.id = debian_build.run_id
WHERE
  package = $1 AND (suite = $2 OR suite = 'unchanged')
  AND %(version_match1)s

UNION

SELECT
  name,
  'unchanged',
  archive_version
FROM
  package
WHERE name = $1 AND %(version_match2)s
"""
    args = [package, suite]
    if version:
        query = query % {
            "version_match1": "debian_build.version %s $3" % (version[0],),
            "version_match2": "archive_version %s $3" % (version[0],),
        }
        args.append(str(version[1]))
    else:
        query = query % {"version_match1": "True", "version_match2": "True"}
    return await conn.fetch(query, *args)


async def iter_published_packages(conn: asyncpg.Connection, suite):
    return await conn.fetch(
        """
select distinct on (package.name) package.name, debian_build.version,
archive_version from debian_build
left join package on package.name = debian_build.source
where debian_build.distribution = $1 and not package.removed
order by package.name, debian_build.version desc
""",
        suite,
    )

