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

from debian.deb822 import (
    Deb822,
    PkgRelation,
    )

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
from .trace import note, warning


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


def get_package_for_paths(paths, regex=False):
    candidates = set()
    for path in paths:
        candidates.update(search_apt_file(path, regex=regex))
    if len(candidates) == 0:
        warning('No packages found that contain %r', path)
        return None
    if len(candidates) > 1:
        warning('More than 1 packages found that contain %r: %r',
                path, candidates)
        # Euhr. Pick the one with the shortest name?
        return sorted(candidates, key=len)[0]
    else:
        return candidates.pop()


def get_package_for_python_module(module, python_version):
    if python_version == 'python3':
        paths = [
            os.path.join(
                '/usr/lib/python3.*/dist-packages',
                module.replace('.', '/'),
                '__init__.py'),
            os.path.join(
                '/usr/lib/python3.*/dist-packages',
                module.replace('.', '/') + '.py')]
        regex = True
    elif python_version == 'python2':
        paths = [
            os.path.join(
                '/usr/lib/python2.*/dist-packages',
                module.replace('.', '/'),
                '__init__.py'),
            os.path.join(
                '/usr/lib/python2.*/dist-packages',
                module.replace('.', '/') + '.py')]
        regex = True
    elif python_version == 'pypy':
        paths = [
            os.path.join(
                '/usr/lib/pypy/dist-packages',
                module.replace('.', '/'),
                '__init__.py'),
            os.path.join(
                '/usr/lib/pypy/dist-packages',
                module.replace('.', '/') + '.py')]
        regex = False
    else:
        raise AssertionError(
            'unknown python version %r' % python_version)
    return get_package_for_paths(paths, regex)


def fix_missing_python_module(tree, error, committer=None):
    with tree.get_file('debian/control') as f:
        control = Deb822(f)
    build_depends = PkgRelation.parse_relations(
        control.get('Build-Depends', ''))
    all_build_deps = set()
    for or_deps in build_depends:
        all_build_deps.update(or_dep['name'] for or_dep in or_deps)
    has_pypy_build_deps = any(x.startswith('pypy') for x in all_build_deps)
    has_cpy2_build_deps = any(x.startswith('python-') for x in all_build_deps)
    has_cpy3_build_deps = any(x.startswith('python3-') for x in all_build_deps)
    default = (not has_pypy_build_deps and
               not has_cpy2_build_deps and
               not has_cpy3_build_deps)

    pypy_pkg = get_package_for_python_module(error.module, 'pypy')
    py2_pkg = get_package_for_python_module(error.module, 'python2')
    py3_pkg = get_package_for_python_module(error.module, 'python3')

    extra_build_deps = []
    if error.python_version == 2:
        if has_pypy_build_deps:
            extra_build_deps.append(pypy_pkg)
        if has_cpy2_build_deps or default:
            extra_build_deps.append(py2_pkg)
    elif error.python_version == 3:
        extra_build_deps.append(py3_pkg)
    else:
        if has_cpy3_build_deps or default:
            extra_build_deps(py3_pkg)
        if has_cpy2_build_deps or default:
            extra_build_deps(py2_pkg)
        if has_pypy_build_deps:
            extra_build_deps.append(pypy_pkg)

    for dep_pkg in extra_build_deps:
        if not add_build_dependency(
                tree, dep_pkg, error.minimum_version, committer=committer):
            return False
    return True


def fix_missing_c_header(tree, error, committer=None):
    package = get_package_for_paths(
        [os.path.join('/usr/include', error.header)], regex=False)
    if package is None:
        package = get_package_for_paths(
            [os.path.join('/usr/include', '.*', error.header)], regex=True)
    if package is None:
        return False
    return add_build_dependency(tree, package, committer=committer)


FIXERS = [
    (MissingPythonModule, fix_missing_python_module),
    (MissingCHeader, fix_missing_c_header),
]


def resolve_error(tree, error, committer=None):
    for error_cls, fixer in FIXERS:
        if isinstance(error, error_cls):
            return fixer(tree, error, committer)
    warning('No fixer found for %r', error)
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
                warning('Last fix did not address the issue. Giving up.')
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
