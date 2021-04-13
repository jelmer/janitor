#!/usr/bin/python3

import apt_pkg
import argparse
import asyncio
from dataclasses import dataclass
from datetime import datetime, timedelta
from debian.changelog import Version
import logging
import re
from typing import List, Tuple, Dict, Optional, Union
from janitor import state
from janitor.candidates import store_candidates
from janitor.config import read_config
from janitor.schedule import do_schedule
from janitor.policy import sync_policy, read_policy
from janitor.udd import UDD
from ognibuild import Requirement
from ognibuild.buildlog import problem_to_upstream_requirement
from ognibuild.debian.apt import AptManager
from ognibuild.resolver.apt import resolve_requirement_apt
from ognibuild.session.plain import PlainSession
from buildlog_consultant import problem_clses
from lintian_brush.debianize import find_upstream, UpstreamInfo

parser = argparse.ArgumentParser("reschedule")
parser.add_argument(
    "--config", type=str, default="janitor.conf", help="Path to configuration."
)
parser.add_argument(
    "--policy", type=str, default="policy.conf", help="Path to policy."
)
parser.add_argument(
    "-r", type=str, help="Run to process."
)


args = parser.parse_args()
with open(args.config, "r") as f:
    config = read_config(f)


DEFAULT_NEW_PACKAGE_PRIORITY = 150
DEFAULT_SUCCESS_CHANCE = 0.5


def recreate_problem(kind, details):
    try:
        return problem_clses[kind](**details)
    except KeyError:
        return None


async def gather_requirements(db, session, run_ids=None):
    async with db.acquire() as conn:
        query = """
SELECT result_code, failure_details FROM last_unabsorbed_runs WHERE result_code != 'success' AND failure_details IS NOT NULL
"""
        args = []
        if run_ids:
            query += " WHERE id = ANY($1::text[])"
            args.append(run_ids)
        for row in await conn.fetch(query, *args):
            kind = row['result_code']
            for prefix in ['build-', 'post-build-', 'dist-']:
                if kind.startswith(prefix):
                    kind = kind[len(prefix):]
            problem = recreate_problem(kind, row['failure_details'])
            if problem is None:
                continue
            requirement = problem_to_upstream_requirement(problem)
            if requirement is None:
                continue
            yield requirement


@dataclass
class NewPackage:

    upstream_info: UpstreamInfo


@dataclass
class UpdatePackage:

    name: str
    desired_version: Optional[Version] = None


async def resolve_requirement(conn, requirement: Requirement) -> List[List[Union[NewPackage, UpdatePackage]]]:
    apt_opts = resolve_requirement_apt(apt_mgr, requirement)
    options = []
    if apt_opts:
        for apt_req in apt_opts:
            option: Optional[List[Union[NewPackage, UpdatePackage]]] = []
            for entry in apt_req.relations:
                for r in entry:
                    versions = apt_mgr.package_versions(r['name'])
                    if not versions:
                        upstream = find_upstream(apt_req)
                        if upstream:
                            option.append(NewPackage(upstream))
                        else:
                            option = None
                            break
                    else:
                        if not r.get('version'):
                            logging.debug('package already available: %s', r['name'])
                        elif r['version'][0] == '>=':
                            depcache = apt_pkg.DepCache(apt_mgr.apt_cache._cache)
                            depcache.init()
                            version = depcache.get_candidate_ver(apt_mgr.apt_cache._cache[r['name']])
                            if not version:
                                logging.warning(
                                    'unable to find source package matching %s', r['name'])
                                option = None
                                break
                            file, index = version.file_list.pop(0)
                            records = apt_pkg.PackageRecords(apt_mgr.apt_cache._cache)
                            records.lookup((file, index))
                            option.append(UpdatePackage(records.source_pkg, r['version'][1]))
                        else:
                            logging.warning("don't know what to do with constraint %r", r['version'])
                            option = None
                            break
                if option is None:
                    break
            if option == []:
                return [[]]
            if option is not None:
                options.append(option)
    else:
        upstream = find_upstream(requirement)
        if upstream:
            options.append([NewPackage(upstream)])

    return options


async def followup_missing_requirement(conn, policy, requirement):
    actions = await resolve_requirement(conn, requirement)
    logging.debug('%s: %r', requirement, actions)
    if actions == []:
        # We don't know what to do
        logging.info('Unable to find actions for requirement %r', requirement)
        return False
    if actions == [[]]:
        # We don't need to do anything - could retry things that need this?
        return False
    if isinstance(actions[0][0], NewPackage):
        package = actions[0][0].upstream_info.name.replace('/', '-') + '-upstream'
        logging.info(
            "Creating new upstream %s => %s",
            package, actions[0][0].upstream_info.branch_url)
        await conn.execute(
            "INSERT INTO package (name, distribution, branch_url, subpath, maintainer_email) "
            "VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
            package, 'upstream', actions[0][0].upstream_info.branch_url, '',
            'dummy@example.com')
        await store_candidates(
            conn,
            [(package, 'debianize', None, DEFAULT_NEW_PACKAGE_PRIORITY,
              DEFAULT_SUCCESS_CHANCE)])
        await sync_policy(conn, policy, package=package)
        await do_schedule(conn, package, "debianize", requestor='schedule-missing-deps')
    elif isinstance(actions[0][0], UpdatePackage):
        logging.info('Scheduling new run for %s/fresh-releases', actions[0][0].name)
        # TODO(jelmer): fresh-snapshots?
        await do_schedule(conn, actions[0][0].name, "fresh-releases", requestor='schedule-missing-deps')
    else:
        raise NotImplementedError('unable to deal with %r' % actions[0][0])
    return True


async def main(db, session):
    requirements = []
    async for requirement in gather_requirements(db, session):
        if requirement not in requirements:
            requirements.append(requirement)

    with open(args.policy, "r") as f:
        policy = read_policy(f)

    async with db.acquire() as conn:
        for requirement in requirements:
            await followup_missing_requirement(conn, policy, requirement)


logging.basicConfig(level=logging.INFO)

db = state.Database(config.database_location)
session = PlainSession()
with session:
    apt_mgr = AptManager.from_session(session)
    asyncio.run(main(db, session))
