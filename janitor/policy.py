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

import asyncpg
from email.utils import parseaddr
from fnmatch import fnmatch
import logging
from google.protobuf import text_format  # type: ignore
import re
from typing import List, TextIO, Tuple, Optional, Dict, Set

from . import policy_pb2


PolicyConfig = policy_pb2.PolicyConfig


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
    if match.HasField("in_base"):
        if match.in_base != in_base:
            return False
    if match.before_stage:
        if release_stages_passed is None:
            raise ValueError(
                'no release stages passed in, unable to match on before_stage')
        if match.before_stage in release_stages_passed:
            return False
    return True


def known_campaigns(config):
    ret = set()
    for policy in config.policy:
        for campaign in policy.campaign:
            ret.add(campaign.name)
    return ret


PUBLISH_MODE_STR = {
    policy_pb2.propose: "propose",
    policy_pb2.attempt_push: "attempt-push",
    policy_pb2.bts: "bts",
    policy_pb2.push: "push",
    policy_pb2.build_only: "build-only",
    policy_pb2.skip: "skip",
}


REVIEW_POLICY_STR = {
    policy_pb2.required: "required",
    policy_pb2.not_required: "not-required",
    None: None,
}


NOTIFY_MODE_STR = {
    policy_pb2.no_notification: "no_notification",
    policy_pb2.email: "email",
}


def apply_policy(
    config: policy_pb2.PolicyConfig,
    campaign: str,
    package_name: str,
    vcs_url: Optional[str],
    maintainer: str,
    uploaders: List[str],
    in_base: bool,
    release_stages_passed: Set[str]
) -> Tuple[Dict[str, Tuple[str, Optional[int]]], Optional[str], Optional[str], Optional[str]]:
    publish_mode = {}
    env = {}
    command = None
    qa_review = None
    broken_notify = policy_pb2.no_notification
    for policy in config.policy:
        if policy.match and not any(
            [
                matches(m, package_name, vcs_url, maintainer, uploaders, in_base, release_stages_passed)
                for m in policy.match
            ]
        ):
            continue
        for env_entry in policy.env:
            if env_entry.value is None:
                del env[env_entry.name]
            else:
                env[env_entry.name] = env_entry.value
        for s in policy.campaign:
            if s.name == campaign:
                break
        else:
            continue
        for publish in s.publish:
            publish_mode[publish.role] = (publish.mode, publish.max_frequency_days)
        if s.command:
            command = s.command
        if s.qa_review:
            qa_review = s.qa_review
        if s.HasField("broken_notify"):
            broken_notify = s.broken_notify
    if command is not None:
        command = ' '.join(
            ['%s=%s' % e for e in sorted(env.items())] +
            [command])
    assert publish_mode
    return (
        {k: (PUBLISH_MODE_STR[v[0]], v[1]) for (k, v) in publish_mode.items()},
        command,
        REVIEW_POLICY_STR[qa_review],
        NOTIFY_MODE_STR[broken_notify],
    )


async def read_release_stages(url: str) -> Set[str]:
    from aiohttp import ClientSession
    from datetime import datetime
    import yaml
    ret: Set[str] = set()
    async with ClientSession() as session:
        async with session.get(url) as resp:
            body = await resp.read()
            y = yaml.safe_load(body)
            for stage, data in y['stages'].items():
                if data['starts'] == 'TBA':
                    continue
                starts = datetime.fromisoformat(data['starts'][:-1])
                if datetime.utcnow() > starts:
                    ret.add(stage)
    return ret


async def update_policy(
    conn: asyncpg.Connection,
    name: str,
    campaign: str,
    publish_mode: Dict[str, Tuple[str, Optional[int]]],
    command: List[str],
    qa_review: Optional[str],
    broken_notify: Optional[str],
) -> None:
    await conn.execute(
        "INSERT INTO policy "
        "(package, suite, command, publish, qa_review, broken_notify) "
        "VALUES ($1, $2, $3, $4, $5, $6) "
        "ON CONFLICT (package, suite) DO UPDATE SET "
        "command = EXCLUDED.command, "
        "publish = EXCLUDED.publish, "
        "qa_review = EXCLUDED.qa_review, "
        "broken_notify = EXCLUDED.broken_notify",
        name,
        campaign,
        command,
        [(role, mode, max_freq) for (role, (mode, max_freq)) in publish_mode.items()],
        qa_review,
        broken_notify,
    )


