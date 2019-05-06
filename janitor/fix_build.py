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

__all__ = [
    'build_incrementally',
]

import os
import subprocess

from breezy.commit import PointlessCommit
from lintian_brush import reset_tree
from lintian_brush.control import (
    add_dependency,
    ensure_minimum_version,
    update_control,
    )
from silver_platter.debian import (
    debcommit,
    )

from .build import attempt_build
from .sbuild_log import (
    MissingPythonModule,
    SbuildFailure,
    )


def add_build_dependency(tree, package, minimum_version=None,
                         committer=None):
    # TODO(jelmer): Make sure "package" actually exists in Debian
    def add_build_dep(control):
        if minimum_version:
            control["Build-Depends"] = ensure_minimum_version(
                control["Build-Depends"],
                package, minimum_version)
        else:
            control["Build-Depends"] = add_dependency(
                control["Build-Depends"], package)

    update_control(
        source_package_cb=add_build_dep,
        path=os.path.join(tree.basedir, 'debian/control'))

    if minimum_version:
        desc = "%s (>= %s)" % (package, minimum_version)
    else:
        desc = package

    subprocess.check_call(
        ["dch", "Add missing dependency on %s" % desc],
        cwd=tree.basedir, stderr=subprocess.DEVNULL)
    try:
        debcommit(tree, committer=committer)
    except PointlessCommit:
        return False
    else:
        return True


def resolve_error(tree, error, committer=None):
    if isinstance(error, MissingPythonModule):
        if error.python_version == 2:
            return add_build_dependency(
                tree, 'python-%s' % error.module, error.minimum_version,
                committer=committer)

    return False


def build_incrementally(
        local_tree, suffix, build_suite, output_directory, build_command,
        build_changelog_entry='Build for debian-janitor apt repository.',
        committer=None):
    last_fixed = None
    while True:
        try:
            attempt_build(local_tree, suffix, build_suite, output_directory,
                          build_command, build_changelog_entry)
        except SbuildFailure as e:
            if e.error is None:
                raise
            if last_fixed == e.error:
                raise
            reset_tree(local_tree)
            if not resolve_error(local_tree, e.error, committer=committer):
                raise
            last_fixed = e.error
