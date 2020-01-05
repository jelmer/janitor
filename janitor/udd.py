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
from debian.changelog import Version
from email.utils import parseaddr
import asyncpg

from silver_platter.debian import (
    convert_debian_vcs_url,
)
from . import state, trace
from .config import read_config
from silver_platter.debian.lintian import (
    DEFAULT_ADDON_FIXERS,
    )
from lintian_brush.vcs import (
    split_vcs_url,
    fixup_broken_git_url,
    canonicalize_vcs_url,
    unsplit_vcs_url,
    )

DEFAULT_VALUE_UNCHANGED = 20
DEFAULT_VALUE_ORPHAN = 60
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

DEFAULT_VALUE_MULTIARCH_HINT = 50
MULTIARCH_HINTS_VALUE = {
    'ma-foreign': 20,
    'file-conflict': 50,
    'ma-foreign-library': 20,
    'dep-any': 20,
    'ma-same': 20,
    'arch-all': 20,
}


def estimate_lintian_fixes_value(tags):
    if not (set(tags) - set(DEFAULT_ADDON_FIXERS)):
        value = DEFAULT_VALUE_LINTIAN_BRUSH_ADDON_ONLY
    else:
        value = DEFAULT_VALUE_LINTIAN_BRUSH
    for tag in tags:
        value += LINTIAN_BRUSH_TAG_VALUES.get(
            tag, LINTIAN_BRUSH_TAG_DEFAULT_VALUE)
    return value


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
        if not uploader:
            continue
        email = parseaddr(uploader)[1]
        if not email:
            continue
        ret.append(email)
    return ret


class UDD(object):

    @classmethod
    async def public_udd_mirror(cls):
        return cls(await connect_udd_mirror())

    def __init__(self, conn):
        self._conn = conn

    async def iter_lintian_fixes_candidates(self, packages, available_fixers):
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
        args = [tuple(available_fixers)]
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
            query += " AND sources.source = ANY($2::text[])"
            args.append(tuple(packages))
        query += " ORDER BY sources.source, sources.version DESC"
        for row in await self._conn.fetch(query, *args):
            package_rows[row[0]] = row[:6]
            package_tags.setdefault((row[0], row[1]), []).append(row[6])
        package_values = package_rows.values()
        for row in package_values:
            tags = sorted(package_tags[row[0], row[1]])
            value = estimate_lintian_fixes_value(tags)
            context = ' '.join(sorted(tags))
            yield (row[0], context, value, None)

    async def iter_unchanged_candidates(self, packages=None):
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
        async with self._conn.transaction():
            async for row in self._conn.cursor(query, *args):
                yield (row[0], None, DEFAULT_VALUE_UNCHANGED, None)

    async def iter_orphan_candidates(self, packages=None):
        args = []
        query = """\
SELECT DISTINCT ON (sources.source) sources.source, now() - orphaned_time, bug
FROM sources
JOIN orphaned_packages ON orphaned_packages.source = sources.source
WHERE sources.vcs_url != '' AND sources.release = 'sid' AND
orphaned_packages.type in ('O') AND
(sources.uploaders != '' OR
 sources.maintainer != 'Debian QA Group <packages@qa.debian.org>')
"""
        if packages is not None:
            query += " AND sources.source = any($1::text[])"
            args.append(tuple(packages))
        async with self._conn.transaction():
            async for row in self._conn.cursor(query, *args):
                yield (row[0], str(row[2]), DEFAULT_VALUE_ORPHAN, None)

    async def iter_fresh_releases_candidates(self, packages=None):
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
        async with self._conn.transaction():
            async for row in self._conn.cursor(query, *args):
                yield (row[0], row[1], DEFAULT_VALUE_NEW_UPSTREAM, None)

    async def iter_fresh_snapshots_candidates(self, packages):
        args = []
        query = """\
SELECT DISTINCT ON (sources.source)
sources.source, exists (
    select from upstream_metadata where
    key = 'Repository' and source = sources.source)
from sources
where sources.vcs_url != '' and position('-' in sources.version) > 0 AND
sources.release = 'sid'
"""
        if packages is not None:
            query += " AND sources.source = any($1::text[])"
            args.append(tuple(packages))
        query += " ORDER BY sources.source, sources.version DESC"
        async with self._conn.transaction():
            async for row in self._conn.cursor(query, *args):
                yield (
                    row[0], None, DEFAULT_VALUE_NEW_UPSTREAM_SNAPSHOTS,
                    1.0 if row[1] else 0.1)

    async def iter_packages_with_metadata(self, packages=None):
        args = []
        query = """
select distinct on (sources.source) sources.source,
    sources.maintainer_email, sources.uploaders, popcon_src.insts,
    coalesce(vcswatch.vcs, sources.vcs_type),
    coalesce(vcswatch.url, sources.vcs_url),
    vcswatch.branch,
    coalesce(vcswatch.browser, sources.vcs_browser),
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
        async with self._conn.transaction():
            async for row in self._conn.cursor(query, *args):
                yield row

    async def iter_removals(self, packages=None):
        query = """\
