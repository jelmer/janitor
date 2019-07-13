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
    'schedule_udd',
    'schedule_ubuntu',
    'schedule_udd_new_upstreams',
    'schedule_udd_new_upstream_snapshots',
]

from datetime import datetime, timedelta

from . import (
    state,
    trace,
    )

from silver_platter.debian import (
    convert_debian_vcs_url,
)
from .policy import (
    read_policy,
    apply_policy,
)
from .udd import UDD

SUCCESS_WEIGHT = 20
POPULARITY_WEIGHT = 1


DEFAULT_VALUE_NEW_UPSTREAM_SNAPSHOTS = 20
DEFAULT_VALUE_NEW_UPSTREAM = 30
DEFAULT_VALUE_LINTIAN_BRUSH_ADDON_ONLY = 10
DEFAULT_VALUE_LINTIAN_BRUSH = 50
LINTIAN_BRUSH_TAG_VALUE = 1

# Default to 5 minutes
DEFAULT_ESTIMATED_DURATION = 15


# These are result codes that suggest some part of the system failed, but
# not what exactly. Recent versions of the janitor will hopefully
# give a better result code.
VAGUE_RESULT_CODES = [
    None, 'worker-failure', 'worker-exception',
    'build-failed-stage-explain-bd-uninstallable', 'build-failed']


def get_ubuntu_package_url(launchpad, package):
    ubuntu = launchpad.distributions['ubuntu']
    lp_repo = launchpad.git_repositories.getDefaultRepository(
        target=ubuntu.getSourcePackage(name=package))
    if lp_repo is None:
        raise ValueError('No git repository for %s' % package)
    return lp_repo.git_ssh_url


async def schedule_ubuntu(policy, propose_addon_only, packages):
    from breezy.plugins.launchpad.lp_api import (
        Launchpad,
        get_cache_directory,
        httplib2,
        )
    proxy_info = httplib2.proxy_info_from_environment('https')
    cache_directory = get_cache_directory()
    launchpad = Launchpad.login_with(
        'bzr', 'production', cache_directory, proxy_info=proxy_info,
        version='devel')

    udd = await UDD.public_udd_mirror()

    with open(policy, 'r') as f:
        policy = read_policy(f)

    async for package in udd.iter_ubuntu_source_packages(
            packages if packages else None):
        mode, update_changelog, committer = apply_policy(
            policy, 'lintian_brush', package.name, package.maintainer_email,
            package.uploader_emails)

        if mode == 'skip':
            trace.mutter('%s: skipping, per policy', package.name)
            continue

        try:
            vcs_url = get_ubuntu_package_url(launchpad, package.name)
        except ValueError as e:
            trace.note('%s: %s', package.name, e)
            continue

        command = ["lintian-brush"]
        if update_changelog == "update":
            command.append("--update-changelog")
        elif update_changelog == "leave":
            command.append("--no-update-changelog")
        elif update_changelog == "auto":
            pass
        else:
            raise ValueError(
                "Invalid value %r for update_changelog" % update_changelog)
        yield (
            vcs_url, mode,
            {'COMMITTER': committer, 'PACKAGE': package.name},
            command, 100)


async def schedule_udd_new_upstreams(policy, packages):
    udd = await UDD.public_udd_mirror()

    with open(policy, 'r') as f:
        policy = read_policy(f)

    async for package, upstream_version in udd.iter_packages_with_new_upstream(
            packages or None):
        # TODO(jelmer): skip if "new-upstream $upstream_version" has already
        # been processed
        try:
            vcs_url = convert_debian_vcs_url(package.vcs_type, package.vcs_url)
        except ValueError as e:
            trace.note('%s: %s', package.name, e)
            continue

        mode, update_changelog, committer = apply_policy(
            policy, 'new_upstream_releases', package.name,
            package.maintainer_email, package.uploader_emails)

        if mode == 'skip':
            trace.mutter('%s: skipping, per policy', package.name)
            continue

        command = ["new-upstream"]
        if update_changelog == "update":
            command.append("--update-changelog")
        elif update_changelog == "leave":
            command.append("--no-update-changelog")
        elif update_changelog == "auto":
            pass
        else:
            raise ValueError(
                "Invalid value %r for update_changelog" % update_changelog)
        yield (
            vcs_url, mode,
            {'COMMITTER': committer,
             'PACKAGE': package.name,
             'CONTEXT': upstream_version,
             'MAINTAINER_EMAIL': package.maintainer_email},
            command, DEFAULT_VALUE_NEW_UPSTREAM)


