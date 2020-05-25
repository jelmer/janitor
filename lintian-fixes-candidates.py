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

from silver_platter.debian.lintian import (
    DEFAULT_ADDON_FIXERS,
    calculate_value,
    )


async def iter_lintian_fixes_candidates(udd, packages, available_fixers):
    """Iterate over all of the packages affected by a set of tags."""
    package_rows = {}
    package_tags = {}

    args = [tuple(available_fixers)]
    query = """
SELECT DISTINCT ON (sources.source)
sources.source,
sources.version,
sources.vcs_type,
sources.vcs_url,
sources.maintainer_email,
sources.uploaders,
ARRAY(SELECT tag FROM lintian WHERE
    sources.source = lintian.package AND
    sources.version = lintian.package_version AND
    lintian.package_type = 'source' AND
    tag = any($1::text[])
)
FROM sources
WHERE
sources.release = 'sid'
AND vcs_type != ''
"""
    if packages is not None:
        query += " AND sources.source = any($2::text[])"
        args.append(tuple(packages))
    query += " ORDER BY sources.source, sources.version DESC"
    for row in await udd.fetch(query, *args):
        package_rows[row[0]] = row[:6]
        package_tags[row[0]] = set(row[6])
    args = [tuple(available_fixers)]
    query = """\
SELECT DISTINCT ON (sources.source)
sources.source,
sources.version,
sources.vcs_type,
sources.vcs_url,
sources.maintainer_email,
sources.uploaders,
ARRAY(SELECT lintian.tag FROM lintian
    INNER JOIN packages ON packages.package = lintian.package \
    and packages.version = lintian.package_version \
WHERE
    lintian.tag = any($1::text[]) AND
    lintian.package_type = 'binary' AND
    sources.version = packages.version and \
    sources.source = packages.source
)
FROM
sources
WHERE sources.release = 'sid' AND vcs_type != ''"""
    if packages is not None:
        query += " AND sources.source = ANY($2::text[])"
        args.append(tuple(packages))
    query += " ORDER BY sources.source, sources.version DESC"
    for row in await udd.fetch(query, *args):
        package_tags.setdefault(row[0], set()).update(row[6])
    for row in package_rows.values():
        tags = sorted(package_tags[row[0]])
        value = calculate_value(tags)
        context = ' '.join(sorted(tags))
        candidate = Candidate()
        candidate.package = row[0]
        candidate.context = context
        candidate.value = value
        yield candidate


async def main():
    import argparse
    from silver_platter.debian.lintian import (
        available_lintian_fixers,
    )

    parser = argparse.ArgumentParser(prog='lintian-fixes-candidates')
    parser.add_argument("packages", nargs='*', default=None)

    args = parser.parse_args()

    tags = set()
    available_fixers = list(available_lintian_fixers())
    for fixer in available_fixers:
        tags.update(fixer.lintian_tags)

    udd = await UDD.public_udd_mirror()
    async for candidate in iter_lintian_fixes_candidates(
            udd, args.packages or None, tags):
        cl = CandidateList()
        cl.candidate.append(candidate)
        print(cl)


if __name__ == '__main__':
    import asyncio
    asyncio.run(main())
