#!/usr/bin/python
# Copyright (C) 2019 Jelmer Vernooij <jelmer@jelmer.uk>
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

import re
import sys

__all__ = [
    'SbuildFailure',
    'parse_sbuild_log',
]


class SbuildFailure(Exception):
    """Sbuild failed to run."""

    def __init__(self, stage, description, error=None):
        self.stage = stage
        self.description = description
        self.error = error


SBUILD_FOCUS_SECTION = {
    'build': 'build',
    'run-post-build-commands': 'post build commands',
    'post-build': 'post build',
    'install-deps': 'install package build dependencies',
}


def worker_failure_from_sbuild_log(build_log_path):
    paragraphs = {}
    with open(build_log_path, 'r') as f:
        for title, offsets, lines in parse_sbuild_log(f):
            if title is not None:
                title = title.lower()
            paragraphs[title] = lines
    failed_stage = find_failed_stage(
        paragraphs.get('summary', []))
    focus_section = SBUILD_FOCUS_SECTION.get(failed_stage)
    if failed_stage in ('run-post-build-commands', 'post-build'):
        # We used to run autopkgtest as the only post build
        # command.
        failed_stage = 'autopkgtest'
    description = None
    error = None
    if failed_stage in ('build', 'autopkgtest'):
        offset, description, error = find_build_failure_description(
            paragraphs.get(focus_section, []))
        if error:
            description = str(error)
    if description is None and failed_stage is not None:
        description = 'build failed stage %s' % failed_stage
    if description is None:
        description = 'build failed'
    return SbuildFailure(failed_stage, description, error=error)


def parse_sbuild_log(f):
    begin_offset = 1
    lines = []
    title = None
    sep = '+' + ('-' * 78) + '+'
    lineno = 0
    line = f.readline()
    lineno += 1
    while line:
        if line.strip() == sep:
            l1 = f.readline()
            l2 = f.readline()
            lineno += 2
            if (l1[0] == '|' and
                    l1.strip()[-1] == '|' and l2.strip() == sep):
                end_offset = lineno-3
                # Drop trailing empty lines
                while lines and lines[-1] == '\n':
                    lines.pop(-1)
                    end_offset -= 1
                if lines:
                    yield title, (begin_offset, end_offset), lines
                title = l1.rstrip()[1:-1].strip()
                lines = []
                begin_offset = lineno
            else:
                lines.extend([line, l1, l2])
        else:
            lines.append(line)
        line = f.readline()
        lineno += 1
    yield title, (begin_offset, lineno), lines


def find_failed_stage(lines):
    for line in lines:
        if not line.startswith('Fail-Stage: '):
            continue
        (key, value) = line.split(': ', 1)
        return value.strip()


class MissingPythonModule(object):

    kind = 'missing-python-dep'

    def __init__(self, module, python_version=None, minimum_version=None):
        self.module = module
        self.python_version = python_version
        self.minimum_version = minimum_version

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            other.module == self.module and \
            other.python_version == self.python_version and \
            other.minimum_version == self.minimum_version

    def __str__(self):
        if self.python_version:
            ret = "Missing python %d module: " % self.python_version
        else:
            ret = "Missing python module: "
        ret += self.module
        if self.minimum_version:
            return ret + " (>= %s)" % self.minimum_version
        else:
            return ret

    def __repr__(self):
        return "%s(%r, minimum_version=%r)" % (
            type(self).__name__, self.module, self.minimum_version)


def python2_module_not_found(m):
    return MissingPythonModule(m.group(1), python_version=2)


def python3_module_not_found(m):
    return MissingPythonModule(m.group(1), python_version=3)


def python_reqs_not_found(m):
    expr = m.group(2)
    if '>=' in expr:
        pkg, minimum = expr.split('>=')
        return MissingPythonModule(pkg.strip(), None, minimum.strip())
    if ' ' not in expr:
        return MissingPythonModule(expr, None)
    # Hmm
    return None


class MissingFile(object):

    kind = 'missing-file'

    def __init__(self, path):
        self.path = path

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            self.path == other.path

    def __str__(self):
        return "Missing file: %s" % self.path

    def __repr__(self):
        return "%s(%r, %r)" % (type(self).__name__, self.path)


