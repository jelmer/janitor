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

import asyncpg
from debian.changelog import Version
import shlex
from typing import Optional, Dict, List, Tuple
from breezy import urlutils
from janitor.state import Codebase


async def popcon(conn: asyncpg.Connection):
    return await conn.fetch("SELECT name, popcon_inst FROM package")


class Package(Codebase):

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


async def get_package_by_branch_url(
    conn: asyncpg.Connection, branch_url: str
) -> Optional[Package]:
    query = """
SELECT
""" + ','.join(Package.field_names) + """
FROM
  package
WHERE
  branch_url = $1 OR branch_url = $2
"""
    branch_url2 = urlutils.split_segment_parameters(branch_url)[0]
    row = await conn.fetchrow(query, branch_url, branch_url2)
    if row is None:
        return None
    return Package.from_row(row)


async def iter_vcs_regressions(conn: asyncpg.Connection):
    query = """\
select
  package.name,
  run.suite,
  run.id,
  run.result_code,
  package.vcswatch_status
from
  last_runs run left join package on run.package = package.name
where
  result_code in (
    'branch-missing',
    'branch-unavailable',
    '401-unauthorized',
    'hosted-on-alioth',
    'missing-control-file'
  )
and
  vcswatch_status in ('old', 'new', 'commits', 'ok')
"""
    return await conn.fetch(query)


async def get_package_by_upstream_branch_url(
    conn: asyncpg.Connection, upstream_branch_url: str
) -> Optional[Package]:
    query = """
SELECT
""" + ','.join(Package.field_names) + """
FROM
  package
WHERE
  name IN (
    SELECT package FROM upstream_branch_urls WHERE url = $1 OR url = $2)
"""
    upstream_branch_url2 = urlutils.split_segment_parameters(upstream_branch_url)[0]
    row = await conn.fetchrow(query, upstream_branch_url, upstream_branch_url2)
    if row is None:
        return None
    return Package.from_row(row)


async def iter_packages(conn: asyncpg.Connection, package=None):
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


async def iter_packages_by_maintainer(conn: asyncpg.Connection, maintainer):
    return [
        (row[0], row[1])
        for row in await conn.fetch(
            "SELECT name, removed FROM package WHERE "
            "maintainer_email = $1 OR $1 = any(uploader_emails)",
            maintainer,
        )
    ]


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
        tuple([Package.from_row(row)] + list(row[10:]))  # type: ignore
        for row in await conn.fetch(query, *args)
    ]


async def iter_candidates_with_policy(
    conn: asyncpg.Connection,
    packages: Optional[List[str]] = None,
    suite: Optional[str] = None,
) -> List[
    Tuple[
        Package,
        str,
        Optional[str],
        Optional[int],
        Optional[float],
        Dict[str, str],
        str,
        List[str],
    ]
]:
    query = """
SELECT
""" + ','.join(['package.%s' % field for field in Package.field_names]) + """,
  candidate.suite,
  candidate.context,
  candidate.value,
  candidate.success_chance,
  policy.publish,
  policy.update_changelog,
  policy.command
FROM candidate
INNER JOIN package on package.name = candidate.package
LEFT JOIN policy ON
    policy.package = package.name AND
    policy.suite = candidate.suite
WHERE NOT package.removed
"""
    args = []
    if suite is not None and packages is not None:
        query += " AND package.name = ANY($1::text[]) AND candidate.suite = $2"
        args.extend([packages, suite])
    elif suite is not None:
        query += " AND candidate.suite = $1"
        args.append(suite)
    elif packages is not None:
        query += " AND package.name = ANY($1::text[])"
        args.append(packages)
    return [
        (
            Package.from_row(row),
            row[len(Package.field_names) + 0],
            row[len(Package.field_names) + 1],
            row[len(Package.field_names) + 2],
            row[len(Package.field_names) + 3],
            (
                dict(row[len(Package.field_names)+4]) if row[len(Package.field_names)+4] is not None else None,
                row[len(Package.field_names)+5],
                shlex.split(row[len(Package.field_names)+6]) if row[len(Package.field_names)+6] is not None else None,
            ),
        )  # type: ignore
        for row in await conn.fetch(query, *args)
    ]


async def get_candidate(conn: asyncpg.Connection, package, suite):
    return await conn.fetchrow(
        "SELECT context, value, success_chance FROM candidate "
        "WHERE package = $1 AND suite = $2",
        package,
        suite,
    )


async def get_last_build_version(
    conn: asyncpg.Connection, package: str, suite: str
) -> Optional[Version]:
    return await conn.fetchval(
        "SELECT build_version FROM run WHERE "
        "build_version IS NOT NULL AND package = $1 AND "
        "build_distribution = $2 ORDER BY build_version DESC",
        package,
        suite,
    )


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
  build_version
FROM
  run
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
            "version_match1": "build_version %s $3" % (version[0],),
            "version_match2": "archive_version %s $3" % (version[0],),
        }
        args.append(str(version[1]))
    else:
        query = query % {"version_match1": "True", "version_match2": "True"}
    return await conn.fetch(query, *args)


async def store_debian_build(
    conn: asyncpg.Connection,
    run_id: str,
    source: str,
    version: Version,
    distribution: str,
):
    await conn.execute(
        "INSERT INTO debian_build (run_id, source, version, distribution) "
        "VALUES ($1, $2, $3, $4)",
        run_id,
        source,
        str(version),
        distribution,
    )


async def update_removals(
    conn: asyncpg.Connection,
    distribution: str,
    items: List[Tuple[str, Optional[Version]]],
) -> None:
    if not items:
        return
    query = """\
UPDATE package SET removed = True
WHERE name = $1 AND distribution = $2 AND archive_version <= $3
"""
    await conn.executemany(
        query,
        [(name, distribution, archive_version) for (name, archive_version) in items],
    )


async def guess_package_from_revision(
    conn: asyncpg.Connection, revision: bytes
) -> Tuple[Optional[str], Optional[str]]:
    query = """\
select distinct package, maintainer_email from run
left join new_result_branch rb ON rb.run_id = run.id
left join package on package.name = run.package
where rb.revision = $1 and run.package is not null
"""
    rows = await conn.fetch(query, revision.decode("utf-8"))
    if len(rows) == 1:
        return rows[0][0], rows[0][1]
    return None, None


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
