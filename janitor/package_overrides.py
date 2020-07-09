#!/usr/bin/python3
# Copyright (C) 2019 Jelmer Vernooij <jelmer@jelmer.uk>
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

from typing import Optional

from google.protobuf import text_format  # type: ignore

import asyncio
import asyncpg

from . import package_overrides_pb2


def read_package_overrides(f):
    ret = {}
    config = text_format.Parse(
        f.read(), package_overrides_pb2.OverrideConfig())
    for override in config.package:
        ret[override.name] = override
    return ret


async def set_upstream_branch_url(
        conn: asyncpg.Connection, package: str, url: Optional[str]) -> None:
    await conn.execute(
        'insert into upstream (name, upstream_branch_url) values ($1, $2) '
        'on conflict (name) do update set '
        'upstream_branch_url = EXCLUDED.upstream_branch_url',
        package, url)


async def main(args):
    from .config import read_config
    from . import state
    from .schedule import do_schedule
    with open('package_overrides.conf', 'r') as f:
        overrides = read_package_overrides(f)

    with open(args.config, 'r') as f:
        config = read_config(f)

    db = state.Database(config.database_location)
    async with db.acquire() as conn:
        currents = {
            k: v for [k, v] in
            await state.iter_upstream_branch_urls(conn)}
        for name in set(currents).union(set(overrides)):
            current = currents.get(name)
            override = overrides.get(name)
            desired = (override.upstream_branch_url if override else None)
            if desired == current:
                continue
            await set_upstream_branch_url(conn, name, desired)
            print('Updating upstream branch URL for %s: %s' % (name, desired))
            if args.reschedule:
                await do_schedule(
                    conn, name, 'fresh-snapshots',
                    requestor='package overrides')


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument(
        '--config', type=str, default='janitor.conf',
        help='Path to configuration.')
    parser.add_argument(
        '--reschedule', action='store_true',
        help='Reschedule when updating.')
    args = parser.parse_args()
    asyncio.run(main(args))
