#!/usr/bin/python3

from janitor.debian import state as debian_state


async def gather_package_list(conn, suite):
    present = await debian_state.iter_published_packages(conn, suite)

    for source, build_version, archive_version in sorted(present):
        yield (source, build_version, archive_version)
