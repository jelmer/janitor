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
from google.protobuf import text_format  # type: ignore
import sys

from . import state, trace
from .config import read_config
from .candidates_pb2 import CandidateList


def iter_candidates_from_script(stdin):
    candidate_list = text_format.Parse(stdin.read(), CandidateList())
    for candidate in candidate_list.candidate:
        yield (
            candidate.package,
            candidate.suite,
            candidate.context,
            candidate.value,
            candidate.success_chance,
        )


async def main():
    import argparse
    from prometheus_client import (
        Gauge,
        push_to_gateway,
        REGISTRY,
    )

    parser = argparse.ArgumentParser(prog="candidates")
    parser.add_argument("packages", nargs="*")
    parser.add_argument(
        "--prometheus", type=str, help="Prometheus push gateway to export to."
    )
    parser.add_argument(
        "--config", type=str, default="janitor.conf", help="Path to configuration."
    )

    args = parser.parse_args()

    last_success_gauge = Gauge(
        "job_last_success_unixtime", "Last time a batch job successfully finished"
    )

    with open(args.config, "r") as f:
        config = read_config(f)

    db = state.Database(config.database_location)

    async with db.acquire() as conn:
        trace.note("Adding candidates.")
        candidates = [
            (package, suite, context, value, success_chance)
            for (
                package,
                suite,
                context,
                value,
                success_chance,
            ) in iter_candidates_from_script(sys.stdin)
        ]
        trace.note("Collected %d candidates.", len(candidates))
        await state.store_candidates(conn, candidates)

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        push_to_gateway(args.prometheus, job="janitor.candidates", registry=REGISTRY)


if __name__ == "__main__":
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