select name, version from package_removal where 'source' = any(arch_array)
"""
        args = []
        if packages:
            query += " and name = ANY($1::text[])"
            args.append(packages)
        return await self._conn.fetch(query, *args)


async def iter_multiarch_fixes(packages=None):
    from lintian_brush.multiarch_hints import (
        download_multiarch_hints,
        parse_multiarch_hints,
        multiarch_hints_by_source,
        )
    with download_multiarch_hints() as f:
        hints = parse_multiarch_hints(f)
        bysource = multiarch_hints_by_source(hints)
    for source, entries in bysource.items():
        if packages is not None and source not in packages:
            continue
        hints = [entry['link'].rsplit('#', 1)[-1] for entry in entries]
        value = sum(map(MULTIARCH_HINTS_VALUE.__getitem__, hints)) + (
            DEFAULT_VALUE_MULTIARCH_HINT)
        yield source, ' '.join(sorted(hints)), value, None


async def update_package_metadata(
        db, udd, package_overrides, selected_packages=None):
    async with db.acquire() as conn:
        existing_packages = {
            package.name: package
            for package in await state.iter_packages(conn)}

        removals = {}
        for name, version in await udd.iter_removals(
                packages=selected_packages):
            if name not in removals:
                removals[name] = Version(version)
            else:
                removals[name] = max(Version(version), removals[name])

        if not selected_packages:
            trace.note('Updating removals.')
            filtered_removals = [
                (name, version) for (name, version) in removals.items()
                if name in existing_packages and
                not existing_packages[name].removed]
            await state.update_removals(conn, filtered_removals)

        trace.note('Updating package metadata.')
        packages = []
        async for (name, maintainer_email, uploaders, insts, vcs_type, vcs_url,
                   vcs_branch, vcs_browser, vcswatch_status, sid_version,
                   vcswatch_version) in udd.iter_packages_with_metadata(
                       selected_packages):
            try:
                override = package_overrides[name]
            except KeyError:
                upstream_branch_url = None
            else:
                vcs_url = override.branch_url or vcs_url
                upstream_branch_url = override.upstream_branch_url

            uploader_emails = extract_uploader_emails(uploaders)

            if vcs_type and vcs_type.capitalize() == 'Git':
                new_vcs_url = fixup_broken_git_url(vcs_url)
                if new_vcs_url != vcs_url:
                    trace.note('Fixing up VCS URL: %s -> %s',
                               vcs_url, new_vcs_url)
                    vcs_url = new_vcs_url

            if vcs_url and vcs_branch:
                (repo_url, orig_branch, subpath) = split_vcs_url(vcs_url)
                if orig_branch != vcs_branch:
                    new_vcs_url = unsplit_vcs_url(
                        repo_url, vcs_branch, subpath)
                    trace.note('Fixing up branch name from vcswatch: %s -> %s',
                               vcs_url, new_vcs_url)
                    vcs_url = new_vcs_url

            if vcs_type is not None:
                # Drop the subpath, we're storing it separately.
                (url, branch, subpath) = split_vcs_url(vcs_url)
                url = unsplit_vcs_url(url, branch)
                url = canonicalize_vcs_url(vcs_type, url)
                try:
                    branch_url = convert_debian_vcs_url(
                        vcs_type.capitalize(), url)
                except ValueError as e:
                    trace.note('%s: %s', name, e)
                    branch_url = None
            else:
                subpath = None
                branch_url = None

            if name not in removals:
                removed = False
            else:
                removed = Version(sid_version) <= removals[name]

            packages.append((
                name, branch_url, subpath, maintainer_email, uploader_emails,
                sid_version, vcs_type, vcs_url, vcs_browser,
                vcswatch_status.lower() if vcswatch_status else None,
                vcswatch_version, insts, removed, upstream_branch_url))
            await state.store_packages(conn, packages)


async def main():
    import argparse
    from janitor import state
    from janitor.package_overrides import read_package_overrides
    from silver_platter.debian.lintian import (
        available_lintian_fixers,
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
    parser.add_argument(
        '--config', type=str, default='janitor.conf',
        help='Path to configuration.')

    parser.add_argument(
        '--suite', type=str, action='append',
        help='Suite to retrieve candidates for.')
    parser.add_argument(
        '--skip-package-metadata', action='store_true',
        help='Skip updating of package information.')
    parser.add_argument(
        '--package-overrides', type=str, default='package_overrides.conf',
        help='Read package overrides.')

    args = parser.parse_args()

    last_success_gauge = Gauge(
        'job_last_success_unixtime',
        'Last time a batch job successfully finished')
    fixer_count = Counter(
        'fixer_count', 'Number of selected fixers.')

    with open(args.config, 'r') as f:
        config = read_config(f)

    with open(args.package_overrides, 'r') as f:
        package_overrides = read_package_overrides(f)

    tags = set()
    available_fixers = list(available_lintian_fixers())
    for fixer in available_fixers:
        tags.update(fixer.lintian_tags)
    fixer_count.inc(len(available_fixers))

    udd = await UDD.public_udd_mirror()

    db = state.Database(config.database_location)

    if not args.skip_package_metadata:
        await update_package_metadata(
            db, udd, package_overrides, args.packages)

    async with db.acquire() as conn:
        CANDIDATE_FNS = [
            ('unchanged', udd.iter_unchanged_candidates(
                args.packages or None)),
            ('lintian-fixes',
             udd.iter_lintian_fixes_candidates(args.packages or None, tags)),
            ('fresh-releases',
             udd.iter_fresh_releases_candidates(args.packages or None)),
            ('fresh-snapshots',
             udd.iter_fresh_snapshots_candidates(args.packages or None)),
            ('multiarch-fixes',
             iter_multiarch_fixes(args.packages or None)),
            ('orphan',
             udd.iter_orphan_candidates(args.packages or None))
            ]

        for suite, candidate_fn in CANDIDATE_FNS:
            if args.suite and suite not in args.suite:
                continue
            trace.note('Adding candidates for %s.', suite)
            candidates = [
                (package, suite, context, value, success_chance)
                async for (package, context, value, success_chance)
                in candidate_fn]
            trace.note('Collected %d candidates for %s.',
                       len(candidates), suite)
            await state.store_candidates(conn, candidates)

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        push_to_gateway(args.prometheus, job='janitor.udd', registry=REGISTRY)


if __name__ == '__main__':
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
