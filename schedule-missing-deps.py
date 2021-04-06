#!/usr/bin/python3

import argparse
import asyncio
from dataclasses import dataclass
from datetime import datetime, timedelta
from debian.changelog import Version
import logging
import re
from typing import List, Tuple, Dict, Optional, Union
from janitor import state
from janitor.config import read_config
from janitor.schedule import do_schedule
from ognibuild import Requirement
from ognibuild.buildlog import problem_to_upstream_requirement
from ognibuild.debian.apt import AptManager
from ognibuild.resolver.apt import resolve_requirement_apt
from ognibuild.session.plain import PlainSession
from buildlog_consultant import problem_clses
from lintian_brush.debianize import find_upstream, UpstreamInfo
from janitor.schedule import do_schedule

parser = argparse.ArgumentParser("reschedule")
parser.add_argument(
    "--config", type=str, default="janitor.conf", help="Path to configuration."
)
args = parser.parse_args()
with open(args.config, "r") as f:
    config = read_config(f)


def recreate_problem(kind, details):
    try:
        return problem_clses[kind](**details)
    except KeyError:
        return None


async def gather_requirements(db, session):
    async with db.acquire() as conn:
        for row in await conn.fetch("""
SELECT result_code, failure_details FROM last_unabsorbed_runs WHERE result_code != 'success' AND failure_details IS NOT NULL
"""):
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
                            # TODO(jelmer): find source name
                            option.append(UpdatePackage(r['name'], r['version'][1]))
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


async def main(db, session):
    requirements = []
    async for requirement in gather_requirements(db, session):
        if requirement not in requirements:
            requirements.append(requirement)

    async with db.acquire() as conn:
        for requirement in requirements:
            actions = await resolve_requirement(conn, requirement)
            logging.debug('%s: %r', requirement, actions)
            if actions == []:
                # We don't know what to do
                continue
            if actions == [[]]:
                # We don't need to do anything - could retry things that need this?
                continue
            if isinstance(actions[0][0], NewPackage):
                print('new-package: debianize %s-upstream => %r' % (actions[0][0].upstream_info.name, actions[0][0].upstream_info.branch_url))
            elif isinstance(actions[0][0], UpdatePackage):

                logging.info('Scheduling new run for %s/fresh-releases', actions[0][0].name)
                await do_schedule(conn, actions[0][0].name, "fresh-releases")



logging.basicConfig(level=logging.DEBUG)

db = state.Database(config.database_location)
session = PlainSession()
with session:
    apt_mgr = AptManager.from_session(session)
    asyncio.run(main(db, session))
