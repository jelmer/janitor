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

from . import (
    state,
    trace,
    )
from .config import read_config

from silver_platter.debian import (
    convert_debian_vcs_url,
)
from .policy import (
    read_policy,
    apply_policy,
)

SUCCESS_WEIGHT = 20
POPULARITY_WEIGHT = 1


# Default to 5 minutes
DEFAULT_ESTIMATED_DURATION = 15


# In some cases, we want to ignore certain results when guessing
# whether a future run is going to be successful.
# For example, some results are transient, or sometimes new runs
# will give a clearer error message.
IGNORE_RESULT_CODE = {
    'worker-exception': lambda run: True,
    'build-failed-stage-explain-bd-uninstallable': lambda run: True,
    # Run worker failures from more than a day ago.
    'worker-failure': lambda run: ((datetime.now() - run.times[0]).days > 0),
}


async def schedule_from_candidates(policy, iter_candidates):
    with open(policy, 'r') as f:
        policy = read_policy(f)

    async for package, suite, command, context, value in iter_candidates:
        try:
            vcs_url = convert_debian_vcs_url(package.vcs_type, package.vcs_url)
        except ValueError as e:
            trace.note('%s: %s', package.name, e)
            continue

        mode, update_changelog, committer = apply_policy(
            policy, suite.replace('-', '_'), package.name,
            package.maintainer_email, package.uploader_emails)

        if mode == 'skip':
            trace.mutter('%s: skipping, per policy', package.name)
            continue

        entry_command = list(command)
        if update_changelog == "update":
            entry_command.append("--update-changelog")
        elif update_changelog == "leave":
            entry_command.append("--no-update-changelog")
        elif update_changelog == "auto":
            pass
        else:
            raise ValueError(
                "Invalid value %r for update_changelog" % update_changelog)
        yield (
            vcs_url, mode,
            {'COMMITTER': committer,
             'PACKAGE': package.name,
             'CONTEXT': context,
             'UPLOADER_EMAILS': ','.join(package.uploader_emails),
             'MAINTAINER_EMAIL': package.maintainer_email},
            entry_command, suite, value)


async def estimate_success_probability(package, suite, context=None):
    # TODO(jelmer): Bias this towards recent runs?
    total = 0
    success = 0
    context_repeated = False
    async for run in state.iter_previous_runs(package, suite):
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
        if context and context in (run.instigated_context, run.context):
            context_repeated = True
    return (
        (success * 10 + 1) /
        (total * 10 + 1) *
        (1.0 if not context_repeated else .10))


async def estimate_duration(package, suite):
    estimated_duration = await state.estimate_duration(package, suite)
    if estimated_duration is not None:
        return estimated_duration

    estimated_duration = await state.estimate_duration(package)
    if estimated_duration is not None:
        return estimated_duration

    # TODO(jelmer): Just fall back to median duration for all builds for suite.
    return timedelta(seconds=DEFAULT_ESTIMATED_DURATION)


async def add_to_queue(todo, dry_run=False, default_offset=0):
    popcon = dict(await state.popcon())
    removed = set(p.name for p in await state.iter_packages()
                  if p.removed)
    max_inst = max([(v or 0) for k, v in popcon.items()])
    trace.note('Maximum inst count: %d', max_inst)
    for vcs_url, mode, env, command, suite, value in todo:
        assert vcs_url is not None
        assert value > 0, "Value: %s" % value
        package = env['PACKAGE']
        if package in removed:
            continue
        estimated_duration = await estimate_duration(
            package, suite)
        estimated_probability_of_success = await estimate_success_probability(
            package, suite, env.get('CONTEXT'))
        assert (estimated_probability_of_success >= 0.0 and
                estimated_probability_of_success <= 1.0), \
            "Probability of success: %s" % estimated_probability_of_success
        estimated_cost = 50 + estimated_duration.total_seconds()
        assert estimated_cost > 0, "Estimated cost: %d" % estimated_cost
        estimated_popularity = max(
            popcon.get(package, (0, 0))[0], 10) / max_inst
        estimated_value = (
            estimated_popularity * estimated_probability_of_success * value)
        assert estimated_value > 0, "Estimated value: %s" % estimated_value
        offset = estimated_cost / estimated_value
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
                vcs_url, package, command, suite, offset=int(offset),
                estimated_duration=estimated_duration,
                context=env.get('CONTEXT'), committer=env.get('COMMITTER'),
                requestor='scheduler')
        else:
            added = True
        if added:
            trace.note('Scheduling %s (%s) with offset %d',
                       package, mode, offset)


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


    args = parser.parse_args()

    last_success_gauge = Gauge(
        'job_last_success_unixtime',
        'Last time a batch job successfully finished')

    with open(args.config, 'r') as f:
        config = read_config(f)

    state.DEFAULT_URL = config.database_location

    iter_candidates = await state.iter_candidates()
    todo = [x async for x in schedule_from_candidates(
        args.policy, iter_candidates)]
    await add_to_queue(todo, dry_run=args.dry_run)

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        push_to_gateway(
            args.prometheus, job='janitor.schedule',
            registry=REGISTRY)


if __name__ == '__main__':
    import asyncio
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
