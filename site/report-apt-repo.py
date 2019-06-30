#!/usr/bin/python3

import argparse
import asyncio
import os
import sys

from debian.changelog import Version

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state, udd  # noqa: E402
from janitor.site import env  # noqa: E402

parser = argparse.ArgumentParser(prog='report-apt-repo')
parser.add_argument("suite")
args = parser.parse_args()


async def get_unstable_versions(present):
    unstable = {}
    if present:
        conn = await udd.UDD.public_udd_mirror()
        async for package in conn.get_source_packages(
                packages=list(present), release='sid'):
            unstable[package.name] = Version(package.version)
    return unstable


async def gather_package_list():
    present = {}
    for source, version in await state.iter_published_packages(args.suite):
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
    sys.stdout.write(
        await template.render_async(packages=await gather_package_list()))

loop = asyncio.get_event_loop()
loop.run_until_complete(write_apt_repo(args.suite))
