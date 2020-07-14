#!/usr/bin/python3

from janitor import state


async def gather_package_list(conn, suite):
    present = await state.iter_published_packages(conn, suite)

    for source, build_version, archive_version in sorted(present):
        yield (source, build_version, archive_version)


async def write_apt_repo(conn, suite):
    return  {
        'packages': gather_package_list(conn, suite),
        'suite': suite,
        }
