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

"""Import candidates."""

import asyncio
from google.protobuf import text_format

from . import state, trace
from .config import read_config
from .candidates_pb2 import CandidateList


async def iter_candidates_from_script(args):
    p = await asyncio.create_subprocess_exec(
        *args, stdout=asyncio.subprocess.PIPE,
        stdin=asyncio.subprocess.PIPE)
    (stdout, unused_stderr) = await p.communicate()
    candidate_list = text_format.Parse(stdout, CandidateList())
    for candidate in candidate_list.candidate:
        yield (candidate.package, candidate.value, candidate.context,
               candidate.success_chance)


async def main():
    import argparse
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
        '--suite', type=str, action='append',
        help='Suite to retrieve candidates for.')

    args = parser.parse_args()

    last_success_gauge = Gauge(
        'job_last_success_unixtime',
        'Last time a batch job successfully finished')

    with open(args.config, 'r') as f:
        config = read_config(f)

    db = state.Database(config.database_location)

    async with db.acquire() as conn:
        CANDIDATE_SCRIPTS = [
            ('unchanged', './unchanged-candidates.py'),
            ('lintian-fixes', './lintian-fixes-candidates.py'),
            ('fresh-releases', './fresh-releases-candidates.py')
            ('fresh-snapshots', './fresh-snapshots-candidates.py'),
            ('multiarch-fixes', './multi-arch-candidates.py'),
            ('orphan', './orphan-candidates.py'),
            ('uncommitted', './uncommitted-candidates.py'),
            ]

        for suite, script in CANDIDATE_SCRIPTS:
            if args.suite and suite not in args.suite:
                continue
            trace.note('Adding candidates for %s.', suite)
            candidates = [
                (package, suite, context, value, success_chance)
                async for (package, context, value, success_chance)
                in iter_candidates_from_script([script] + args.packages)]
            trace.note('Collected %d candidates for %s.',
                       len(candidates), suite)
            await state.store_candidates(conn, candidates)

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        push_to_gateway(
            args.prometheus, job='janitor.candidates', registry=REGISTRY)


if __name__ == '__main__':
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
