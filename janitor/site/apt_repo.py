#!/usr/bin/python3

import argparse
import asyncio
import sys

from debian.changelog import Version

from janitor import state
from janitor.site import env


async def get_unstable_versions(present):
    unstable = {}
    if present:
        async for package in state.iter_sources_with_unstable_version(
                packages=list(present)):
            unstable[package.name] = Version(package.version)
    return unstable


async def gather_package_list(suite):
    present = {}
    for source, version in await state.iter_published_packages(suite):
        present[source] = Version(version)

    unstable = await get_unstable_versions(present)

    ret = []
    for source in sorted(present):
        ret.append((
            source,
            present[source].upstream_version,
            unstable[source].upstream_version
            if source in unstable else ''))
    return ret


async def write_apt_repo(suite):
    template = env.get_template(suite + '.html')
    return await template.render_async(
        packages=await gather_package_list(suite))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(prog='report-apt-repo')
    parser.add_argument("suite")
    args = parser.parse_args()

    loop = asyncio.get_event_loop()
    sys.stdout.write(loop.run_until_complete(write_apt_repo(args.suite)))
