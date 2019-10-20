#!/usr/bin/python3

from debian.changelog import Version

from janitor import state
from janitor.site import env


async def get_unstable_versions(conn, present):
    unstable = {}
    if present:
        for package, version in await state.iter_sources_with_unstable_version(
                conn, packages=list(present)):
            unstable[package] = Version(version)
    return unstable


async def gather_package_list(conn, suite):
    present = {}
    for source, version in await state.iter_published_packages(
            conn, suite):
        present[source] = Version(version)

    unstable = await get_unstable_versions(conn, present)

    for source in sorted(present):
        yield (
            source,
            present[source].upstream_version,
            unstable[source].upstream_version
            if source in unstable else '')


async def write_apt_repo(conn, suite):
    template = env.get_template(suite + '.html')
    return await template.render_async(
        packages=gather_package_list(conn, suite),
        suite=suite)
