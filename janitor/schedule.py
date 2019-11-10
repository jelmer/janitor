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
    'add_to_queue',
    'schedule_from_candidates',
]

from datetime import datetime, timedelta
from debian.deb822 import PkgRelation

from . import (
    state,
    trace,
    )
from .config import read_config

from .policy import (
    read_policy,
    apply_policy,
)

SUCCESS_WEIGHT = 20
POPULARITY_WEIGHT = 1


# Default estimation if there is no median for the suite or the package.
DEFAULT_ESTIMATED_DURATION = 15
DEFAULT_SCHEDULE_OFFSET = -1
SUITE_TO_COMMAND = {
    'lintian-fixes': ['lintian-brush'],
    'fresh-releases': ['new-upstream'],
    'fresh-snapshots': ['new-upstream', '--snapshot'],
    'unchanged': ['just-build'],
    }


TRANSIENT_ERROR_RESULT_CODES = [
    'install-deps-file-fetch-failure',
    'apt-get-update-file-fetch-failure',
    'build-failed-stage-explain-bd-uninstallable',
    'worker-exception',
]

# In some cases, we want to ignore certain results when guessing
# whether a future run is going to be successful.
# For example, some results are transient, or sometimes new runs
# will give a clearer error message.
IGNORE_RESULT_CODE = {
    # Run worker failures from more than a day ago.
    'worker-failure': lambda run: ((datetime.now() - run.times[0]).days > 0),
}

IGNORE_RESULT_CODE.update(
    {code: lambda run: True for code in TRANSIENT_ERROR_RESULT_CODES})


PUBLISH_MODE_VALUE = {
    'build-only': 0,
    'push': 50,
    'propose': 40,
    'attempt-push': 45,
    }


def full_command(suite, update_changelog):
    entry_command = list(SUITE_TO_COMMAND[suite])
    if update_changelog == "update":
        entry_command.append("--update-changelog")
    elif update_changelog == "leave":
        entry_command.append("--no-update-changelog")
    elif update_changelog == "auto":
        pass
    else:
        raise ValueError(
            "Invalid value %r for update_changelog" % update_changelog)
    return entry_command


async def schedule_from_candidates(policy, iter_candidates):
    for package, suite, context, value in iter_candidates:
        if package.branch_url is None:
            continue

        publish_mode, update_changelog, committer = apply_policy(
            policy, suite,
            package.name, package.maintainer_email, package.uploader_emails)

        if publish_mode == 'skip':
            trace.mutter('%s: skipping, per policy', package.name)
            continue

        value += PUBLISH_MODE_VALUE[publish_mode]

        entry_command = full_command(suite, update_changelog)

        yield (
            package.name,
            {'COMMITTER': committer,
             'PACKAGE': package.name,
             'CONTEXT': context,
             'UPLOADER_EMAILS': ','.join(package.uploader_emails),
             'MAINTAINER_EMAIL': package.maintainer_email},
            entry_command, suite, value)


async def estimate_success_probability(conn, package, suite, context=None):
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
        if run.result_code == 'success':
            success += 1
        same_context = False
        if context and context in (run.instigated_context, run.context):
            same_context = True
        if run.result_code == 'install-deps-unsatisfied-dependencies':
            START = 'Unsatisfied dependencies: '
            if run.description.startswith(START):
                unsatisfied_dependencies = PkgRelation.parse_relations(
                    run.description[len(START):])
                if await deps_satisfied(conn, suite, unsatisfied_dependencies):
                    success += 1
                    same_context = False
        if same_context:
            same_context_multiplier = 0.1

    return (
        (success * 10 + 1) /
        (total * 10 + 1) *
        same_context_multiplier)


