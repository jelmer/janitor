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

"""Importing of upstream metadata."""

from __future__ import absolute_import

from debian.changelog import Version
from google.protobuf import text_format  # type: ignore
from typing import List, Optional

from breezy.git.mapping import default_mapping

from silver_platter.debian import (
    convert_debian_vcs_url,
)
from . import state, trace
from .config import read_config
from .package_metadata_pb2 import PackageList
from debmutate.vcs import (
    split_vcs_url,
    unsplit_vcs_url,
    )
from lintian_brush.vcs import (
    fixup_broken_git_url,
    canonicalize_vcs_url,
    )


async def update_package_metadata(
        conn, distribution: str, provided_packages, package_overrides):
    trace.note('Updating package metadata.')
    packages = []
    for package in provided_packages:
        try:
            override = package_overrides[package.name]
        except KeyError:
            vcs_url = package.vcs_url
        else:
            vcs_url = override.branch_url or package.vcs_url or None

        vcs_last_revision = None

        if package.vcs_type and package.vcs_type.capitalize() == 'Git':
            new_vcs_url = fixup_broken_git_url(vcs_url)
            if new_vcs_url != vcs_url:
                trace.note('Fixing up VCS URL: %s -> %s',
                           vcs_url, new_vcs_url)
                vcs_url = new_vcs_url
            if package.commit_id:
                vcs_last_revision = (
                    default_mapping.revision_id_foreign_to_bzr(
                        package.commit_id.encode('ascii')))

        if vcs_url and package.vcs_branch:
            (repo_url, orig_branch, subpath) = split_vcs_url(vcs_url)
            if orig_branch != package.vcs_branch:
                new_vcs_url = unsplit_vcs_url(
                    repo_url, package.vcs_branch, subpath)
                trace.note('Fixing up branch name from vcswatch: %s -> %s',
                           vcs_url, new_vcs_url)
                vcs_url = new_vcs_url

        if package.vcs_type:
            # Drop the subpath, we're storing it separately.
            (url, branch, subpath) = split_vcs_url(vcs_url)
            url = unsplit_vcs_url(url, branch)
            url = canonicalize_vcs_url(package.vcs_type, url)
            try:
                branch_url = convert_debian_vcs_url(
                    package.vcs_type.capitalize(), url)
            except ValueError as e:
                trace.note('%s: %s', package.name, e)
                branch_url = None
        else:
            subpath = None
            branch_url = None

        packages.append((
            package.name, distribution, branch_url if branch_url else None,
            subpath if subpath else None,
            package.maintainer_email if package.maintainer_email else None,
            package.uploader_email if package.uploader_email else [],
            package.archive_version if package.archive_version else None,
            package.vcs_type if package.vcs_type else None, vcs_url,
            package.vcs_browser if package.vcs_browser else None,
            vcs_last_revision.decode('utf-8') if vcs_last_revision else None,
            package.vcswatch_status.lower()
                if package.vcswatch_status else None,
            package.vcswatch_version if package.vcswatch_version else None,
            package.insts, package.removed))
    await conn.executemany(
        "INSERT INTO package "
        "(name, distribution, branch_url, subpath, maintainer_email, "
        "uploader_emails, archive_version, vcs_type, vcs_url, vcs_browse, "
        "vcs_last_revision, vcswatch_status, vcswatch_version, popcon_inst, "
        "removed) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, "
        "$13, $14, $15) ON CONFLICT (name, distribution) DO UPDATE SET "
        "branch_url = EXCLUDED.branch_url, "
        "subpath = EXCLUDED.subpath, "
        "maintainer_email = EXCLUDED.maintainer_email, "
        "uploader_emails = EXCLUDED.uploader_emails, "
        "archive_version = EXCLUDED.archive_version, "
        "vcs_type = EXCLUDED.vcs_type, "
        "vcs_url = EXCLUDED.vcs_url, "
        "vcs_last_revision = EXCLUDED.vcs_last_revision, "
        "vcs_browse = EXCLUDED.vcs_browse, "
        "vcswatch_status = EXCLUDED.vcswatch_status, "
        "vcswatch_version = EXCLUDED.vcswatch_version, "
        "popcon_inst = EXCLUDED.popcon_inst, "
        "removed = EXCLUDED.removed",
        packages)


async def mark_removed_packages(conn, distribution: str, removals):
    existing_packages = {
        package.name: package
        for package in await state.iter_packages(conn)}
    trace.note('Updating removals.')
    filtered_removals = [
        (removal.name, Version(removal.version) if removal.version else None)
        for removal in removals
        if removal.name in existing_packages and
        not existing_packages[removal.name].removed]
    await state.update_removals(conn, distribution, filtered_removals)


def iter_packages_from_script(stdin):
    package_list = text_format.Parse(stdin.read(), PackageList())
    return package_list.package, package_list.removal


async def main():
    import argparse
    import sys
    from janitor.package_overrides import read_package_overrides
    from prometheus_client import (
        Gauge,
        push_to_gateway,
        REGISTRY,
    )

    parser = argparse.ArgumentParser(prog='package_metadata')
    parser.add_argument('--prometheus', type=str,
                        help='Prometheus push gateway to export to.')
    parser.add_argument(
        '--config', type=str, default='janitor.conf',
        help='Path to configuration.')

    parser.add_argument(
        '--distribution', type=str, default='unstable',
        help='Distribution to import metadata for.')

    parser.add_argument(
        '--package-overrides', type=str, default='package_overrides.conf',
        help='Read package overrides.')

    args = parser.parse_args()

    last_success_gauge = Gauge(
        'job_last_success_unixtime',
        'Last time a batch job successfully finished')

    with open(args.config, 'r') as f:
        config = read_config(f)

    with open(args.package_overrides, 'r') as f:
        package_overrides = read_package_overrides(f)

    db = state.Database(config.database_location)

    packages, removals = iter_packages_from_script(sys.stdin)

    async with db.acquire() as conn:
        await update_package_metadata(conn, args.distribution, packages, package_overrides)
        if removals:
            await mark_removed_packages(conn, args.distribution, removals)

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        push_to_gateway(args.prometheus, job='janitor.package_metadata',
                        registry=REGISTRY)


if __name__ == '__main__':
    import asyncio
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
