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
    "add_to_queue",
    "schedule_from_candidates",
]

from datetime import datetime, timedelta
import logging
from typing import Optional, List, Tuple

from debian.changelog import Version
from debian.deb822 import PkgRelation

import asyncpg

from . import (
    state,
)
from .debian import state as debian_state
from .config import read_config

FIRST_RUN_BONUS = 100.0


# Default estimation if there is no median for the suite or the package.
DEFAULT_ESTIMATED_DURATION = 15
DEFAULT_SCHEDULE_OFFSET = -1.0


TRANSIENT_ERROR_RESULT_CODES = [
    "cancelled",
    "install-deps-file-fetch-failure",
    "apt-get-update-file-fetch-failure",
    "build-failed-stage-apt-get-update",
    "build-failed-stage-apt-get-dist-upgrade",
    "build-failed-stage-explain-bd-uninstallable",
    "502-bad-gateway",
    "worker-exception",
    "build-failed-stage-create-session",
    "apt-get-update-missing-release-file",
    "no-space-on-device",
    "worker-killed",
    "too-many-requests",
    "autopkgtest-testbed-chroot-disappeared",
    "autopkgtest-file-fetch-failure",
    "check-space-insufficient-disk-space",
    "worker-resume-branch-unavailable",
    "explain-bd-uninstallable-apt-file-fetch-failure",
    "worker-timeout",
    "result-push-failed",
    "dist-apt-file-fetch-failure",
    "autopkgtest-apt-file-fetch-failure",
]

# In some cases, we want to ignore certain results when guessing
# whether a future run is going to be successful.
# For example, some results are transient, or sometimes new runs
# will give a clearer error message.
IGNORE_RESULT_CODE = {
    # Run worker failures from more than a day ago.
    "worker-failure": lambda run: ((datetime.now() - run.times[0]).days > 0),
}

IGNORE_RESULT_CODE.update(
    {code: lambda run: True for code in TRANSIENT_ERROR_RESULT_CODES}
)


PUBLISH_MODE_VALUE = {
    "build-only": 0,
    "push": 500,
    "propose": 400,
    "attempt-push": 450,
    "bts": 100,
}


def full_command(update_changelog: str, command: List[str]) -> List[str]:
    """Generate the full command to run.

    Args:
      update_changelog: changelog updating policy
      command: Command to run (as list of arguments)
    Returns:
      full list of arguments
    """
    entry_command = command
    if update_changelog == "update":
        entry_command.append("--update-changelog")
    elif update_changelog == "leave":
        entry_command.append("--no-update-changelog")
    elif update_changelog == "auto":
        pass
    else:
        raise ValueError("Invalid value %r for update_changelog" % update_changelog)
    return entry_command


async def schedule_from_candidates(iter_candidates_with_policy):
    for (
        package,
        suite,
        context,
        value,
        success_chance,
        policy,
    ) in iter_candidates_with_policy:
        if package.branch_url is None:
            continue

        (publish_mode, update_changelog, command) = policy

        if publish_mode is None:
            logging.info("%s: no policy defined", package.name)
            continue

        if all([mode == "skip" for mode in publish_mode]):
            logging.debug("%s: skipping, per policy", package.name)
            continue

        if not command:
            logging.debug("%s: skipping, no command set", package.name)
            continue

        for mode in publish_mode.values():
            value += PUBLISH_MODE_VALUE[mode]

        entry_command = full_command(update_changelog, command)

        yield (package.name, context, entry_command, suite, value, success_chance)


async def estimate_success_probability(
    conn: asyncpg.Connection, package: str, suite: str, context: Optional[str] = None
) -> Tuple[float, int]:
    # TODO(jelmer): Bias this towards recent runs?
    total = 0
    success = 0
    if context is None:
        same_context_multiplier = 0.5
    else:
        same_context_multiplier = 1.0
    async for run in state.iter_previous_runs(conn, package, suite):
        try:
            ignore_checker = IGNORE_RESULT_CODE[run.result_code]
        except KeyError:
            pass
        else:
            if ignore_checker(run):
                continue
        total += 1
        if run.result_code == "success":
            success += 1
        same_context = False
        if context and context in (run.instigated_context, run.context):
            same_context = True
        if run.result_code == "install-deps-unsatisfied-dependencies":
            START = "Unsatisfied dependencies: "
            if run.description and run.description.startswith(START):
                unsatisfied_dependencies = PkgRelation.parse_relations(
                    run.description[len(START) :]
                )
                if await deps_satisfied(conn, suite, unsatisfied_dependencies):
                    success += 1
                    same_context = False
        if same_context:
            same_context_multiplier = 0.1

    if total == 0:
        # If there were no previous runs, then it doesn't really matter that
        # we don't know the context.
        same_context_multiplier = 1.0

    return ((success * 10 + 1) / (total * 10 + 1) * same_context_multiplier), total


