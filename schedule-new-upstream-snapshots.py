#!/usr/bin/python3
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


import argparse
import asyncio
import os

from prometheus_client import (
    Gauge,
    push_to_gateway,
    REGISTRY,
)

import silver_platter   # noqa: F401

import sys
sys.path.insert(0, os.path.dirname(__file__))

from janitor.schedule import (
    schedule_udd_new_upstream_snapshots,
    add_to_queue,
)  # noqa: E402
from janitor.trace import (
    note,
)  # noqa: E402

parser = argparse.ArgumentParser(prog='propose-new-upstream')
parser.add_argument("packages", nargs='*')
parser.add_argument("--policy",
                    help="Policy file to read.", type=str,
                    default='policy.conf')
parser.add_argument("--dry-run",
                    help="Create branches but don't push or propose anything.",
                    action="store_true", default=False)
parser.add_argument('--shuffle',
                    help='Shuffle order in which packages are processed.',
                    action='store_true')
parser.add_argument('--default-offset', default=10, type=int,
                    help='Default offset.')
parser.add_argument('--prometheus', type=str,
                    help='Prometheus push gateway to export to.')
args = parser.parse_args()


last_success_gauge = Gauge(
    'job_last_success_unixtime',
    'Last time a batch job successfully finished')


note('Querying UDD...')
loop = asyncio.get_event_loop()


SUITE = 'fresh-snapshots'


async def main():
    todo = [x async for x in schedule_udd_new_upstream_snapshots(
            args.policy, args.packages, shuffle=args.shuffle)]
    await add_to_queue(
        todo, SUITE, dry_run=args.dry_run,
        default_offset=args.default_offset)

loop.run_until_complete(main())

last_success_gauge.set_to_current_time()
if args.prometheus:
    push_to_gateway(
        args.prometheus, job='janitor.schedule-new-upstream-snapshots',
        registry=REGISTRY)
