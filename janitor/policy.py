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
from google.protobuf import text_format  # type: ignore
import re
from typing import List, TextIO, Tuple, Optional, Dict

from . import policy_pb2


def read_policy(f: TextIO) -> policy_pb2.PolicyConfig:
    return text_format.Parse(f.read(), policy_pb2.PolicyConfig())


def matches(match, package_name, vcs_url, package_maintainer, package_uploaders, in_base, release_stages_passed):
    package_maintainer_email = parseaddr(package_maintainer)[1]
    for maintainer in match.maintainer:
        if not fnmatch(package_maintainer_email, maintainer):
            return False
    package_uploader_emails = [
        parseaddr(uploader)[1] for uploader in (package_uploaders or [])
    ]
    for uploader in match.uploader:
        if not any([fnmatch(u, uploader) for u in package_uploader_emails]):
            return False
    for source_package in match.source_package:
        if not fnmatch(package_name, source_package):
            return False
    for vcs_url_regex in match.vcs_url_regex:
        if vcs_url is None or not re.fullmatch(vcs_url_regex, vcs_url):
            return False
    if match.in_base is not None:
        if match.in_base != in_base:
            return False
    if match.stage is not None:
        if match.stage.before in release_stages_passed.get(
                match.stage.codename, []):
            return False
    return True


def known_suites(config):
    ret = set()
    for policy in config.policy:
        for suite in policy.suite:
            ret.add(suite.name)
    return ret


PUBLISH_MODE_STR = {
    policy_pb2.propose: "propose",
    policy_pb2.attempt_push: "attempt-push",
    policy_pb2.bts: "bts",
    policy_pb2.push: "push",
    policy_pb2.build_only: "build-only",
}


POLICY_MODE_STR = {
    policy_pb2.auto: "auto",
    policy_pb2.update_changelog: "update",
    policy_pb2.leave_changelog: "leave",
}


def apply_policy(
    config: policy_pb2.PolicyConfig,
    suite: str,
    package_name: str,
    vcs_url: Optional[str],
    maintainer: str,
    uploaders: List[str],
    in_base: bool,
    release_stages_passed: Dict[str, List[str]]
) -> Tuple[Dict[str, str], str, List[str]]:
    publish_mode = {}
    update_changelog = policy_pb2.auto
    command = None
    for policy in config.policy:
        if policy.match and not any(
            [
                matches(m, package_name, vcs_url, maintainer, uploaders, in_base, release_stages_passed)
                for m in policy.match
            ]
        ):
            continue
        if policy.HasField("changelog") is not None:
            update_changelog = policy.changelog
        for s in policy.suite:
            if s.name == suite:
                break
        else:
            continue
        for publish in s.publish:
            publish_mode[publish.role] = publish.mode
        if s.command:
            command = s.command
    return (
        {k: PUBLISH_MODE_STR[v] for (k, v) in publish_mode.items()},
        POLICY_MODE_STR[update_changelog],
        shlex.split(command),
    )


async def read_release_stages(urls):
    from aiohttp import ClientSession
    from datetime import date, datetime
    import yaml
    ret = {}
    async with ClientSession() as session:
        for url in urls:
            async with session.get(url) as resp:
                body = await resp.read()
                y = yaml.safe_load(body)
                codename = y['codename']
                for stage, data in y['stages'].items():
                    if data['starts'] == 'TBA':
                        continue
                    starts = datetime.fromisoformat(data['starts'][:-1])
                    if datetime.now() > starts:
                        ret.setdefault(codename, []).append(stage)
    return ret


async def main(args):
    from .config import read_config
    from . import state
    from .debian import state as debian_state

    with open(args.policy, "r") as f:
        policy = read_policy(f)

    release_stages_passed = await read_release_stages(args.freeze_dates)

    suites = known_suites(policy)

    with open(args.config, "r") as f:
        config = read_config(f)

    current_policy = {}
    db = state.Database(config.database_location)
    async with db.acquire() as conn:
        async for (package, suite, cur_pol) in state.iter_policy(conn):
            current_policy[(package, suite)] = cur_pol
        for package in await debian_state.iter_packages(conn):
            for suite in suites:
                intended_policy = apply_policy(
                    policy,
                    suite,
                    package.name,
                    package.vcs_url,
                    package.maintainer_email,
                    package.uploader_emails,
                    package.in_base,
                    release_stages_passed
                )
                stored_policy = current_policy.get((package.name, suite))
                if stored_policy != intended_policy:
                    print("%s/%s -> %r" % (package.name, suite, intended_policy))
                    await state.update_policy(
                        conn, package.name, suite, *intended_policy
                    )


if __name__ == "__main__":
    import argparse
    import asyncio

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--config", type=str, default="janitor.conf", help="Path to configuration."
    )
    parser.add_argument(
        "--freeze-dates", action="append",
        type=str, help="Read freeze dates from URL.")
    parser.add_argument(
        "--policy",
        type=str,
        default="policy.conf",
        help="Path to policy configuration.",
    )
    args = parser.parse_args()
    asyncio.run(main(args))