async def estimate_duration(
    conn: asyncpg.Connection, package: str, suite: str
) -> timedelta:
    """Estimate the duration of a package build for a certain suite."""
    estimated_duration = await state.estimate_duration(
        conn, package=package, suite=suite
    )
    if estimated_duration is not None:
        return estimated_duration

    estimated_duration = await state.estimate_duration(conn, package=package)
    if estimated_duration is not None:
        return estimated_duration

    estimated_duration = await state.estimate_duration(conn, suite=suite)
    if estimated_duration is not None:
        return estimated_duration

    return timedelta(seconds=DEFAULT_ESTIMATED_DURATION)


async def add_to_queue(
    conn: asyncpg.Connection,
    todo,
    dry_run: bool = False,
    default_offset: float = 0.0,
    bucket: str = "default",
) -> None:
    popcon = {k: (v or 0) for (k, v) in await debian_state.popcon(conn)}
    removed = set(p.name for p in await debian_state.iter_packages(conn) if p.removed)
    if popcon:
        max_inst = max([(v or 0) for v in popcon.values()])
        if max_inst:
            logging.info("Maximum inst count: %d", max_inst)
    else:
        max_inst = None
    for package, context, command, suite, value, success_chance in todo:
        assert package is not None
        assert value > 0, "Value: %s" % value
        if package in removed:
            continue
        estimated_duration = await estimate_duration(conn, package, suite)
        assert estimated_duration >= timedelta(
            0
        ), "%s: estimated duration < 0.0: %r" % (package, estimated_duration)
        (
            estimated_probability_of_success,
            total_previous_runs,
        ) = await estimate_success_probability(conn, package, suite, context)
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
            "Package %s: "
            "estimated_popularity(%.2f) * "
            "probability_of_success(%.2f) * value(%d) = "
            "estimated_value(%.2f), estimated cost (%f)",
            package,
            estimated_popularity,
            estimated_probability_of_success,
            value,
            estimated_value,
            estimated_cost,
        )

        if not dry_run:
            added = await state.add_to_queue(
                conn,
                package=package,
                suite=suite,
                command=command,
                offset=offset,
                bucket=bucket,
                estimated_duration=estimated_duration,
                context=context,
                requestor="scheduler",
            )
        else:
            added = True
        if added:
            logging.info("Scheduling %s (%s) with offset %f", package, suite, offset)


async def dep_available(
    conn: asyncpg.Connection,
    suite: str,
    name: str,
    archqual: Optional[str] = None,
    arch: Optional[str] = None,
    version: Optional[Tuple[str, Version]] = None,
    restrictions=None,
) -> bool:
    available = await debian_state.version_available(conn, name, suite, version)
    if available:
        return True
    return False


async def deps_satisfied(conn: asyncpg.Connection, suite: str, dependencies) -> bool:
    for dep in dependencies:
        for subdep in dep:
            if await dep_available(conn, suite=suite, **subdep):
                break
        else:
            return False
    return True


async def main():
    import argparse
    from janitor import state
    from prometheus_client import (
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
    parser.add_argument("--suite", type=str, help="Restrict to a specific suite.")
    parser.add_argument("packages", help="Package to process.", nargs="*")

    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format='%(message)s')

    last_success_gauge = Gauge(
        "job_last_success_unixtime", "Last time a batch job successfully finished"
    )

    with open(args.config, "r") as f:
        config = read_config(f)

    db = state.Database(config.database_location)

    async with db.acquire() as conn:
        iter_candidates_with_policy = await debian_state.iter_candidates_with_policy(
            conn, packages=(args.packages or None), suite=args.suite
        )
        todo = [x async for x in schedule_from_candidates(iter_candidates_with_policy)]
        await add_to_queue(conn, todo, dry_run=args.dry_run)

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        push_to_gateway(args.prometheus, job="janitor.schedule", registry=REGISTRY)


async def do_schedule_control(
    conn: asyncpg.Connection,
    package: str,
    main_branch_revision: Optional[bytes],
    offset: Optional[float] = None,
    refresh: bool = False,
    bucket: str = "default",
    requestor: Optional[str] = None,
) -> Tuple[float, Optional[timedelta]]:
    command = ["just-build"]
    if main_branch_revision is not None:
        command.append("--revision=%s" % main_branch_revision.decode("utf-8"))
    return await do_schedule(
        conn,
        package,
        "unchanged",
        offset=offset,
        refresh=refresh,
        bucket=bucket,
        requestor=requestor,
        command=command,
    )


class PolicyUnavailable(Exception):
    def __init__(self, suite: str, package: str):
        self.suite = suite
        self.package = package


async def do_schedule(
    conn: asyncpg.Connection,
    package: str,
    suite: str,
    offset: Optional[float] = None,
    bucket: str = "default",
    refresh: bool = False,
    requestor: Optional[str] = None,
    estimated_duration=None,
    command=None,
) -> Tuple[float, Optional[timedelta]]:
    if offset is None:
        offset = DEFAULT_SCHEDULE_OFFSET
    if command is None:
        (update_changelog, command) = await state.get_policy(conn, package, suite)
        if not command or not update_changelog:
            raise PolicyUnavailable(suite, package)
        command = full_command(update_changelog, command)
    if estimated_duration is None:
        estimated_duration = await estimate_duration(conn, package, suite)
    await state.add_to_queue(
        conn,
        package,
        command,
        suite,
        offset,
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
