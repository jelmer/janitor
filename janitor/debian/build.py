#!/usr/bin/python3
# Copyright (C) 2018-2022 Jelmer Vernooij <jelmer@jelmer.uk>
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

import os
import logging
import sys
from typing import Optional, Any

from ognibuild.debian.apt import AptManager
from ognibuild.debian.fix_build import build_incrementally
from ognibuild.session import SessionSetupFailure, Session
from ognibuild.session.plain import PlainSession
from ognibuild.session.schroot import SchrootSession
from ognibuild.debian.build import (
    build_once,
    ChangelogNotEditable,
    MissingChangesFile,
    DetailedDebianBuildFailure,
    UnidentifiedDebianBuildError,
)
from silver_platter.debian import MissingUpstreamTarball

from . import tree_set_changelog_version


MAX_BUILD_ITERATIONS = 50
DEFAULT_BUILD_COMMAND = 'sbuild -A -s -v'


class BuildFailure(Exception):
    """Building failed."""

    def __init__(self, code: str, description: str, stage: Optional[str] = None,
                 details: Optional[Any] = None) -> None:
        self.code = code
        self.description = description
        self.details = details
        self.stage = stage

    def json(self):
        ret = {
            "code": self.code,
            "description": self.description,
            'details': self.details,
            'stage': self.stage,
        }
        return ret


def build(local_tree, subpath, output_directory, *, chroot=None, command=None,
          suffix=None, distribution=None, last_build_version=None,
          lintian_profile=None, lintian_suppress_tags=None, committer=None,
          apt_repository=None, apt_repository_key=None, extra_repositories=None,
          update_changelog=None, dep_server_url=None):
    if not local_tree.has_filename(os.path.join(subpath, 'debian/changelog')):
        raise BuildFailure("missing-changelog", "Missing changelog", stage="pre-check")

    session: Session
    if chroot:
        session = SchrootSession(chroot)
    else:
        session = PlainSession()

    source_date_epoch = local_tree.branch.repository.get_revision(
        local_tree.branch.last_revision()
    ).timestamp

    try:
        with session:
            apt = AptManager(session)
            if command:
                if last_build_version:
                    # Update the changelog entry with the previous build version;
                    # This allows us to upload incremented versions for subsequent
                    # runs.
                    tree_set_changelog_version(
                        local_tree, last_build_version, subpath
                    )

                try:
                    if not suffix:
                        (changes_names, cl_entry) = build_once(
                            local_tree,
                            distribution,
                            output_directory,
                            build_command=command,
                            subpath=subpath,
                            source_date_epoch=source_date_epoch,
                            apt_repository=apt_repository,
                            apt_repository_key=apt_repository_key,
                            extra_repositories=extra_repositories,
                        )
                    else:
                        (changes_names, cl_entry) = build_incrementally(
                            local_tree,
                            apt, "~" + suffix,
                            distribution,
                            output_directory,
                            build_command=command,
                            build_changelog_entry="Build for debian-janitor apt repository.",
                            committer=committer,
                            subpath=subpath,
                            source_date_epoch=source_date_epoch,
                            update_changelog=update_changelog,
                            max_iterations=MAX_BUILD_ITERATIONS,
                            apt_repository=apt_repository,
                            apt_repository_key=apt_repository_key,
                            extra_repositories=extra_repositories,
                            dep_server_url=dep_server_url,
                        )
                except ChangelogNotEditable as e:
                    raise BuildFailure(
                        "build-changelog-not-editable", str(e)) from e
                except MissingUpstreamTarball as e:
                    raise BuildFailure(
                        "build-missing-upstream-source", "unable to find upstream source",
                    ) from e
                except MissingChangesFile as e:
                    raise BuildFailure(
                        "build-missing-changes",
                        "Expected changes path %s does not exist." % e.filename,
                        details={'filename': e.filename},
                    ) from e
                except DetailedDebianBuildFailure as e:
                    try:
                        details = e.error.json()
                    except NotImplementedError:
                        details = None
                    raise BuildFailure(
                        e.error.kind, e.description, stage=e.stage,
                        details=details) from e
                except UnidentifiedDebianBuildError as e:
                    raise BuildFailure("failed", e.description, stage=e.stage) from e
                logging.info("Built %r.", changes_names)
    except SessionSetupFailure as e:
        if e.errlines:
            sys.stderr.buffer.writelines(e.errlines)
        raise BuildFailure('session-setup-failure', str(e), stage="session-setup",) from e
    from .lintian import run_lintian
    lintian_result = run_lintian(
        output_directory, changes_names, profile=lintian_profile,
        suppress_tags=lintian_suppress_tags)
    return {'lintian': lintian_result}


def build_from_config(local_tree, subpath, output_directory, config, env):
    build_distribution = config.get("build-distribution")
    build_command = config.get("build-command", DEFAULT_BUILD_COMMAND)
    build_suffix = config.get("build-suffix")
    last_build_version = config.get("last-build-version")
    chroot = config.get("chroot")
    lintian_profile = config.get('lintian', {}).get('profile')
    lintian_suppress_tags = config.get('lintian', {}).get("suppress-tags")
    apt_repository = config.get('base-apt-repository')
    apt_repository_key = config.get('base-apt-repository-signed-by')
    extra_repositories = config.pop('build-extra-repositories', [])
    dep_server_url = config.get('dep_server_url')
    committer = env.get("COMMITTER")
    uc = env.get("DEB_UPDATE_CHANGELOG", "auto")
    if uc == "auto":
        update_changelog = None
    elif uc == "update":
        update_changelog = True
    elif uc == "leave":
        update_changelog = True
    else:
        logging.warning(
            'Invalid value for DEB_UPDATE_CHANGELOG: %s, '
            'defaulting to auto.', uc)
        update_changelog = None
    return build(
        local_tree, subpath, output_directory, chroot=chroot,
        lintian_profile=lintian_profile,
        lintian_suppress_tags=lintian_suppress_tags,
        last_build_version=last_build_version,
        suffix=build_suffix,
        distribution=build_distribution,
        command=build_command,
        committer=committer,
        apt_repository=apt_repository,
        apt_repository_key=apt_repository_key,
        extra_repositories=extra_repositories,
        update_changelog=update_changelog,
        dep_server_url=dep_server_url)


def main():
    import argparse
    import json
    parser = argparse.ArgumentParser()
    parser.add_argument('--config', type=str, help="Path to configuration (JSON)")
    parser.add_argument('output_directory', type=str, help="Output directory")
    args = parser.parse_args()

    import breezy.bzr  # noqa: F401
    import breezy.git  # noqa: F401
    from breezy.workingtree import WorkingTree

    wt, subpath = WorkingTree.open_containing('.')

    if args.config:
        with open(args.config, 'r') as f:
            config = json.load(f)
    else:
        config = {}

    try:
        result = build_from_config(
            wt, subpath, args.output_directory, config=config,
            env=os.environ)
    except BuildFailure as e:
        json.dump(e.json(), sys.stdout)
        return 1

    json.dump(result, sys.stdout, indent=4)
    return 0


if __name__ == '__main__':
    sys.exit(main())
