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
import asyncpg
import logging
import sys
from typing import List, Optional, Tuple

from google.protobuf import text_format  # type: ignore

from . import state
from .config import read_config
from .candidates_pb2 import CandidateList


def iter_candidates_from_script(stdin):
    candidate_list = text_format.Parse(stdin.read(), CandidateList())
    for candidate in candidate_list.candidate:
        yield (
            candidate.package,
            candidate.suite,
            candidate.command,
            candidate.context,
            candidate.value,
            candidate.success_chance,
        )


async def store_candidates(
        conn: asyncpg.Connection,
        entries: List[Tuple[str, str, Optional[str], Optional[str], Optional[str], Optional[int],
                            Optional[float]]]):
    """Store candidates.

    Args:
      conn: Database connection
      entries: List of tuples with
        (package, campaign, change_set, conext, value, success_chance)
    """
    await conn.executemany(
        "INSERT INTO candidate "
        "(package, suite, command, change_set, context, value, success_chance) "
        "VALUES ($1, $2, $3, $4, $5, $6, $7) "
        "ON CONFLICT (package, suite, coalesce(change_set, ''::text)) "
        "DO UPDATE SET context = EXCLUDED.context, value = EXCLUDED.value, "
        "success_chance = EXCLUDED.success_chance, command = EXCLUDED.command",
        entries,
    )


async def main():
    import argparse
    from aiohttp_openmetrics import (
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
    parser.add_argument("--gcp-logging", action='store_true', help='Use Google cloud logging.')

    args = parser.parse_args()

    if args.gcp_logging:
        import google.cloud.logging
        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    else:
        logging.basicConfig(level=logging.INFO)

    last_success_gauge = Gauge(
        "job_last_success_unixtime", "Last time a batch job successfully finished"
    )

    with open(args.config, "r") as f:
        config = read_config(f)

    campaign_names = [campaign.name for campaign in config.campaign]

    async with state.create_pool(config.database_location) as pool, pool.acquire() as conn:
        known_packages = set()
        async with conn.transaction():
            async for record in conn.cursor('SELECT name FROM package'):
                known_packages.add(record[0])

        logging.info("Adding candidates.")
        proposed_candidates = [
            (package, suite, command, None, context, value, success_chance)
            for (
                package,
                suite,
                command,
                context,
                value,
                success_chance,
            ) in iter_candidates_from_script(sys.stdin)
        ]
        logging.info("Collected %d candidates.", len(proposed_candidates))
        candidates = []
        for entry in proposed_candidates:
            package = entry[0]
            if package not in known_packages:
                logging.warning(
                    'ignoring candidate %s/%s; package unknown',
                    package, entry[1])
                continue
            if entry[1] not in campaign_names:
                logging.warning('unknown suite %r', entry[1])
                continue
            candidates.append(entry)
        await store_candidates(conn, candidates)

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        await push_to_gateway(
            args.prometheus, job="janitor.candidates", registry=REGISTRY)


if __name__ == "__main__":
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