def file_not_found(m):
    if m.group(1).startswith('/'):
        return MissingFile(m.group(1))
    return None


class MissingGoPackage(object):

    kind = 'missing-go-package'

    def __init__(self, package):
        self.package = package

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            self.package == other.package

    def __str__(self):
        return "Missing Go package: %s" % self.package

    def __repr__(self):
        return "%s(%r, %r)" % (type(self).__name__, self.package)


def missing_go_package(m):
    return MissingGoPackage(m.group(1))


class MissingCHeader(object):

    kind = 'missing-c-header'

    def __init__(self, header):
        self.header = header

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            self.header == other.header

    def __str__(self):
        return "Missing C Header: %s" % self.header

    def __repr__(self):
        return "%s(%r, %r)" % (type(self).__name__, self.header)


def c_header_missing(m):
    return MissingCHeader(m.group(1))


class MissingNodeModule(object):

    kind = 'missing-node-module'

    def __init__(self, module):
        self.module = module

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            self.module == other.module

    def __str__(self):
        return "Missing Node Module: %s" % self.module

    def __repr__(self):
        return "%s(%r, %r)" % (type(self).__name__, self.module)


def node_module_missing(m):
    return MissingNodeModule(m.group(1))


build_failure_regexps = [
    (r'make\[1\]: \*\*\* No rule to make target '
        r'\'(.*)\', needed by \'.*\'\.  Stop\.', file_not_found),
    (r'dh_.*: Cannot find \(any matches for\) "(.*)" \(tried in .*\)',
     None),
    (r'(distutils.errors.DistutilsError|error): '
        r'Could not find suitable distribution '
        r'for Requirement.parse\(\'(.*)\'\)', python_reqs_not_found),
    ('E   ImportError: cannot import name (.*)', python2_module_not_found),
    ('E   ImportError: No module named (.*)', python2_module_not_found),
    ('ModuleNotFoundError: No module named \'(.*)\'',
     python3_module_not_found),
    ('.*: cannot find package "(.*)" in any of:', missing_go_package),
    ('ImportError: No module named (.*)', python2_module_not_found),
    (r'.*:\d+:\d+: fatal error: (.*\.h): No such file or directory',
     c_header_missing),
    (r'Error: Cannot find module \'(.*)\'', node_module_missing),
]

compiled_build_failure_regexps = [
    (re.compile(regexp), cb) for (regexp, cb) in build_failure_regexps]


def find_build_failure_description(lines):
    """Find the key failure line in build output.

    Returns:
      tuple with (line offset, line, error object)
    """
    OFFSET = 20
    for i, line in enumerate(lines[-OFFSET:], 1):
        line = line.strip('\n')
        for regexp, cb in compiled_build_failure_regexps:
            m = regexp.match(line)
            if m:
                if cb:
                    err = cb(m)
                else:
                    err = None
                return max(len(lines) - OFFSET, 0) + i, line, err
    return None, None, None


def main(argv=None):
    import argparse
    parser = argparse.ArgumentParser('janitor.sbuild_log')
    parser.add_argument('path', type=str)
    args = parser.parse_args()

    section_offsets = {}
    section_lines = {}
    with open(args.path, 'r') as f:
        for title, offsets, lines in parse_sbuild_log(f):
            print('Section %s (lines %d-%d)' % (
                title, offsets[0], offsets[1]))
            if title is not None:
                title = title.lower()
            section_offsets[title] = offsets
            section_lines[title] = lines

    failed_stage = find_failed_stage(section_lines.get('summary', []))
    focus_section = SBUILD_FOCUS_SECTION.get(failed_stage)
    if failed_stage == 'run-post-build-commands':
        # We used to run autopkgtest as the only post build
        # command.
        failed_stage = 'autopkgtest'
    if failed_stage:
        print('Failed stage: %s (focus section: %s)' % (
            failed_stage, focus_section))
    if failed_stage in ('build', 'autopkgtest'):
        offset, line, error = find_build_failure_description(
            section_lines.get(focus_section, []))
        if offset:
            print('Failed line: %d:' %
                  (section_offsets[focus_section][0] + offset))
            print(line)
        if error:
            print('Error: %s' % error)


if __name__ == '__main__':
    sys.exit(main(sys.argv))
