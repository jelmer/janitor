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
import sys

from prometheus_client import (
    Gauge,
    push_to_gateway,
    REGISTRY,
)

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402


parser = argparse.ArgumentParser(prog='refresh-proposal-status')
parser.add_argument(
    '--prometheus', type=str,
    help='Prometheus push gateway to export to.')
args = parser.parse_args()

run_count = Gauge(
    'run_count', 'Number of total runs.')
run_with_build_count = Gauge(
    'run_with_build_count', 'Number of total runs with package built.',
    labelnames=('suite', ))
run_with_proposal_count = Gauge(
    'run_with_proposal_count', 'Number of total runs with merge proposal.')
last_success_gauge = Gauge(
    'job_last_success_unixtime',
    'Last time a batch job successfully finished')


for (run_id, (start_time, finish_time), command, description, package_name,
        merge_proposal_url, build_version,
        build_distribution) in state.iter_runs():
    run_count.inc()
    if build_version:
        run_with_build_count.labels(suite=build_distribution).inc()
    if merge_proposal_url:
        run_with_proposal_count.inc()


last_success_gauge.set_to_current_time()
if args.prometheus:
    push_to_gateway(
        args.prometheus, job='janitor.export-stats',
        registry=REGISTRY)
