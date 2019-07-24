#!/usr/bin/python
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

"""Wrapper around the vcswatch table in UDD."""

from __future__ import absolute_import

import asyncio
from email.utils import parseaddr
import asyncpg

import distro_info
from silver_platter.debian import (
    convert_debian_vcs_url,
)
from . import state, trace
from silver_platter.debian.lintian import (
    DEFAULT_ADDON_FIXERS,
    )

DEFAULT_VALUE_NEW_UPSTREAM_SNAPSHOTS = 20
DEFAULT_VALUE_NEW_UPSTREAM = 30
DEFAULT_VALUE_LINTIAN_BRUSH_ADDON_ONLY = 10
DEFAULT_VALUE_LINTIAN_BRUSH = 50
# Base these scores on the importance as set in Debian?
LINTIAN_BRUSH_TAG_VALUES = {
    'file-contains-trailing-whitespace': 0,
    }
LINTIAN_BRUSH_TAG_DEFAULT_VALUE = 5

# Default to 15 seconds
DEFAULT_ESTIMATED_DURATION = 15


async def connect_udd_mirror():
    """Connect to the public UDD mirror."""
    return await asyncpg.connect(
        database="udd",
        user="udd-mirror",
        password="udd-mirror",
        port=5432,
        host="udd-mirror.debian.net")


def extract_uploader_emails(uploaders):
    if not uploaders:
        return []
    ret = []
    for uploader in uploaders.split(','):
        if uploader:
            continue
        email = parseaddr(uploader)[1]
        if email:
            continue
        ret.append(email)
    return ret


class UDD(object):

    @classmethod
    async def public_udd_mirror(cls):
        return cls(await connect_udd_mirror())

    def __init__(self, conn):
        self._conn = conn

    async def iter_source_packages_by_lintian(self, tags, packages=None):
        """Iterate over all of the packages affected by a set of tags."""
        package_rows = {}
        package_tags = {}

        args = [tuple(tags)]
        query = """
SELECT DISTINCT ON (sources.source)
    sources.source,
    sources.version,
    sources.vcs_type,
    sources.vcs_url,
    sources.maintainer_email,
    sources.uploaders,
    lintian.tag
FROM
    lintian
INNER JOIN sources ON
    sources.source = lintian.package AND
    sources.version = lintian.package_version AND
    sources.release = 'sid'
WHERE tag = any($1::text[]) and package_type = 'source' AND vcs_type != ''
"""
        if packages is not None:
            query += " AND sources.source = any($2::text[])"
            args.append(tuple(packages))
        query += " ORDER BY sources.source, sources.version DESC"
        for row in await self._conn.fetch(query, *args):
            package_rows[row[0]] = row[:6]
            package_tags.setdefault((row[0], row[1]), []).append(row[6])
        args = [tuple(tags)]
        query = """\
SELECT DISTINCT ON (sources.source)
    sources.source,
    sources.version,
    sources.vcs_type,
    sources.vcs_url,
    sources.maintainer_email,
    sources.uploaders,
    lintian.tag
FROM
    lintian
INNER JOIN packages ON packages.package = lintian.package \
and packages.version = lintian.package_version \
inner join sources on sources.version = packages.version and \
sources.source = packages.source and sources.release = 'sid' \
where lintian.tag = any($1::text[]) and lintian.package_type = 'binary' \
and vcs_type != ''"""
        if packages is not None:
            query += " AND sources.source IN $2"
            args.append(tuple(packages))
        query += " ORDER BY sources.source, sources.version DESC"
        for row in await self._conn.fetch(query, *args):
            package_rows[row[0]] = row[:6]
            package_tags.setdefault((row[0], row[1]), []).append(row[6])
        package_values = package_rows.values()
        for row in package_values:
            yield (row[0], package_tags[row[0], row[1]])

    async def iter_lintian_fixes_candidates(
            self, packages, available_fixers):
        async for package, tags in self.iter_source_packages_by_lintian(
                available_fixers, packages if packages else None):
            if not (set(tags) - set(DEFAULT_ADDON_FIXERS)):
                value = DEFAULT_VALUE_LINTIAN_BRUSH_ADDON_ONLY
            else:
                value = DEFAULT_VALUE_LINTIAN_BRUSH
            for tag in tags:
                value += LINTIAN_BRUSH_TAG_VALUES.get(
                    tag, LINTIAN_BRUSH_TAG_DEFAULT_VALUE)
            context = ' '.join(sorted(tags))
            yield package, 'lintian-fixes', ['lintian-brush'], context, value

    async def iter_packages_with_new_upstream(self, packages=None):
        args = []
        query = """\
SELECT DISTINCT ON (sources.source)
sources.source, upstream.upstream_version FROM upstream \
INNER JOIN sources ON upstream.version = sources.version \
AND upstream.source = sources.source where \
status = 'newer package available' AND \
sources.vcs_url != '' AND \
sources.release = 'sid'
"""
        if packages is not None:
            query += " AND upstream.source = any($1::text[])"
            args.append(tuple(packages))
        query += " ORDER BY sources.source, sources.version DESC"
        for row in await self._conn.fetch(query, *args):
            yield (row[0], 'fresh-releases', ['new-upstream'], row[1],
                   DEFAULT_VALUE_NEW_UPSTREAM)

    async def iter_fresh_snapshots_candidates(self, packages):
        args = []
        query = """\
SELECT DISTINCT ON (sources.source)
sources.source from sources
where sources.vcs_url != '' and position('-' in sources.version) > 0 AND
sources.release = 'sid'
"""
        if packages is not None:
            query += " AND sources.source = any($1::text[])"
            args.append(tuple(packages))
        query += " ORDER BY sources.source, sources.version DESC"
        for row in await self._conn.fetch(query, *args):
            yield (row[0], 'fresh-snapshots', ['new-upstream', '--snapshot'],
                   None, DEFAULT_VALUE_NEW_UPSTREAM_SNAPSHOTS)

    async def iter_packages_with_metadata(self):
        query = """select distinct on (sources.source) sources.source,
        sources.maintainer_email, sources.uploaders, popcon_src.insts,
        sources.vcs_type, sources.vcs_url, sources.vcs_browser, sources.version
        from sources left join popcon_src on sources.source = popcon_src.source
        where sources.release = 'sid' order by sources.source, sources.version
        desc"""
        return await self._conn.fetch(query)


