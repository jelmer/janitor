#!/usr/bin/python
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

from __future__ import absolute_import

__all__ = [
    "bulk_add_to_queue",
]

from datetime import datetime, timedelta
import logging
from typing import Optional, List, Tuple

from debian.changelog import Version

import asyncpg

from .compat import shlex_join
from .config import read_config
from .queue import Queue

FIRST_RUN_BONUS = 100.0


# Default estimation if there is no median for the campaign or the package.
DEFAULT_ESTIMATED_DURATION = 15
DEFAULT_SCHEDULE_OFFSET = -1.0


TRANSIENT_ERROR_RESULT_CODES = [
    "cancelled",
    "aborted",
    "install-deps-file-fetch-failure",
    "apt-get-update-file-fetch-failure",
    "build-failed-stage-apt-get-update",
    "build-failed-stage-apt-get-dist-upgrade",
    "build-failed-stage-explain-bd-uninstallable",
    "502-bad-gateway",
    "worker-502-bad-gateway",
    "build-failed-stage-create-session",
    "apt-get-update-missing-release-file",
    "no-space-on-device",
    "worker-killed",
    "too-many-requests",
    "autopkgtest-testbed-chroot-disappeared",
    "autopkgtest-file-fetch-failure",
    "autopkgtest-apt-file-fetch-failure",
    "check-space-insufficient-disk-space",
    "worker-resume-branch-unavailable",
    "explain-bd-uninstallable-apt-file-fetch-failure",
    "worker-timeout",
    "worker-clone-bad-gateway",
    "result-push-failed",
    "result-push-bad-gateway",
    "dist-apt-file-fetch-failure",
    "post-build-testbed-chroot-disappeared",
    "post-build-file-fetch-failure",
    "post-build-apt-file-fetch-failure",
    "pull-rate-limited",
    "session-setup-failure",
    "run-disappeared",
]

# In some cases, we want to ignore certain results when guessing
# whether a future run is going to be successful.
# For example, some results are transient, or sometimes new runs
# will give a clearer error message.
IGNORE_RESULT_CODE = {
    # Run worker failures from more than a day ago.
    "worker-failure": lambda run: ((datetime.utcnow() - run['start_time']).days > 0),
}

IGNORE_RESULT_CODE.update(
    {code: lambda run: True for code in TRANSIENT_ERROR_RESULT_CODES}
)


PUBLISH_MODE_VALUE = {
    "skip": 0,
    "build-only": 0,
    "push": 500,
    "propose": 400,
    "attempt-push": 450,
    "bts": 100,
}


async def iter_candidates_with_policy(
        conn: asyncpg.Connection,
        packages: Optional[List[str]] = None,
        campaign: Optional[str] = None):
    query = """
SELECT
  package.name AS package,
  package.branch_url AS branch_url,
  candidate.suite AS campaign,
  candidate.context AS context,
  candidate.value AS value,
  candidate.success_chance AS success_chance,
  policy.publish AS publish,
  policy.command AS command
FROM candidate
INNER JOIN package on package.name = candidate.package
INNER JOIN policy ON
    policy.package = package.name AND
    policy.suite = candidate.suite
WHERE
  NOT package.removed AND
  package.branch_url IS NOT NULL
"""
    args = []
    if campaign is not None and packages is not None:
        query += " AND package.name = ANY($1::text[]) AND candidate.suite = $2"
        args.extend([packages, campaign])
    elif campaign is not None:
        query += " AND candidate.suite = $1"
        args.append(campaign)
    elif packages is not None:
        query += " AND package.name = ANY($1::text[])"
        args.append(packages)
    return await conn.fetch(query, *args)


def queue_item_from_candidate_and_policy(row):
    value = row['value']
    for entry in row['publish']:
        value += PUBLISH_MODE_VALUE[entry['mode']]

    return (row['package'], row['context'], row['command'], row['campaign'],
            value, row['success_chance'])


async def estimate_success_probability(
    conn: asyncpg.Connection, package: str, campaign: str, context: Optional[str] = None
) -> Tuple[float, int]:
    # TODO(jelmer): Bias this towards recent runs?
    total = 0
    success = 0
    if context is None:
        same_context_multiplier = 0.5
    else:
        same_context_multiplier = 1.0
    for run in await conn.fetch("""
SELECT result_code, instigated_context, context, failure_details, start_time
FROM run
WHERE package = $1 AND suite = $2
ORDER BY start_time DESC
""", package, campaign):
        try:
            ignore_checker = IGNORE_RESULT_CODE[run['result_code']]
        except KeyError:
            pass
        else:
            if ignore_checker(run):
                continue
        total += 1
        if run['result_code'] == "success":
            success += 1
        same_context = False
        if context and context in (run['instigated_context'], run['context']):
            same_context = True
        if (run['result_code'] == "install-deps-unsatisfied-dependencies" and run['failure_details']
                and run['failure_details'].get('relations')):
            if await deps_satisfied(conn, campaign, run['failure_details']['relations']):
                success += 1
                same_context = False
        if same_context:
            same_context_multiplier = 0.1

    if total == 0:
        # If there were no previous runs, then it doesn't really matter that
        # we don't know the context.
        same_context_multiplier = 1.0

    return ((success * 10 + 1) / (total * 10 + 1) * same_context_multiplier), total


