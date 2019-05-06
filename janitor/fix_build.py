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
    MissingCHeader,
    SbuildFailure,
    )
from .trace import note, warning, show_error


DEFAULT_MAX_ITERATIONS = 10


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

    note("Adding build dependency: %s", desc)
    subprocess.check_call(
        ["dch", "Add missing dependency on %s." % desc],
        cwd=tree.basedir, stderr=subprocess.DEVNULL)
    try:
        debcommit(tree, committer=committer)
    except PointlessCommit:
        return False
    else:
        return True


def search_apt_file(path, regex=False):
    args = ['/usr/bin/apt-file', 'search', '-l']
    if regex:
        args.append('-x')
    args.append(path)
    try:
        return subprocess.check_output(args).decode().splitlines()
    except subprocess.CalledProcessError:
        return []


def add_build_dependency_for_path(
        tree, path, minimum_version=None,
        committer=None, regex=False):
    if not isinstance(path, list):
        paths = [path]
    else:
        paths = path
    candidates = set()
    for path in paths:
        candidates.update(search_apt_file(path, regex=regex))
    if len(candidates) == 0:
        warning('No packages found that contain %r', path)
        return False
    if len(candidates) > 1:
        warning('More than 1 packages found that contain %r: %r',
                path, candidates)
        # Euhr. Pick the one with the shortest name?
        package = sorted(candidates, key=len)[0]
    else:
        package = candidates.pop()
    return add_build_dependency(
            tree, package, minimum_version=minimum_version,
            committer=committer)


def resolve_error(tree, error, committer=None):
    if isinstance(error, MissingPythonModule):
        if error.python_version == 2:
            paths = [
                os.path.join(
                    '/usr/lib/python2.*/dist-packages',
                    error.module.replace('.', '/'),
                    '__init__.py'),
                os.path.join(
                    '/usr/lib/python2.*/dist-packages',
                    error.module.replace('.', '/') + '.py')]
            return add_build_dependency_for_path(
                tree, paths, error.minimum_version, committer=committer,
                regex=True)
        elif error.python_version == 3:
            paths = [
                os.path.join(
                    '/usr/lib/python3.*/dist-packages',
                    error.module.replace('.', '/'),
                    '__init__.py'),
                os.path.join(
                    '/usr/lib/python3.*/dist-packages',
                    error.module.replace('.', '/') + '.py')]
            return add_build_dependency_for_path(
                tree, paths, error.minimum_version, committer=committer,
                regex=True)
    elif isinstance(error, MissingCHeader):
        return add_build_dependency_for_path(
            tree, [os.path.join('/usr/include', error.header)],
            committer=committer, regex=True)

    return False


def build_incrementally(
        local_tree, suffix, build_suite, output_directory, build_command,
        build_changelog_entry='Build for debian-janitor apt repository.',
        committer=None, max_iterations=DEFAULT_MAX_ITERATIONS):
    fixed_errors = []
    while True:
        try:
            return attempt_build(
                local_tree, suffix, build_suite, output_directory,
                build_command, build_changelog_entry)
        except SbuildFailure as e:
            if e.error is None:
                raise
            if e.error in fixed_errors:
                raise
            if max_iterations is not None \
                    and len(fixed_errors) > max_iterations:
                show_error('Last fix did not address the issue. Giving up.')
                raise
            reset_tree(local_tree)
            if not resolve_error(local_tree, e.error, committer=committer):
                raise
            fixed_errors.append(e.error)
            if os.path.exists(os.path.join(output_directory, 'build.log')):
                i = 1
                while os.path.exists(
                        os.path.join(output_directory, 'build.log.%d' % i)):
                    i += 1
                os.rename(os.path.join(output_directory, 'build.log'),
                          os.path.join(output_directory, 'build.log.%d' % i))


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
