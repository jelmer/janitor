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

from ognibuild.debian.apt import AptManager
from ognibuild.debian.fix_build import build_incrementally
from ognibuild.session import SessionSetupFailure
from ognibuild.session.plain import PlainSession
from ognibuild.session.schroot import SchrootSession
from ognibuild.debian.build import (
    build_once,
    MissingChangesFile,
    DetailedDebianBuildFailure,
    UnidentifiedDebianBuildError,
)
from silver_platter.debian import MissingUpstreamTarball

from . import tree_set_changelog_version

# TODO(jelmer): Get rid of this circular import
from ..worker import WorkerFailure


MAX_BUILD_ITERATIONS = 50


def build(ws, subpath, output_directory, chroot=None, command=None,
          suffix=None, distribution=None, last_build_version=None,
          lintian_profile=None, lintian_suppress_tags=None, committer=None,
          apt_repository=None, apt_repository_key=None, extra_repositories=None,
          update_changelog=None):
    if not ws.local_tree.has_filename(os.path.join(subpath, 'debian/changelog')):
        raise WorkerFailure("not-debian-package", "Not a Debian package")

    if chroot:
        session = SchrootSession(chroot)
    else:
        session = PlainSession()
    try:
        with session:
            apt = AptManager(session)
            if command:
                if last_build_version:
                    # Update the changelog entry with the previous build version;
                    # This allows us to upload incremented versions for subsequent
                    # runs.
                    tree_set_changelog_version(
                        ws.local_tree, last_build_version, subpath
                    )

                source_date_epoch = ws.local_tree.branch.repository.get_revision(
                    ws.main_branch.last_revision()
                ).timestamp
                try:
                    if not suffix:
                        (changes_names, cl_entry) = build_once(
                            ws.local_tree,
                            distribution,
                            output_directory,
                            command,
                            subpath=subpath,
                            source_date_epoch=source_date_epoch,
                            apt_repository=apt_repository,
                            apt_repository_key=apt_repository_key,
                            extra_repositories=extra_repositories,
                        )
                    else:
                        (changes_names, cl_entry) = build_incrementally(
                            ws.local_tree,
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
                        )
                except MissingUpstreamTarball:
                    raise WorkerFailure(
                        "build-missing-upstream-source", "unable to find upstream source"
                    )
                except MissingChangesFile as e:
                    raise WorkerFailure(
                        "build-missing-changes",
                        "Expected changes path %s does not exist." % e.filename,
                        details={'filename': e.filename}
                    )
                except DetailedDebianBuildFailure as e:
                    if e.stage and not e.error.is_global:
                        code = "%s-%s" % (e.stage, e.error.kind)
                    else:
                        code = e.error.kind
                    try:
                        details = e.error.json()
                    except NotImplementedError:
                        details = None
                        actions = None
                    else:
                        from .missing_deps import resolve_requirement
                        from ognibuild.buildlog import problem_to_upstream_requirement
                        # Maybe there's a follow-up action we can consider?
                        req = problem_to_upstream_requirement(e.error)
                        if req:
                            actions = resolve_requirement(apt, req)
                            if actions:
                                logging.info('Suggesting follow-up actions: %r', actions)
                        else:
                            actions = None
                    raise WorkerFailure(code, e.description, details=details, followup_actions=actions)
                except UnidentifiedDebianBuildError as e:
                    if e.stage is not None:
                        code = "build-failed-stage-%s" % e.stage
                    else:
                        code = "build-failed"
                    raise WorkerFailure(code, e.description)
                logging.info("Built %r.", changes_names)
    except SessionSetupFailure as e:
        if e.errlines:
            sys.stderr.buffer.writelines(e.errlines)
        raise WorkerFailure('session-setup-failure', str(e))
    from .lintian import run_lintian
    lintian_result = run_lintian(
        output_directory, changes_names, profile=lintian_profile,
        suppress_tags=lintian_suppress_tags)
    return {'lintian': lintian_result}
