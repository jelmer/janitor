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
from google.protobuf import text_format

from . import policy_pb2


def read_policy(f):
    return text_format.Parse(f.read(), policy_pb2.PolicyConfig())


def matches(match, package_name, package_maintainer, package_uploaders):
    for maintainer in match.maintainer:
        if maintainer != parseaddr(package_maintainer)[1]:
            return False
    package_uploader_emails = [
        parseaddr(uploader)[1]
        for uploader in (package_uploaders or [])]
    for uploader in match.uploader:
        if uploader not in package_uploader_emails:
            return False
    for source_package in match.source_package:
        if source_package != package_name:
            return False
    return True


def apply_policy(config, suite, package_name, maintainer, uploaders):
    field = suite.replace('-', '_')
    mode = policy_pb2.skip
    update_changelog = 'auto'
    for policy in config.policy:
        if (policy.match and
                not any([matches(m, package_name, maintainer, uploaders)
                         for m in policy.match])):
            continue
        if field is not None and policy.HasField(field):
            mode = getattr(policy, field)
        if policy.changelog is not None:
            update_changelog = policy.changelog
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
         }[update_changelog])


async def main(args):
    from .config import read_config
    from . import state, SUITES

    with open('policy.conf', 'r') as f:
        policy = read_policy(f)

    with open(args.config, 'r') as f:
        config = read_config(f)

    db = state.Database(config.database_location)
    async with db.acquire() as conn:
        for package in await state.iter_packages(conn):
            for suite in SUITES:
                publish_mode, changelog_mode = apply_policy(
                    policy, suite, package.name, package.maintainer_email,
                    package.uploader_emails)
                await state.update_publish_policy(
                    conn, package.name, suite, publish_mode, changelog_mode)


if __name__ == '__main__':
    import argparse
    import asyncio
    parser = argparse.ArgumentParser()
    parser.add_argument(
        '--config', type=str, default='janitor.conf',
        help='Path to configuration.')
    args = parser.parse_args()
    asyncio.run(main(args))