async def _estimate_duration(
    conn: asyncpg.Connection,
    package: Optional[str] = None,
    campaign: Optional[str] = None,
    limit: Optional[int] = 1000,
) -> Optional[timedelta]:
    query = """
SELECT AVG(duration) FROM
(select finish_time - start_time as duration FROM run
WHERE """
    args = []
    if package is not None:
        query += " package = $1"
        args.append(package)
    if campaign is not None:
        if package:
            query += " AND"
        query += " suite = $%d" % (len(args) + 1)
        args.append(campaign)
    query += " ORDER BY finish_time DESC"
    if limit is not None:
        query += " LIMIT %d" % limit
    query += ") as q"
    return await conn.fetchval(query, *args)


async def estimate_duration(
    conn: asyncpg.Connection, package: str, campaign: str
) -> timedelta:
    """Estimate the duration of a package build for a certain campaign."""
    estimated_duration = await _estimate_duration(
        conn, package=package, campaign=campaign
    )
    if estimated_duration is not None:
        return estimated_duration

    estimated_duration = await _estimate_duration(conn, package=package)
    if estimated_duration is not None:
        return estimated_duration

    estimated_duration = await _estimate_duration(conn, campaign=campaign)
    if estimated_duration is not None:
        return estimated_duration

    return timedelta(seconds=DEFAULT_ESTIMATED_DURATION)


async def bulk_add_to_queue(
    conn: asyncpg.Connection,
    todo,
    dry_run: bool = False,
    default_offset: float = 0.0,
    bucket: str = "default",
) -> None:
    popcon = {k: (v or 0) for (k, v) in await conn.fetch("SELECT name, popcon_inst FROM package")}
    if popcon:
        max_inst = max([(v or 0) for v in popcon.values()])
        if max_inst:
            logging.info("Maximum inst count: %d", max_inst)
    else:
        max_inst = None
    for package, context, command, campaign, value, success_chance in todo:
        assert package is not None
        assert value > 0, "Value: %s" % value
        estimated_duration = await estimate_duration(conn, package, campaign)
        assert estimated_duration >= timedelta(
            0
        ), "%s: estimated duration < 0.0: %r" % (package, estimated_duration)
        (
            estimated_probability_of_success,
            total_previous_runs,
        ) = await estimate_success_probability(conn, package, campaign, context)
        if total_previous_runs == 0:
            value += FIRST_RUN_BONUS
        assert (
            estimated_probability_of_success >= 0.0
            and estimated_probability_of_success <= 1.0
        ), ("Probability of success: %s" % estimated_probability_of_success)
        if success_chance is not None:
            success_chance *= estimated_probability_of_success
        estimated_cost = 20000.0 + (
            1.0 * estimated_duration.total_seconds() * 1000.0
            + estimated_duration.microseconds
        )
        assert estimated_cost > 0.0, "%s: Estimated cost: %f" % (
            package,
            estimated_cost,
        )
        if max_inst:
            estimated_popularity = max(
                popcon.get(package, 0.0) / float(max_inst) * 5.0, 1.0
            )
        else:
            estimated_popularity = 1.0
        estimated_value = (
            estimated_popularity * estimated_probability_of_success * value
        )
        assert estimated_value > 0.0, "Estimated value: %s" % estimated_value
        offset = estimated_cost / estimated_value
        assert offset > 0.0
        offset = default_offset + offset
        logging.info(
            "Package %s/%s: "
            "estimated_popularity(%.2f) * "
            "probability_of_success(%.2f) * value(%d) = "
            "estimated_value(%.2f), estimated cost (%f)",
            campaign, package,
            estimated_popularity,
            estimated_probability_of_success,
            value,
            estimated_value,
            estimated_cost,
        )

        if not dry_run:
            queue = Queue(conn)
            await queue.add(
                package=package,
                campaign=campaign,
                change_set=None,
                command=command,
                offset=offset,
                bucket=bucket,
                estimated_duration=estimated_duration,
                context=context,
                requestor="scheduler",
            )
        logging.info("Scheduled %s (%s) with offset %f", package, campaign, offset)


