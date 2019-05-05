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
VAGUE_RESULT_CODES = [None, 'worker-failure', 'worker-exception']


def get_ubuntu_package_url(launchpad, package):
    ubuntu = launchpad.distributions['ubuntu']
    lp_repo = launchpad.git_repositories.getDefaultRepository(
        target=ubuntu.getSourcePackage(name=package))
    if lp_repo is None:
        raise ValueError('No git repository for %s' % package)
    return lp_repo.git_ssh_url


def schedule_ubuntu(policy, propose_addon_only, packages, shuffle=False):
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

    udd = UDD.public_udd_mirror()

    with open(policy, 'r') as f:
        policy = read_policy(f)

    for package in udd.iter_ubuntu_source_packages(
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


def schedule_udd_new_upstreams(policy, packages, shuffle=False):
    udd = UDD.public_udd_mirror()

    with open(policy, 'r') as f:
        policy = read_policy(f)

    for package, upstream_version in udd.iter_packages_with_new_upstream(
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


def schedule_udd_new_upstream_snapshots(policy, packages, shuffle=False):
    udd = UDD.public_udd_mirror()

    with open(policy, 'r') as f:
        policy = read_policy(f)

    for package in udd.iter_source_packages_with_vcs(packages or None):
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


def schedule_udd(policy, propose_addon_only, packages, available_fixers,
                 shuffle=False):
    udd = UDD.public_udd_mirror()

    with open(policy, 'r') as f:
        policy = read_policy(f)

    for package, tags in udd.iter_source_packages_by_lintian(
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


def determine_priority(package, command, mode, context=None, priority=0):
    # Priority increase for packages that have never been processed before
    FIRST_RUN_BONUS = 100

    # Penalty if the context has already been processed
    CONTEXT_PROCESSED_PENALTY = 1000

    LAST_SUCCESSFUL_BONUS = 90
    LAST_VAGUE_BONUS = 30

    NO_CONTEXT_FAILURE_PENALTY = 30
    NO_CONTEXT_REFRESH_FREQUENCY = 7
    NO_CONTEXT_REFRESH_BONUS = 50

    previous_runs = list(
        state.iter_previous_runs(package, command))
    if previous_runs:
        (last_start_time, last_duration, last_context,
         last_main_branch_revision, last_result_code) = (
             previous_runs[0])
        if last_context and last_context == context:
            priority -= CONTEXT_PROCESSED_PENALTY
        elif last_context is None:
            age = (datetime.now() - last_start_time)
            if age.days > NO_CONTEXT_REFRESH_FREQUENCY:
                priority += NO_CONTEXT_REFRESH_BONUS
            elif last_result_code != 'success':
                priority -= NO_CONTEXT_FAILURE_PENALTY
        if last_result_code == 'success':
            priority += LAST_SUCCESSFUL_BONUS
        elif last_result_code in VAGUE_RESULT_CODES:
            priority += LAST_VAGUE_BONUS
        priority -= (last_duration.total_seconds() / 60) / 10
    else:
        priority += FIRST_RUN_BONUS
    return priority


def add_to_queue(todo, dry_run=False, default_priority=0):
    for vcs_url, mode, env, command, priority in todo:
        priority = default_priority + priority + determine_priority(
            env['PACKAGE'], command, mode, env.get('CONTEXT'))
        if not dry_run:
            added = state.add_to_queue(
                vcs_url, mode, env, command, priority=priority)
        else:
            added = True
        if added:
            trace.note('Scheduling %s (%s) with priority %d', env['PACKAGE'],
                       mode, priority)
