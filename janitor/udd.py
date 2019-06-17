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

from email.utils import parseaddr
import asyncpg

import distro_info


class PackageData(object):

    def __init__(self, name, version, vcs_type, vcs_url, maintainer_email,
                 uploader_emails):
        self.name = name
        self.version = version
        self.vcs_type = vcs_type
        self.vcs_url = vcs_url
        self.maintainer_email = maintainer_email
        self.uploader_emails = uploader_emails


async def connect_udd_mirror():
    """Connect to the public UDD mirror."""
    return await asyncpg.connect(
        database="udd",
        user="udd-mirror",
        password="udd-mirror",
        port=5432,
        host="udd-mirror.debian.net")


def extract_uploader_emails(uploaders):
    return ([parseaddr(p)[0] for p in uploaders.split(',')]
            if uploaders else [])


class UDD(object):

    @classmethod
    async def public_udd_mirror(cls):
        return cls(await connect_udd_mirror())

    def __init__(self, conn):
        self._conn = conn

    async def get_source_packages(self, packages, release=None):
        args = [tuple(packages)]
        query = (
            "SELECT DISTINCT ON (source) "
            "source, version, vcs_type, vcs_url, "
            "maintainer_email, uploaders "
            "FROM sources WHERE source = any($1::text[])")
        if release:
            query += " AND release = $2"
            args.append(release)
        query += " ORDER BY source, version DESC"
        uploader_emails = extract_uploader_emails(row[5])
        for row in await self._conn.fetch(query, *args):
            yield PackageData(
                    name=row[0], version=row[1], vcs_type=row[2],
                    vcs_url=row[3], maintainer_email=row[4],
                    uploader_emails=uploader_emails)

    async def iter_ubuntu_source_packages(self, packages=None, shuffle=False):
        # TODO(jelmer): Support shuffle
        if shuffle:
            raise NotImplementedError(self.iter_ubuntu_source_packages)
        release = distro_info.UbuntuDistroInfo().devel()
        query = """
SELECT
    DISTINCT ON (source)
    source, version, vcs_type, vcs_url, maintainer_email, uploaders
    FROM ubuntu_sources WHERE vcs_type != '' AND
release = $1 AND version LIKE '%%ubuntu%%' AND
NOT EXISTS (SELECT * FROM sources WHERE
source = ubuntu_sources.source)"""
        args = [release]
        if packages:
            query += " AND source IN $2"
            args.append(packages)
        query += """ ORDER BY source, version DESC"""
        for row in await self._conn.fetch(query, *args):
            uploader_emails = extract_uploader_emails(row[5])
            yield PackageData(
                name=row[0], version=row[1], vcs_type=row[2], vcs_url=row[3],
                maintainer_email=row[4],
                uploader_emails=uploader_emails)

    async def iter_source_packages_by_lintian(self, tags, packages=None,
                                              shuffle=False):
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
        if shuffle:
            package_values = list(package_values)
            import random
            random.shuffle(package_values)
        for row in package_values:
            uploader_emails = extract_uploader_emails(row[5])
            yield (PackageData(
                name=row[0], version=row[1], vcs_type=row[2], vcs_url=row[3],
                maintainer_email=row[4], uploader_emails=uploader_emails),
                package_tags[row[0], row[1]])

    async def iter_packages_with_new_upstream(self, packages=None):
        args = []
        query = """\
SELECT DISTINCT ON (sources.source)
sources.source, sources.version, sources.vcs_type, sources.vcs_url, \
sources.maintainer_email, sources.uploaders, \
upstream.upstream_version from upstream \
INNER JOIN sources on upstream.version = sources.version \
AND upstream.source = sources.source where \
status = 'newer package available' AND \
sources.vcs_url != '' \
"""
        if packages is not None:
            query += " AND upstream.source = any($1::text[])"
            args.append(tuple(packages))
        query += " ORDER BY sources.source, sources.version DESC"
        for row in await self._conn.fetch(query, *args):
            uploader_emails = extract_uploader_emails(row[5])
            yield PackageData(
                name=row[0], version=row[1], vcs_type=row[2], vcs_url=row[3],
                maintainer_email=row[4], uploader_emails=uploader_emails
                ), row[6]

    async def iter_source_packages_with_vcs(self, packages=None):
        args = []
        query = """\
SELECT DISTINCT ON (sources.source)
sources.source, sources.version, sources.vcs_type, sources.vcs_url,
sources.maintainer_email, sources.uploaders from sources
where sources.vcs_url != '' and position('-' in sources.version) > 0
"""
        if packages is not None:
            query += " AND sources.source = any($1::text[])"
            args.append(tuple(packages))
        query += " ORDER BY sources.source, sources.version DESC"
        for row in await self._conn.fetch(query, *args):
            uploader_emails = extract_uploader_emails(row[5])
            yield PackageData(
                name=row[0], version=row[1], vcs_type=row[2], vcs_url=row[3],
                maintainer_email=row[4], uploader_emails=uploader_emails
                )

    async def get_popcon_score(self, package):
        query = "SELECT insts FROM sources_popcon WHERE name = $1"
        row = await self._conn.fetchrow(query, package)
        if row:
            return row[0]
        return None

    async def binary_package_exists(self, package, suite=None):
        args = [package]
        query = "SELECT package FROM packages WHERE package = $1"
        if suite:
            query += " AND release = $2"
            args.append(suite)
        row = await self._conn.fetchrow(query, *args)
        return (row is not None)
