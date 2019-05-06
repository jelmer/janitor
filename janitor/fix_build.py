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
import sys

from breezy.commit import PointlessCommit
from lintian_brush import reset_tree
from lintian_brush.control import (
    add_dependency,
    ensure_minimum_version,
    update_control,
    )
from silver_platter.debian import (
    debcommit,
    DEFAULT_BUILDER,
    )

from .build import attempt_build
from .sbuild_log import (
    MissingPythonModule,
    SbuildFailure,
    )
from .udd import UDD


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


def add_build_dependency_options(
        tree, package_candidates, minimum_version=None,
        committer=None):
    udd = UDD.public_udd_mirror()
    package = None
    for candidate in package_candidates:
        # TODO(jelmer): Check if this exists in the archive
        if udd.binary_package_exists(candidate):
            package = candidate
            break
    else:
        return False
    return add_build_dependency(
            tree, package, minimum_version=minimum_version,
            committer=committer)


def resolve_error(tree, error, committer=None):
    if isinstance(error, MissingPythonModule):
        if error.python_version == 2:
            candidates = [
                "python-%s" % (
                    error.module.split('.')[:i]
                    for i in range(1, error.module.count('.')))]
            if error.module.startswith('py'):
                candidates.append('python-%s' % error.module[2:])
            candidates.append(error.module)
            # Check if python-X, X or python-X.lstrip('py') exists
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
            return attempt_build(
                local_tree, suffix, build_suite, output_directory,
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


def main(argv=None):
    import argparse
    parser = argparse.ArgumentParser('janitor.fix_build')
    parser.add_argument('--suffix', type=str,
                        help="Suffix to use for test builds.",
                        default='fixbuild1')
    parser.add_argument('--suite', type=str,
                        help="Suite to target.",
                        default='unstable')
    parser.add_argument('--output-directory', type=str,
                        help="Output directory.", default=None)
    parser.add_argument('--committer', type=str,
                        help='Committer string (name and email)',
                        default=None)
    parser.add_argument(
        '--build-command', type=str,
        help='Build command',
        default=(DEFAULT_BUILDER + ' -A -s -v -d$DISTRIBUTION'))

    args = parser.parse_args()
    from breezy.workingtree import WorkingTree
    tree = WorkingTree.open('.')
    build_incrementally(
        tree, args.suffix, args.suite, args.output_directory,
        args.build_command, committer=args.committer)


if __name__ == '__main__':
    sys.exit(main(sys.argv))