async def dep_available(
    conn: asyncpg.Connection,
    name: str,
    archqual: Optional[str] = None,
    arch: Optional[str] = None,
    distribution: Optional[str] = None,
    version: Optional[Tuple[str, Version]] = None,
    restrictions=None,
) -> bool:
    query = """\
SELECT
  1
FROM
  all_debian_versions
WHERE
  source = $1 AND %(version_match)s
"""
    args = [name]
    if version:
        version_match = "version %s $2" % (version[0],)
        args.append(str(version[1]))
    else:
        version_match = "True"

    return bool(await conn.fetchval(
        query % {"version_match": version_match}, *args))


async def deps_satisfied(conn: asyncpg.Connection, campaign: str, dependencies) -> bool:
    for dep in dependencies:
        for subdep in dep:
            if await dep_available(conn, **subdep):
                break
        else:
            return False
    return True


async def main():
    import argparse
    from janitor import state
    from aiohttp_openmetrics import (
        Gauge,
        push_to_gateway,
        REGISTRY,
    )

    parser = argparse.ArgumentParser(prog="janitor.schedule")
    parser.add_argument(
        "--dry-run",
        help="Create branches but don't push or propose anything.",
        action="store_true",
        default=False,
    )
    parser.add_argument(
        "--prometheus", type=str, help="Prometheus push gateway to export to."
    )
    parser.add_argument(
        "--config", type=str, default="janitor.conf", help="Path to configuration."
    )
    parser.add_argument("--campaign", type=str, help="Restrict to a specific campaign.")
    parser.add_argument("--gcp-logging", action='store_true', help='Use Google cloud logging.')
    parser.add_argument("packages", help="Package to process.", nargs="*")
    parser.add_argument("--debug", action="store_true")

    args = parser.parse_args()

    if args.gcp_logging:
        import google.cloud.logging
        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    else:
        if args.debug:
            level = logging.DEBUG
        else:
            level = logging.INFO
        logging.basicConfig(level=level, format="%(message)s")

    last_success_gauge = Gauge(
        "job_last_success_unixtime", "Last time a batch job successfully finished"
    )

    logging.info('Reading configuration')
    with open(args.config, "r") as f:
        config = read_config(f)

    async with state.create_pool(config.database_location) as conn:
        logging.info('Finding candidates with policy')
        logging.info('Determining schedule for candidates')
        todo = [
            queue_item_from_candidate_and_policy(row)
            for row in
            await iter_candidates_with_policy(
                conn, packages=(args.packages or None), campaign=args.campaign)]
        logging.info('Adding %d items to queue', len(todo))
        await bulk_add_to_queue(conn, todo, dry_run=args.dry_run)

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        await push_to_gateway(args.prometheus, job="janitor.schedule", registry=REGISTRY)


async def do_schedule_control(
    conn: asyncpg.Connection,
    package: str,
    change_set: Optional[str] = None,
    main_branch_revision: Optional[bytes] = None,
    offset: Optional[float] = None,
    refresh: bool = False,
    bucket: str = "control",
    requestor: Optional[str] = None,
    estimated_duration: Optional[timedelta] = None
) -> Tuple[float, Optional[timedelta]]:
    command = ["brz", "up"]
    if main_branch_revision is not None:
        command.append("--revision=%s" % main_branch_revision.decode("utf-8"))
    return await do_schedule(
        conn,
        package,
        "control",
        change_set=change_set,
        offset=offset,
        refresh=refresh,
        bucket=bucket,
        requestor=requestor,
        command=shlex_join(command),
    )


class PolicyUnavailable(Exception):
    def __init__(self, campaign: str, package: str):
        self.campaign = campaign
        self.package = package


async def do_schedule(
    conn: asyncpg.Connection,
    package: str,
    campaign: str,
    change_set: Optional[str] = None,
    offset: Optional[float] = None,
    bucket: str = "default",
    refresh: bool = False,
    requestor: Optional[str] = None,
    estimated_duration=None,
    command: Optional[str] = None,
) -> Tuple[float, Optional[timedelta]]:
    if offset is None:
        offset = DEFAULT_SCHEDULE_OFFSET
    if command is None:
        policy = await conn.fetchrow(
            "SELECT command "
            "FROM policy WHERE package = $1 AND suite = $2",
            package, campaign)
        if not policy:
            raise PolicyUnavailable(campaign, package)
        command = policy['command']
    if estimated_duration is None:
        estimated_duration = await estimate_duration(conn, package, campaign)
    queue = Queue(conn)
    await queue.add(
        package=package,
        command=command,
        campaign=campaign,
        change_set=change_set,
        offset=offset,
        bucket=bucket,
        estimated_duration=estimated_duration,
        refresh=refresh,
        requestor=requestor,
    )
    return offset, estimated_duration


if __name__ == "__main__":
    import asyncio

    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
