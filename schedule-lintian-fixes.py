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
    Counter,
    Gauge,
    push_to_gateway,
    REGISTRY,
)

import silver_platter   # noqa: F401
from silver_platter.debian.lintian import (
    available_lintian_fixers,
    DEFAULT_ADDON_FIXERS,
)

import sys
sys.path.insert(0, os.path.dirname(__file__))

from janitor.candidate import iter_lintian_fixes_candidates  # noqa: E402
from janitor.schedule import (
    add_to_queue,
    schedule_from_candidates,
    )  # noqa: E402
from janitor.trace import (
    note,
)  # noqa: E402

parser = argparse.ArgumentParser(prog='propose-lintian-fixes')
parser.add_argument("packages", nargs='*')
parser.add_argument("--fixers",
                    help="Fixers to run.", type=str, action='append')
parser.add_argument("--policy",
                    help="Policy file to read.", type=str,
                    default='policy.conf')
parser.add_argument("--dry-run",
                    help="Create branches but don't push or propose anything.",
                    action="store_true", default=False)
parser.add_argument('--propose-addon-only',
                    help='Fixers that should be considered add-on-only.',
                    type=str, action='append',
                    default=DEFAULT_ADDON_FIXERS)
parser.add_argument('--prometheus', type=str,
                    help='Prometheus push gateway to export to.')
args = parser.parse_args()


fixer_count = Counter(
    'fixer_count', 'Number of selected fixers.')
last_success_gauge = Gauge(
    'job_last_success_unixtime',
    'Last time a batch job successfully finished')


fixer_scripts = {}
for fixer in available_lintian_fixers():
    for tag in fixer.lintian_tags:
        fixer_scripts[tag] = fixer

available_fixers = set(fixer_scripts)
if args.fixers:
    available_fixers = available_fixers.intersection(set(args.fixers))

fixer_count.inc(len(available_fixers))


async def main():
    note('Querying UDD...')
    iter_candidates = iter_lintian_fixes_candidates(
        args.packages, available_fixers, args.propose_addon_only)
    todo = [x async for x in schedule_from_candidates(
        args.policy, iter_candidates)]
    await add_to_queue(todo, dry_run=args.dry_run)

loop = asyncio.get_event_loop()
loop.run_until_complete(main())

last_success_gauge.set_to_current_time()
if args.prometheus:
    push_to_gateway(
        args.prometheus, job='janitor.schedule-lintian-fixes',
        registry=REGISTRY)
