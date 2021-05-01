#!/usr/bin/python3
# Copyright (C) 2021 Jelmer Vernooij <jelmer@jelmer.uk>
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

import argparse
import asyncio
import logging

from ognibuild.buildlog import problem_to_upstream_requirement
from ognibuild.debian.apt import AptManager
from ognibuild.session.plain import PlainSession
from buildlog_consultant import problem_clses

from janitor import state
from janitor.candidates import store_candidates
from janitor.config import read_config
from janitor.debian.missing_deps import NewPackage, UpdatePackage, resolve_requirement
from janitor.schedule import do_schedule, full_command
from janitor.policy import sync_policy, read_policy

DEFAULT_NEW_PACKAGE_PRIORITY = 150
DEFAULT_UPDATE_PACKAGE_PRIORITY = 150
DEFAULT_SUCCESS_CHANCE = 0.5


def reconstruct_problem(result_code, failure_details):
    kind = result_code
    for prefix in ['build-', 'post-build-', 'dist-', 'install-deps-']:
        if kind.startswith(prefix):
            kind = kind[len(prefix):]
    try:
        return problem_clses[kind].from_json(failure_details)
    except KeyError:
        return None


async def gather_requirements(db, session, run_ids=None):
    async with db.acquire() as conn:
        query = """
SELECT package, suite, result_code, failure_details FROM last_unabsorbed_runs WHERE result_code != 'success' AND failure_details IS NOT NULL
"""
        args = []
        if run_ids:
            query += " AND id = ANY($1::text[])"
            args.append(run_ids)
        for row in await conn.fetch(query, *args):
            problem = reconstruct_problem(row['result_code'], row['failure_details'])
            requirement = problem_to_upstream_requirement(problem)
            if requirement is None:
                continue
            yield row['package'], row['suite'], requirement


async def schedule_new_package(conn, upstream_info, policy, requestor=None, origin=None):
    from debmutate.vcs import unsplit_vcs_url
    package = upstream_info['name'].replace('/', '-') + '-upstream'
    logging.info(
        "Creating new upstream %s => %s",
        package, upstream_info['branch_url'])
    vcs_url = unsplit_vcs_url(upstream_info['branch_url'], None, upstream_info['subpath'])
    await conn.execute(
        "INSERT INTO package (name, distribution, branch_url, subpath, maintainer_email, origin, vcs_url) "
        "VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING",
        package, 'upstream', upstream_info['branch_url'], '',
        'dummy@example.com', origin, vcs_url)
    await store_candidates(
        conn,
        [(package, 'debianize', None, DEFAULT_NEW_PACKAGE_PRIORITY,
          DEFAULT_SUCCESS_CHANCE)])
    await sync_policy(conn, policy, package=package)
    policy = await conn.fetchrow(
        "SELECT update_changelog, command "
        "FROM policy WHERE package = $1 AND suite = $2",
        package, 'debianize')
    command = policy['command']
    if upstream_info['version']:
        command += ' --upstream-version=%s' % upstream_info['version']
    command = full_command(policy['update_changelog'], command)
    await do_schedule(conn, package, "debianize", requestor=requestor, bucket='missing-deps', command=command)


async def schedule_update_package(conn, policy, package, desired_version, requestor=None):
    logging.info('Scheduling new run for %s/fresh-releases', package)
    # TODO(jelmer): Do something with desired_version
    # TODO(jelmer): fresh-snapshots?
    await conn.execute(
        "INSERT INTO candidate "
        "(package, suite, context, value, success_chance) "
        "VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO IGNORE",
        package, 'fresh-releases', None, DEFAULT_UPDATE_PACKAGE_PRIORITY,
        DEFAULT_SUCCESS_CHANCE)
    await sync_policy(conn, policy, package=package)
    await do_schedule(conn, package, "fresh-releases", requestor=requestor, bucket='missing-deps')


async def followup_missing_requirement(conn, apt_mgr, policy, requirement, needed_by=None):
    requestor = 'schedule-missing-deps'
    if needed_by is not None:
        origin = 'dependency of %s' % needed_by
    else:
        origin = None
    actions = await resolve_requirement(apt_mgr, requirement)
    logging.debug('%s: %r', requirement, actions)
    if actions == []:
        # We don't know what to do
        logging.info('Unable to find actions for requirement %r', requirement)
        return False
    if actions == [[]]:
        # We don't need to do anything - could retry things that need this?
        return False
    if isinstance(actions[0][0], NewPackage):
        if needed_by:
            requestor += ' (needed by %s)' % needed_by
        await schedule_new_package(
            conn, actions[0][0].upstream_info.json(), policy,
            requestor=requestor, origin=origin)
    elif isinstance(actions[0][0], UpdatePackage):
        if needed_by:
            requestor += ' (%s needed by %s)' % (actions[0][0].desired_version, needed_by)
        await schedule_update_package(
            conn, policy, actions[0][0].name, actions[0][0].desired_version,
            requestor=requestor)
    else:
        raise NotImplementedError('unable to deal with %r' % actions[0][0])
    return True


async def main():
    parser = argparse.ArgumentParser("reschedule")
    parser.add_argument(
        "--config", type=str, default="janitor.conf", help="Path to configuration."
    )
    parser.add_argument(
        "--policy", type=str, default="policy.conf", help="Path to policy."
    )
    parser.add_argument(
        "-r", dest="run_id", type=str, help="Run to process.", action="append"
    )
    parser.add_argument('--debug', action='store_true')


    args = parser.parse_args()
    with open(args.config, "r") as f:
        config = read_config(f)

    if args.debug:
        logging.basicConfig(level=logging.DEBUG)
    else:
        logging.basicConfig(level=logging.INFO)

    db = state.Database(config.database_location)
    session = PlainSession()
    with session:
        requirements = {}
        async for package, suite, requirement in gather_requirements(db, session, args.run_id or None):
            requirements.setdefault(requirement, []).append((package, suite))

        with open(args.policy, "r") as f:
            policy = read_policy(f)

        apt_mgr = AptManager.from_session(session)

        async with db.acquire() as conn:
            for requirement, needed_by in requirements.items():
                await followup_missing_requirement(
                    conn, apt_mgr, policy, requirement,
                    needed_by=', '.join(["%s/%s" % (package, suite) for (package, suite) in needed_by]))

if __name__ == '__main__':
    asyncio.run(main())
