#!/usr/bin/python3
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

from email.utils import parseaddr
from fnmatch import fnmatch
import shlex
from google.protobuf import text_format

from . import policy_pb2


def read_policy(f):
    return text_format.Parse(f.read(), policy_pb2.PolicyConfig())


def matches(match, package_name, package_maintainer, package_uploaders):
    package_maintainer_email = parseaddr(package_maintainer)[1]
    for maintainer in match.maintainer:
        if not fnmatch(package_maintainer_email, maintainer):
            return False
    package_uploader_emails = [
        parseaddr(uploader)[1]
        for uploader in (package_uploaders or [])]
    for uploader in match.uploader:
        if not any([fnmatch(u, uploader) for u in package_uploader_emails]):
            return False
    for source_package in match.source_package:
        if not fnmatch(package_name, source_package):
            return False
    return True


def known_suites(config):
    ret = set()
    for policy in config.policy:
        for suite in policy.suite:
            ret.add(suite.name)
    return ret


def apply_policy(config, suite, package_name, maintainer, uploaders):
    mode = policy_pb2.skip
    update_changelog = policy_pb2.auto
    command = None
    for policy in config.policy:
        if (policy.match and
                not any([matches(m, package_name, maintainer, uploaders)
                         for m in policy.match])):
            continue
        if policy.HasField('changelog') is not None:
            update_changelog = policy.changelog
        for s in policy.suite:
            if s.name == suite:
                break
        else:
            continue
        if s.HasField('mode'):
            mode = s.mode
        if s.command:
            command = s.command
    return (
        {policy_pb2.propose: 'propose',
         policy_pb2.attempt_push: 'attempt-push',
         policy_pb2.push: 'push',
         policy_pb2.skip: 'skip',
         policy_pb2.build_only: 'build-only',
         }[mode],
        {policy_pb2.auto: 'auto',
         policy_pb2.update_changelog: 'update',
         policy_pb2.leave_changelog: 'leave',
         }[update_changelog],
        shlex.split(command))


async def main(args):
    from .config import read_config
    from . import state

    with open('policy.conf', 'r') as f:
        policy = read_policy(f)

    suites = known_suites(policy)

    with open(args.config, 'r') as f:
        config = read_config(f)

    current_policy = {}
    db = state.Database(config.database_location)
    async with db.acquire() as conn:
        async for (package, suite, cur_pol) in state.iter_publish_policy(conn):
            current_policy[(package, suite)] = cur_pol
        for package in await state.iter_packages(conn):
            for suite in suites:
                package_policy = apply_policy(
                    policy, suite, package.name, package.maintainer_email,
                    package.uploader_emails)
                if current_policy.get((package.name, suite)) != package_policy:
                    print('%s/%s -> %r' % (
                        package.name, suite, package_policy))
                    await state.update_publish_policy(
                        conn, package.name, suite, *package_policy)


if __name__ == '__main__':
    import argparse
    import asyncio
    parser = argparse.ArgumentParser()
    parser.add_argument(
        '--config', type=str, default='janitor.conf',
        help='Path to configuration.')
    args = parser.parse_args()
    asyncio.run(main(args))
