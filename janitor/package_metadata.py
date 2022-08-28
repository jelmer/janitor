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

import logging
from typing import List, Tuple, Sequence

from debian.changelog import Version
from google.protobuf import text_format  # type: ignore

from breezy.git.mapping import default_mapping

from silver_platter.debian import (
    convert_debian_vcs_url,
)
from . import state
from .config import read_config
from .package_metadata_pb2 import PackageList, PackageMetadata, PackageRemoval
from debmutate.vcs import (
    split_vcs_url,
    unsplit_vcs_url,
)
from upstream_ontologist.vcs import (
    find_public_repo_url,
)
from lintian_brush.vcs import (
    fixup_broken_git_url,
    canonicalize_vcs_url,
    determine_browser_url,
)


async def update_package_metadata(
    conn, distribution: str, provided_packages: List[PackageMetadata]
):
    logging.info("Updating package metadata.")
    packages = []
    codebases = []
    for package in provided_packages:
        vcs_last_revision = None
        vcs_url = package.vcs_url
        if package.vcs_type and package.vcs_type.capitalize() == "Git":
            new_vcs_url = fixup_broken_git_url(vcs_url)
            if new_vcs_url != vcs_url:
                logging.info("Fixing up VCS URL: %s -> %s", vcs_url, new_vcs_url)
                vcs_url = new_vcs_url
            if package.commit_id:
                vcs_last_revision = default_mapping.revision_id_foreign_to_bzr(
                    package.commit_id.encode("ascii")
                )

        if package.vcs_type:
            # Drop the subpath, we're storing it separately.
            (url, branch, subpath) = split_vcs_url(vcs_url)
            url = unsplit_vcs_url(url, branch)
            url = canonicalize_vcs_url(package.vcs_type, url)
            try:
                branch_url = convert_debian_vcs_url(package.vcs_type.capitalize(), url)
            except ValueError as e:
                logging.info("%s: %s", package.name, e)
                branch_url = None
            url = find_public_repo_url(url) or url
        else:
            subpath = None
            branch_url = None

        if vcs_url:
            vcs_browser = determine_browser_url(package.vcs_type, vcs_url)
        else:
            vcs_browser = None

        if vcs_browser is None and package.vcs_browser:
            vcs_browser = package.vcs_browser

        packages.append(
            (
                package.name,
                distribution,
                branch_url if branch_url else None,
                subpath if subpath else None,
                package.maintainer_email if package.maintainer_email else None,
                package.uploader_email if package.uploader_email else [],
                package.archive_version if package.archive_version else None,
                package.vcs_type.lower() if package.vcs_type else None,
                vcs_url,
                vcs_browser,
                vcs_last_revision.decode("utf-8") if vcs_last_revision else None,
                package.vcswatch_status.lower() if package.vcswatch_status else None,
                package.vcswatch_version if package.vcswatch_version else None,
                package.insts,
                package.removed,
                package.in_base,
                package.origin
            )
        )
        codebases.append(
            (
                package.name,
                branch_url if branch_url else None,
                subpath if subpath else None,
                package.vcs_type.lower() if package.vcs_type else None,
                vcs_last_revision.decode("utf-8") if vcs_last_revision else None,
            )
        )
    async with conn.transaction():
        await conn.executemany(
            "INSERT INTO package "
            "(name, distribution, branch_url, subpath, maintainer_email, "
            "uploader_emails, archive_version, vcs_type, vcs_url, vcs_browse, "
            "vcs_last_revision, vcswatch_status, vcswatch_version, popcon_inst, "
            "removed, in_base, origin) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, "
            "$13, $14, $15, $16, $17) ON CONFLICT (name, distribution) DO UPDATE SET "
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
            "removed = EXCLUDED.removed, "
            "in_base = EXCLUDED.in_base",
            packages,
        )
        await conn.executemany(
            "INSERT INTO codebase "
            "(name, branch_url, subpath, vcs_type, vcs_last_revision) "
            "VALUES ($1, $2, $3, $4, $5)"
            "ON CONFLICT (name) DO UPDATE SET "
            "branch_url = EXCLUDED.branch_url, subpath = EXCLUDED.subpath, "
            "vcs_type = EXCLUDED.vcs_type, "
            "vcs_last_revision = EXCLUDED.vcs_last_revision ",
            codebases
        )


async def mark_removed_packages(conn, distribution: str, removals: List[PackageRemoval]):
    existing_packages = set([
        row['name'] for row in await conn.fetch(
            "SELECT name FROM package WHERE NOT removed")])
    logging.info("Updating removals.")
    query = """\
UPDATE package SET removed = True
WHERE name = $1 AND distribution = $2 AND archive_version <= $3
"""
    await conn.executemany(
        query, [
            (removal.name, distribution, Version(removal.version) if removal.version else None)
            for removal in removals
            if removal.name in existing_packages])


def iter_packages_from_script(stdin) -> Tuple[Sequence[PackageMetadata], Sequence[PackageRemoval]]:
    package_list = text_format.Parse(stdin.read(), PackageList())
    return package_list.package, package_list.removal


async def main():
    import argparse
    import sys
    from aiohttp_openmetrics import (
        Gauge,
        push_to_gateway,
        REGISTRY,
    )

    parser = argparse.ArgumentParser(prog="package_metadata")
    parser.add_argument(
        "--prometheus", type=str, help="Prometheus push gateway to export to."
    )
    parser.add_argument(
        "--config", type=str, default="janitor.conf", help="Path to configuration."
    )

    parser.add_argument(
        "--distribution",
        type=str,
        default="unstable",
        help="Distribution to import metadata for.",
    )

    parser.add_argument("--gcp-logging", action='store_true', help='Use Google cloud logging.')

    args = parser.parse_args()
    if args.gcp_logging:
        import google.cloud.logging
        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    else:
        logging.basicConfig(level=logging.INFO)

    last_success_gauge = Gauge(
        "job_last_success_unixtime", "Last time a batch job successfully finished"
    )

    with open(args.config, "r") as f:
        config = read_config(f)

    logging.info('Reading data')
    packages, removals = iter_packages_from_script(sys.stdin)

    async with state.create_pool(config.database_location) as conn:
        logging.info(
            'Updating package data for %d packages',
            len(packages))
        await update_package_metadata(conn, args.distribution, packages)
        if removals:
            logging.info('Removing %d packages', len(removals))
            await mark_removed_packages(conn, args.distribution, removals)

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        await push_to_gateway(
            args.prometheus, job="janitor.package_metadata", registry=REGISTRY
        )


if __name__ == "__main__":
    import asyncio

    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
