#!/usr/bin/python3

from janitor import state
from janitor.site import env


async def gather_package_list(conn, suite):
    present = await state.iter_published_packages(conn, suite)

    for source, build_version, archive_version in sorted(present):
        yield (source, build_version, archive_version)


async def write_apt_repo(conn, suite):
    template = env.get_template(suite + '.html')
    return await template.render_async(
        packages=gather_package_list(conn, suite),
        suite=suite)
