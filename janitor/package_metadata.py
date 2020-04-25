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

"""Importing of upstream metadata from UDD."""

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
from lintian_brush.vcs import (
    split_vcs_url,
    fixup_broken_git_url,
    canonicalize_vcs_url,
    unsplit_vcs_url,
    )
from .udd import UDD


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


async def iter_packages_with_metadata(udd, packages=None):
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
    for row in await udd.fetch(query, *args):
        yield row


async def iter_removals(udd, packages=None):
    query = """\
select name, version from package_removal where 'source' = any(arch_array)
"""
    args = []
    if packages:
        query += " and name = ANY($1::text[])"
        args.append(packages)
    return await udd.fetch(query, *args)


async def update_package_metadata(
        db, udd, package_overrides, selected_packages=None):
    async with db.acquire() as conn:
        existing_packages = {
            package.name: package
            for package in await state.iter_packages(conn)}

        removals = {}
        for name, version in await iter_removals(
                udd, packages=selected_packages):
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
                   vcswatch_version) in iter_packages_with_metadata(
                       udd, selected_packages):
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
    from janitor.package_overrides import read_package_overrides
    from prometheus_client import (
        Gauge,
        push_to_gateway,
        REGISTRY,
    )

    parser = argparse.ArgumentParser(prog='candidates')
    parser.add_argument("packages", nargs='*')
    parser.add_argument('--prometheus', type=str,
                        help='Prometheus push gateway to export to.')
    parser.add_argument(
        '--config', type=str, default='janitor.conf',
        help='Path to configuration.')

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

    udd = await UDD.public_udd_mirror()

    db = state.Database(config.database_location)

    await update_package_metadata(
        db, udd, package_overrides, args.packages)

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        push_to_gateway(args.prometheus, job='janitor.package_metadata', registry=REGISTRY)


if __name__ == '__main__':
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