async def main():
    import argparse
    from breezy import trace
    from janitor import state
    from silver_platter.debian.lintian import (
        available_lintian_fixers,
        DEFAULT_ADDON_FIXERS,
    )
    from prometheus_client import (
        Counter,
        Gauge,
        push_to_gateway,
        REGISTRY,
    )

    parser = argparse.ArgumentParser(prog='candidates')
    parser.add_argument("packages", nargs='*')
    parser.add_argument(
        "--dry-run",
        help="Create branches but don't push or propose anything.",
        action="store_true", default=False)
    parser.add_argument('--prometheus', type=str,
                        help='Prometheus push gateway to export to.')
    args = parser.parse_args()

    last_success_gauge = Gauge(
        'job_last_success_unixtime',
        'Last time a batch job successfully finished')
    fixer_count = Counter(
        'fixer_count', 'Number of selected fixers.')

    tags = set()
    available_fixers = list(available_lintian_fixers())
    for fixer in available_fixers:
        tags.update(fixer.lintian_tags)
    fixer_count.inc(len(available_fixers))

    udd = await UDD.public_udd_mirror()
    packages = []
    for (
        name, maintainer_email, uploaders, insts, vcs_type, vcs_url,
        vcs_browser, sid_version) in await udd.iter_packages_with_metadata():
            uploader_emails = extract_uploader_emails(uploaders)

            try:
                branch_url = convert_debian_vcs_url(vcs_type, vcs_url)
            except ValueError as e:
                trace.note('%s: %s', name, e)
                branch_url = None

            packages.append((
                    name, branch_url, maintainer_email,
                    uploader_emails, sid_version,
                    vcs_type, vcs_url, vcs_browser, insts))
    await state.store_packages(packages)

    candidates = []

    async for (package, suite, command, context,
               value) in iter_lintian_fixes_candidates(
            args.packages, tags):
        candidates.append((
            package, suite, command, context, value))

    async for (package, suite, command, context,
               value) in iter_fresh_releases_candidates(args.packages):
        candidates.append((
            package, suite, command, context, value))

    async for (package, suite, command, context,
               value) in iter_fresh_snapshots_candidates(args.packages):
        candidates.append((
            package, suite, command, context, value))

    await state.store_candidates(candidates)

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        push_to_gateway(
            args.prometheus, job='janitor.schedule-new-upstream-snapshots',
            registry=REGISTRY)


if __name__ == '__main__':
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
