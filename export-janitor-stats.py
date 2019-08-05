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
import sys

from prometheus_client import (
    Counter,
    Gauge,
    Histogram,
    generate_latest,
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

run_count = Counter(
    'run_count', 'Number of total runs.',
    labelnames=('command', ))
run_result_count = Counter(
    'run_result_count', 'Number of runs by code.',
    labelnames=('command', 'result_code'))
run_with_build_count = Counter(
    'run_with_build_count', 'Number of total runs with package built.',
    labelnames=('command', ))
run_with_proposal_count = Counter(
    'run_with_proposal_count', 'Number of total runs with merge proposal.',
    labelnames=('command', ))
duration = Histogram(
    'duration', 'Build duration',
    labelnames=('command', 'result_code'))
never_processed_count = Counter(
    'never_processed_count', 'Number of items never processed.',
    labelnames=('suite', ))
last_success_gauge = Gauge(
    'job_last_success_unixtime',
    'Last time a batch job successfully finished')

loop = asyncio.get_event_loop()

for package_name, command, result_code, log_id, description, run_duration in (
        loop.run_until_complete(state.iter_last_runs())):
    run_count.labels(command=command).inc()
    run_result_count.labels(command=command, result_code=result_code).inc()
    duration.labels(
        command=command,
        result_code=result_code).observe(run_duration.total_seconds())

for suite, count in loop.run_until_complete(state.get_never_processed()):
    never_processed_count.label(suite).set(count)


last_success_gauge.set_to_current_time()
if args.prometheus:
    push_to_gateway(
        args.prometheus, job='janitor.export-stats',
        registry=REGISTRY)
else:
    sys.stdout.buffer.write(generate_latest())