async def iter_policy(conn: asyncpg.Connection, package: Optional[str] = None):
    query = "SELECT package, suite, publish, command, qa_review, broken_notify FROM policy"
    args = []
    if package:
        query += " WHERE package = $1"
        args.append(package)
    for row in await conn.fetch(query, *args):
        yield (
            row['package'],
            row['suite'],
            (
                {k[0]: (k[1], k[2]) for k in row['publish']},
                row['command'],
                row['qa_review'],
                row['broken_notify'],
            ),
        )


async def iter_packages(conn: asyncpg.Connection, package: Optional[str] = None):
    query = """
SELECT
  name,
  vcs_url,
  maintainer_email,
  uploader_emails,
  in_base
FROM
  package
WHERE
  NOT removed
"""
    args = []
    if package:
        query += " AND name = $1"
        args.append(package)
    return await conn.fetch(query, *args)


async def sync_policy(conn, policy, selected_package=None):
    current_policy = {}
    campaigns = known_campaigns(policy)
    if policy.freeze_dates_url:
        release_stages_passed = await read_release_stages(policy.freeze_dates_url)
        logging.info('Release stages passed: %r', release_stages_passed)
    else:
        release_stages_passed = None
    num_updated = 0
    logging.info('Creating current policy')
    async for (package, campaign, cur_pol) in iter_policy(conn, package=selected_package):
        current_policy[(package, campaign)] = cur_pol
    logging.info('Current policy: %d entries', len(current_policy))
    logging.info('Updating policy')
    for package in await iter_packages(conn, package=selected_package):
        updated = False
        for campaign in campaigns:
            intended_policy = apply_policy(
                policy,
                campaign,
                package['name'],
                package['vcs_url'],
                package['maintainer_email'],
                package['uploader_emails'],
                package['in_base'],
                release_stages_passed
            )
            stored_policy = current_policy.pop((package['name'], campaign), None)
            if stored_policy != intended_policy:
                logging.debug("%s/%s -> %r" % (package['name'], campaign, intended_policy))
                await update_policy(
                    conn, package['name'], campaign, *intended_policy
                )
                updated = True
        if updated:
            num_updated += 1
    await conn.executemany(
        'DELETE FROM policy WHERE package = $1 AND suite = $2',
        current_policy.keys())
    logging.info('Deleted %d from policy', len(current_policy.keys()))
    return num_updated


async def main(argv):
    import argparse
    from .config import read_config
    from . import state


    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--config", type=str, default="janitor.conf", help="Path to configuration."
    )
    parser.add_argument(
        "--policy",
        type=str,
        default="policy.conf",
        help="Path to policy configuration.",
    )
    parser.add_argument('--debug', action='store_true')
    parser.add_argument("--gcp-logging", action='store_true', help='Use Google cloud logging.')
    parser.add_argument('package', type=str, nargs='?')
    args = parser.parse_args(argv[1:])

    if args.gcp_logging:
        import google.cloud.logging
        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    elif args.debug:
        logging.basicConfig(level=logging.DEBUG)
    else:
        logging.basicConfig(level=logging.INFO)

    with open(args.policy, "r") as f:
        policy = read_policy(f)

    with open(args.config, "r") as f:
        config = read_config(f)

    db = state.Database(config.database_location)
    async with db.acquire() as conn:
        num_updated = await sync_policy(
            conn, policy, selected_package=args.package)
    logging.info('Updated policy for %d packages.', num_updated)


if __name__ == "__main__":
    import asyncio
    import sys
    asyncio.run(main(sys.argv))