async def schedule_udd_new_upstream_snapshots(policy, packages):
    udd = await UDD.public_udd_mirror()

    with open(policy, 'r') as f:
        policy = read_policy(f)

    async for package in udd.iter_source_packages_with_vcs(packages or None):
        try:
            vcs_url = convert_debian_vcs_url(package.vcs_type, package.vcs_url)
        except ValueError as e:
            trace.note('%s: %s', package.name, e)
            continue

        mode, update_changelog, committer = apply_policy(
            policy, 'new_upstream_snapshots', package.name,
            package.maintainer_email, package.uploader_emails)

        if mode == 'skip':
            trace.mutter('%s: skipping, per policy', package.name)
            continue

        command = ["new-upstream", "--snapshot"]
        if update_changelog == "update":
            command.append("--update-changelog")
        elif update_changelog == "leave":
            command.append("--no-update-changelog")
        elif update_changelog == "auto":
            pass
        else:
            raise ValueError(
                "Invalid value %r for update_changelog" % update_changelog)
        yield (
            vcs_url, mode,
            {'COMMITTER': committer,
             'PACKAGE': package.name,
             'CONTEXT': None,
             'MAINTAINER_EMAIL': package.maintainer_email},
            command, DEFAULT_VALUE_NEW_UPSTREAM_SNAPSHOTS)


async def schedule_udd(policy, propose_addon_only, packages, available_fixers):
    udd = await UDD.public_udd_mirror()

    with open(policy, 'r') as f:
        policy = read_policy(f)

    async for package, tags in udd.iter_source_packages_by_lintian(
            available_fixers, packages if packages else None):
        try:
            vcs_url = convert_debian_vcs_url(package.vcs_type, package.vcs_url)
        except ValueError as e:
            trace.note('%s: %s', package.name, e)
            continue

        mode, update_changelog, committer = apply_policy(
            policy, 'lintian_brush', package.name, package.maintainer_email,
            package.uploader_emails)

        if mode == 'skip':
            trace.mutter('%s: skipping, per policy', package.name)
            continue

        command = ["lintian-brush"]
        if update_changelog == "update":
            command.append("--update-changelog")
        elif update_changelog == "leave":
            command.append("--no-update-changelog")
        elif update_changelog == "auto":
            pass
        else:
            raise ValueError(
                "Invalid value %r for update_changelog" % update_changelog)
        if not (set(tags) - set(propose_addon_only)):
            # Penalty for whitespace-only fixes
            value = DEFAULT_VALUE_LINTIAN_BRUSH_ADDON_ONLY
        else:
            value = DEFAULT_VALUE_LINTIAN_BRUSH 
        value += len(tags) * LINTIAN_BRUSH_TAG_VALUE
        context = ' '.join(sorted(tags))
        yield (
            vcs_url, mode,
            {'COMMITTER': committer,
             'PACKAGE': package.name,
             'CONTEXT': context,
             'MAINTAINER_EMAIL': package.maintainer_email},
            command, value)


async def estimate_success_probability(package, suite, context=None):
    # TODO(jelmer): Bias this towards recent runs?
    total = 0
    success = 0
    context_repeated = False
    async for run in state.iter_previous_runs(package, suite):
        total += 1
        if run.result_code == 'success':
            success += 1
        if context and context in (run.instigated_context, run.context):
            context_repeated = True
    return (success * 10 + 1) / (total * 10 + 1) * (1.0 if not context_repeated else .10)


async def estimate_duration(package, suite):
    estimated_duration = await state.estimate_duration(package, suite)
    if estimated_duration is not None:
        return estimated_duration
    # TODO(jelmer): Just fall back to duration for any builds for package?
    # TODO(jelmer): Just fall back to median duration for all builds for suite.
    return timedelta(seconds=DEFAULT_ESTIMATED_DURATION)


async def add_to_queue(todo, suite, dry_run=False, default_offset=0):
    udd = await UDD.public_udd_mirror()
    popcon = {package: (inst, vote) for (package, inst, vote) in await udd.popcon()}
    max_inst = max([(v[0] or 0) for k, v in popcon.items()])
    trace.note('Maximum inst count: %d', max_inst)
    for vcs_url, mode, env, command, value in todo:
        assert value > 0, "Value: %s" % value
        package = env['PACKAGE']
        estimated_duration = await estimate_duration(
            package, suite)
        estimated_probability_of_success = await estimate_success_probability(
            package, suite, env.get('CONTEXT'))
        assert (estimated_probability_of_success >= 0.0 and
                estimated_probability_of_success <= 1.0), \
            "Probability of success: %s" % estimated_probability_of_success
        estimated_cost = 50 + estimated_duration.total_seconds()
        assert estimated_cost > 0, "Estimated cost: %d" % estimated_cost
        estimated_popularity = max(popcon.get(package, (0, 0))[0], 10) / max_inst
        estimated_value = (estimated_popularity * estimated_probability_of_success * value)
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
                vcs_url, env, command, suite, offset=int(offset),
                estimated_duration=estimated_duration)
        else:
            added = True
        if added:
            trace.note('Scheduling %s (%s) with offset %d',
                       package, mode, offset)
