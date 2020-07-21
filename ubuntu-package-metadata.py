#!/usr/bin/python3
# Copyright (C) 2020 Jelmer Vernooij <jelmer@jelmer.uk>
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

"""Exporting of upstream metadata from Ubuntu."""

from janitor.package_metadata_pb2 import PackageList
from launchpadlib.launchpad import Launchpad
from launchpadlib.uris import LPNET_SERVICE_ROOT
from typing import List, Optional


async def main():
    import apt_pkg
    import argparse
    parser = argparse.ArgumentParser(prog='ubuntu-package-metadata')
    parser.add_argument("url", nargs='*')
    parser.add_argument(
        '--distroseries', type=str, default=None,
        help='Distribution series')
    parser.add_argument(
        '--difference-type', type=str,
        choices=['Unique to derived series', 'Different versions'],
        default='Unique to derived series',
        help='Only return differences of this type')
    args = parser.parse_args()

    lp = Launchpad.login_with(
        'debian-janitor', service_root=LPNET_SERVICE_ROOT, version='devel')

    ubuntu = lp.distributions['ubuntu']

    if args.distroseries:
        distroseries = ubuntu.series[args.distroseries]
    else:
        distroseries = ubuntu.current_series

    parentseries = distroseries.getParentSeries()[0]

    for sp in ubuntu.main_archive.getPublishedSources(
            status='Published', distro_series=distroseries):
        if 'ubuntu' not in sp.source_package_version:
            continue
        ps = parentseries.main_archive.getPublishedSources(
                source_name=sp.source_package_name, status='Published')
        pl = PackageList()
        if len(ps):
            removal = pl.removal.add()
            removal.name = sp.source_package_name
            removal.version = sp.source_package_version
        else:
            package = pl.package.add()
            package.name = sp.source_package_name
            package.vcs_type = 'Git'
            package.vcs_url = (
                'https://git.launchpad.net/ubuntu/+source/ubuntu-dev-tools '
                '-b ubuntu/devel')
            package.vcs_browser = (
                'https://code.launchpad.net/~usd-import-team/ubuntu/+source'
                '/%s/+git/%s/+ref/ubuntu/devel' % (package.name, package.name))
            package.archive_version = sp.source_package_version
            package.removed = False
        print(pl)


if __name__ == '__main__':
    import asyncio
    asyncio.run(main())
