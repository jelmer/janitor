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

import logging
from typing import List, Tuple, Sequence

from debian.changelog import Version
from google.protobuf import text_format  # type: ignore

from breezy.git.mapping import default_mapping

from breezy import urlutils

from . import state
from .config import read_config
from .package_metadata_pb2 import PackageList, PackageMetadata, PackageRemoval


async def update_package_metadata(
    conn, distribution: str, provided_packages: Sequence[PackageMetadata]
):
    logging.info("Updating package metadata.")
    packages = []
    codebases = []
    for package in provided_packages:
        vcs_last_revision = None
        if package.vcs_type and package.vcs_type.lower() == "git":
            if package.commit_id:
                vcs_last_revision = default_mapping.revision_id_foreign_to_bzr(
                    package.commit_id.encode("ascii")
                )
        if package.repository_url and package.branch:
            branch_url = urlutils.join_segments_parameters(
                package.repository_url.rstrip('/'),
                {'branch': urlutils.escape(package.branch)})
        else:
            branch_url = package.repository_url

        packages.append(
            (
                package.name,
                distribution,
                branch_url if branch_url else None,
                package.subpath if package.subpath else None,
                package.maintainer_email if package.maintainer_email else None,
                package.uploader_email if package.uploader_email else [],
                package.archive_version if package.archive_version else None,
                package.vcs_type.lower() if package.vcs_type else None,
                package.browse_url,
                vcs_last_revision.decode("utf-8") if vcs_last_revision else None,
                package.removed,
                package.in_base,
                package.origin,
                package.name if branch_url is not None else None,
            )
        )
        if branch_url is not None:
            codebases.append((
                package.name,
                branch_url,
                package.repository_url,
                package.branch,
                package.subpath,
                package.vcs_type.lower() if package.vcs_type else None,
                vcs_last_revision.decode("utf-8") if vcs_last_revision else None,
                package.value))
    await conn.executemany(
        "INSERT INTO codebase "
        "(name, branch_url, url, branch, subpath, vcs_type, "
        "vcs_last_revision, value) "
        "VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        "ON CONFLICT (name) DO UPDATE SET "
        "branch_url = EXCLUDED.branch_url, subpath = EXCLUDED.subpath, "
        "vcs_type = EXCLUDED.vcs_type, "
        "vcs_last_revision = EXCLUDED.vcs_last_revision, "
        "value = EXCLUDED.value, url = EXCLUDED.url, branch = EXCLUDED.branch",
        codebases
    )
    await conn.executemany(
        "INSERT INTO package "
        "(name, distribution, branch_url, subpath, maintainer_email, "
        "uploader_emails, archive_version, vcs_type, vcs_browse, "
        "vcs_last_revision, "
        "removed, in_base, origin, codebase) "
        "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, "
        "$13, $14) "
        "ON CONFLICT (name, distribution) DO UPDATE SET "
        "branch_url = EXCLUDED.branch_url, "
        "subpath = EXCLUDED.subpath, "
        "maintainer_email = EXCLUDED.maintainer_email, "
        "uploader_emails = EXCLUDED.uploader_emails, "
        "archive_version = EXCLUDED.archive_version, "
        "vcs_type = EXCLUDED.vcs_type, "
        "vcs_last_revision = EXCLUDED.vcs_last_revision, "
        "vcs_browse = EXCLUDED.vcs_browse, "
        "removed = EXCLUDED.removed, "
        "in_base = EXCLUDED.in_base, "
        "codebase = EXCLUDED.codebase",
        packages,
    )


async def mark_removed_packages(
        conn, distribution: str, removals: Sequence[PackageRemoval]):
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
            (removal.name, distribution, Version(removal.version)
                if removal.version else None)
            for removal in removals
            if removal.name in existing_packages])


def iter_packages_from_script(stdin) -> Tuple[
        Sequence[PackageMetadata], Sequence[PackageRemoval]]:
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

    parser.add_argument(
        "--gcp-logging", action='store_true', help='Use Google cloud logging.')

    parser.add_argument(
        "--remove-unmentioned",
        action="store_true",
        help="Mark packages not included in input as removed.")

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

    async with await state.create_pool(config.database_location) as db, \
            db.acquire() as conn,\
            conn.transaction():
        if args.remove_unmentioned:
            referenced_packages = set(package.name for package in packages)
            await conn.fetch(
                'UPDATE package SET removed = True WHERE NOT (name = ANY($1::text[]))',
                referenced_packages)

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
