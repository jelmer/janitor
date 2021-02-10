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

"""Importing of upstream metadata."""

from __future__ import absolute_import

from google.protobuf import text_format  # type: ignore
from typing import List, Tuple

from . import state, trace
from .config import read_config
from .codebase_metadata_pb2 import CodebaseList, CodebaseMetadata


async def update_codebase_metadata(conn, provided_codebases):
    trace.note("Updating codebase metadata.")
    codebases = []
    for codebase in provided_codebases:
        codebases.append((codebase.name, codebase.branch_url, codebase.subpath))
    await conn.executemany(
        "INSERT INTO upstream_codebase (name, branch_url, subpath) "
        "VALUES ($1, $2, $3)"
        "ON CONFLICT (name) DO UPDATE SET "
        "branch_url = EXCLUDED.branch_url, "
        "subpath = EXCLUDED.subpath",
        codebases,
    )


def iter_codebases_from_script(stdin) -> Tuple[List[CodebaseMetadata]]:
    codebase_list = text_format.Parse(stdin.read(), CodebaseList())
    return codebase_list.codebase


async def main():
    import argparse
    import sys
    from prometheus_client import (
        Gauge,
        push_to_gateway,
        REGISTRY,
    )

    parser = argparse.ArgumentParser(prog="codebase_metadata")
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

    codebases = iter_codebases_from_script(sys.stdin)

    async with db.acquire() as conn:
        await update_codebase_metadata(conn, codebases)

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        push_to_gateway(
            args.prometheus, job="janitor.codebase_metadata", registry=REGISTRY
        )


if __name__ == "__main__":
    import asyncio

    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