async def estimate_duration(conn, package, suite):
    estimated_duration = await state.estimate_duration(
        conn, package=package, suite=suite)
    if estimated_duration is not None:
        return estimated_duration

    estimated_duration = await state.estimate_duration(conn, package=package)
    if estimated_duration is not None:
        return estimated_duration

    estimated_duration = await state.estimate_duration(conn, suite=suite)
    if estimated_duration is not None:
        return estimated_duration

    return timedelta(seconds=DEFAULT_ESTIMATED_DURATION)


async def add_to_queue(conn, todo, dry_run=False, default_offset=0):
    popcon = {k: (v or 0) for (k, v) in await state.popcon(conn)}
    removed = set(p.name for p in await state.iter_packages(conn)
                  if p.removed)
    max_inst = max([(v or 0) for v in popcon.values()])
    trace.note('Maximum inst count: %d', max_inst)
    for package, env, command, suite, value in todo:
        assert package is not None
        assert value > 0, "Value: %s" % value
        if package in removed:
            continue
        estimated_duration = await estimate_duration(
            conn, package, suite)
        estimated_probability_of_success = await estimate_success_probability(
            conn, package, suite, env.get('CONTEXT'))
        assert (estimated_probability_of_success >= 0.0 and
                estimated_probability_of_success <= 1.0), \
            "Probability of success: %s" % estimated_probability_of_success
        estimated_cost = 50 + estimated_duration.total_seconds()
        assert estimated_cost > 0, "Estimated cost: %d" % estimated_cost
        estimated_popularity = max(popcon.get(package, 0), 10) / max_inst
        estimated_value = (
            estimated_popularity * estimated_probability_of_success * value)
        assert estimated_value > 0, "Estimated value: %s" % estimated_value
        offset = estimated_cost / estimated_value
        assert offset > 0
        offset = default_offset + offset
        trace.note(
            'Package %s: '
            'estimated value((%.2f * %d) * (%.2f * %d) * %d = %.2f), '
            'estimated cost (%d)',
            package, estimated_popularity, POPULARITY_WEIGHT,
            estimated_probability_of_success, SUCCESS_WEIGHT,
            value, estimated_value, estimated_cost)

        if not dry_run:
            added = await state.add_to_queue(
                conn, package, command, suite, offset=int(offset),
                estimated_duration=estimated_duration,
                context=env.get('CONTEXT'), committer=env.get('COMMITTER'),
                requestor='scheduler')
        else:
            added = True
        if added:
            trace.note('Scheduling %s (%s) with offset %d',
                       package, suite, offset)


async def dep_available(
        conn, suite, name, archqual=None, arch=None, version=None,
        restrictions=None):
    available = await state.version_available(conn, name, suite, version)
    if available:
        return True
    return False


async def deps_satisfied(conn, suite, dependencies):
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

    parser = argparse.ArgumentParser(prog='janitor.schedule')
    parser.add_argument("--policy",
                        help="Policy file to read.", type=str,
                        default='policy.conf')
    parser.add_argument(
        "--dry-run",
        help="Create branches but don't push or propose anything.",
        action="store_true", default=False)
    parser.add_argument('--prometheus', type=str,
                        help='Prometheus push gateway to export to.')
    parser.add_argument(
        '--config', type=str, default='janitor.conf',
        help='Path to configuration.')
    parser.add_argument('packages', help='Package to process.', nargs='*')

    args = parser.parse_args()

    last_success_gauge = Gauge(
        'job_last_success_unixtime',
        'Last time a batch job successfully finished')

    with open(args.config, 'r') as f:
        config = read_config(f)

    with open(args.policy, 'r') as f:
        policy = read_policy(f)

    db = state.Database(config.database_location)

    async with db.acquire() as conn:
        iter_candidates = await state.iter_candidates(
            conn, packages=(args.packages or None))
        todo = [x async for x in schedule_from_candidates(
            policy, iter_candidates)]
        await add_to_queue(conn, todo, dry_run=args.dry_run)

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        push_to_gateway(
            args.prometheus, job='janitor.schedule',
            registry=REGISTRY)


if __name__ == '__main__':
    import asyncio
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
