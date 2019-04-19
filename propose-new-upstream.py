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
import os

from prometheus_client import (
    Gauge,
    push_to_gateway,
    REGISTRY,
)

import silver_platter   # noqa: F401

from breezy.trace import (
    note,
)


import sys
sys.path.insert(0, os.path.dirname(__file__))

from janitor.runner import (
    process_queue,
    get_open_mps_per_maintainer,
    open_proposal_count,
    )  # noqa: E402
from janitor.schedule import schedule_udd_new_upstreams  # noqa: E402

parser = argparse.ArgumentParser(prog='propose-new-upstream')
parser.add_argument("packages", nargs='*')
parser.add_argument("--policy",
                    help="Policy file to read.", type=str,
                    default='policy.conf')
parser.add_argument("--dry-run",
                    help="Create branches but don't push or propose anything.",
                    action="store_true", default=False)
parser.add_argument('--pre-check',
                    help='Command to run to check whether to process package.',
                    type=str)
parser.add_argument('--post-check',
                    help='Command to run to check package before pushing.',
                    type=str)
parser.add_argument('--build-command',
                    help='Build package to verify it.', type=str,
                    default='sbuild -v')
parser.add_argument('--shuffle',
                    help='Shuffle order in which packages are processed.',
                    action='store_true')
parser.add_argument('--refresh',
                    help='Discard old branch and apply fixers from scratch.',
                    action='store_true')
parser.add_argument('--log-dir',
                    help='Directory to store logs in.',
                    type=str, default='public_html/pkg')
parser.add_argument('--prometheus', type=str,
                    help='Prometheus push gateway to export to.')
parser.add_argument('--incoming', type=str,
                    help='Path to copy built Debian packages into.')
parser.add_argument(
    '--max-mps-per-maintainer',
    default=0,
    type=int, help='Maximum number of open merge proposals per maintainer.')
args = parser.parse_args()


last_success_gauge = Gauge(
    'job_last_success_unixtime',
    'Last time a batch job successfully finished')


if args.max_mps_per_maintainer or args.prometheus:
    open_mps_per_maintainer = get_open_mps_per_maintainer()
    for maintainer_email, count in open_mps_per_maintainer.items():
        open_proposal_count.labels(maintainer=maintainer_email).inc(count)
else:
    open_mps_per_maintainer = None


note('Querying UDD...')
todo = schedule_udd_new_upstreams(
    args.policy, args.packages, shuffle=args.shuffle)

process_queue(
    todo,
    max_mps_per_maintainer=args.max_mps_per_maintainer,
    open_mps_per_maintainer=open_mps_per_maintainer,
    refresh=args.refresh, pre_check=args.pre_check,
    build_command=args.build_command, post_check=args.post_check,
    dry_run=args.dry_run, incoming=args.incoming,
    output_directory=args.log_dir)

last_success_gauge.set_to_current_time()
if args.prometheus:
    push_to_gateway(args.prometheus, job='propose-new-upstreams',
                    registry=REGISTRY)
