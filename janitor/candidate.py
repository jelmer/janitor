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
    'iter_fresh_snapshots_candidates',
    'iter_fresh_releases_candidates',
    'iter_lintian_fixes_candidates',
]

from .udd import UDD

DEFAULT_VALUE_NEW_UPSTREAM_SNAPSHOTS = 20
DEFAULT_VALUE_NEW_UPSTREAM = 30
DEFAULT_VALUE_LINTIAN_BRUSH_ADDON_ONLY = 10
DEFAULT_VALUE_LINTIAN_BRUSH = 50
LINTIAN_BRUSH_TAG_VALUE = 1

# Default to 5 minutes
DEFAULT_ESTIMATED_DURATION = 15


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
            policy, 'lintian_fixes', package.name, package.maintainer_email,
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


async def iter_fresh_releases_candidates(packages=None):
    udd = await UDD.public_udd_mirror()
    async for package, upstream_version in udd.iter_packages_with_new_upstream(
            packages or None):
        yield (package, 'fresh-releases', ['new-upstream'], upstream_version,
               DEFAULT_VALUE_NEW_UPSTREAM)


async def iter_fresh_snapshots_candidates(packages):
    udd = await UDD.public_udd_mirror()
    async for package in udd.iter_source_packages_with_vcs(packages or None):
        yield (package, 'fresh-snapshots', ['new-upstream', '--snapshot'],
               None, DEFAULT_VALUE_NEW_UPSTREAM_SNAPSHOTS)


async def iter_lintian_fixes_candidates(
        packages, available_fixers, propose_addon_only):
    udd = await UDD.public_udd_mirror()
    async for package, tags in udd.iter_source_packages_by_lintian(
            available_fixers, packages if packages else None):
        if not (set(tags) - set(propose_addon_only)):
            # Penalty for whitespace-only fixes
            value = DEFAULT_VALUE_LINTIAN_BRUSH_ADDON_ONLY
        else:
            value = DEFAULT_VALUE_LINTIAN_BRUSH
        value += len(tags) * LINTIAN_BRUSH_TAG_VALUE
        context = ' '.join(sorted(tags))
        yield package, 'lintian-fixes', ['lintian-brush'], context, value


async def main():
    import argparse
    from janitor import state
    from silver_platter.debian.lintian import (
        available_lintian_fixers,
        DEFAULT_ADDON_FIXERS,
    )

    parser = argparse.ArgumentParser(prog='propose-lintian-fixes')
    parser.add_argument("packages", nargs='*')
    parser.add_argument("--fixers",
                        help="Fixers to run.", type=str, action='append')
    parser.add_argument("--policy",
                        help="Policy file to read.", type=str,
                        default='policy.conf')
    parser.add_argument("--dry-run",
                        help="Create branches but don't push or propose anything.",
                        action="store_true", default=False)
    parser.add_argument('--propose-addon-only',
                        help='Fixers that should be considered add-on-only.',
                        type=str, action='append',
                        default=DEFAULT_ADDON_FIXERS)
    parser.add_argument('--prometheus', type=str,
                        help='Prometheus push gateway to export to.')
    args = parser.parse_args()

    tags = set()
    for fixer in available_lintian_fixers():
        tags.update(fixer.lintian_tags)

    async for package, suite, command, context, value in iter_lintian_fixes_candidates(
            args.packages, tags, args.propose_addon_only):
        await state.store_candidate(package.name, suite, command, context, value)


if __name__ == '__main__':
    import asyncio
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
