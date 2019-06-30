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

from datetime import datetime

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


async def schedule_ubuntu(policy, propose_addon_only, packages, shuffle=False):
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
            packages if packages else None, shuffle=shuffle):
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
            command, 0)


async def schedule_udd_new_upstreams(policy, packages, shuffle=False):
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
            command, 0)


async def schedule_udd_new_upstream_snapshots(policy, packages, shuffle=False):
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
            command, 0)


async def schedule_udd(policy, propose_addon_only, packages, available_fixers,
                 shuffle=False):
    udd = await UDD.public_udd_mirror()

    with open(policy, 'r') as f:
        policy = read_policy(f)

    async for package, tags in udd.iter_source_packages_by_lintian(
            available_fixers, packages if packages else None, shuffle=shuffle):
        priority = 0
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
            priority -= 200
        context = ' '.join(sorted(tags))
        yield (
            vcs_url, mode,
            {'COMMITTER': committer,
             'PACKAGE': package.name,
             'CONTEXT': context,
             'MAINTAINER_EMAIL': package.maintainer_email},
            command, priority)


priority_per_tag = {
    # Priority increase for packages that have never been processed before
    'first_run': 100,
    'last_successful_bonus': 90,
    'last_vague_bonus': 30,
    'no_context_failure': -30,
    # Penalty if the context has already been processed
    'context_processed': -1000,
    'no_context_refresh_bonus': 50,
    }


def determine_tags(package, command, mode, previous_runs, context=None,
                       priority=0):

    NO_CONTEXT_REFRESH_FREQUENCY = 14

    if previous_runs:
        (last_start_time, last_duration, last_instigated_context,
         last_context, last_main_branch_revision, last_result_code) = (
            previous_runs[0])
        # TODO(jelmer): Should last_context and last_instigated_context be
        # treated differently?
        if context and context in (last_context, last_instigated_context):
            yield 'context_processed'
        elif context is None:
            age = (datetime.now() - last_start_time)
            if age.days > NO_CONTEXT_REFRESH_FREQUENCY:
                yield 'no_context_refresh_bonus'
            elif last_result_code != 'success':
                yield 'no_context_failure'
        else:
            if last_result_code == 'success':
                yield 'last_successful_bonus'
        if last_result_code in VAGUE_RESULT_CODES:
            yield 'last_vague_bonus'
        priority -= (last_duration.total_seconds() / 60) / 10
    else:
        yield 'first_run'


async def add_to_queue(todo, dry_run=False, default_priority=0):
    for vcs_url, mode, env, command, priority in todo:
        package = env['PACKAGE']
        previous_runs = list(await state.iter_previous_runs(package, command))
        tags = determine_tags(
            package, command, mode, previous_runs, env.get('CONTEXT'))
        priority = default_priority + priority + sum(
            priority_per_tag[t] for t in tags)
        estimated_duration = None
        if previous_runs:
            for (last_start_time, last_duration, last_instigated_context,
                 last_context, last_main_branch_revision,
                 last_result_code) in previous_runs:
                if last_result_code == 'success':
                    estimated_duration = last_duration
                    break
        if not dry_run:
            added = await state.add_to_queue(
                vcs_url, env, command, priority=priority,
                estimated_duration=estimated_duration)
        else:
            added = True
        if added:
            trace.note('Scheduling %s (%s) with priority %d',
                       package, mode, priority)
