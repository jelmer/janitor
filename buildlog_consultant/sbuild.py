#!/usr/bin/python
# Copyright (C) 2019-2021 Jelmer Vernooij <jelmer@jelmer.uk>
# encoding: utf-8
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

from debian.deb822 import PkgRelation
import os
import posixpath
import re
import sys
from typing import List, Tuple, Iterator, BinaryIO, Optional, Dict, Union
import textwrap
import yaml

import logging

__all__ = [
    'SbuildFailure',
    'parse_sbuild_log',
]

logger = logging.getLogger(__name__)


class SbuildFailure(Exception):
    """Sbuild failed to run."""

    def __init__(self, stage: str,
                 description: str,
                 error: Optional['Problem'] = None,
                 context: Optional[
                     Union[Tuple[str], Tuple[str, Optional[str]]]] = None):
        self.stage = stage
        self.description = description
        self.error = error
        self.context = context

    def __repr__(self):
        return '%s(%r, %r, error=%r, context=%r)' % (
            type(self).__name__, self.stage, self.description,
            self.error, self.context)


SBUILD_FOCUS_SECTION = {
    'build': 'build',
    'run-post-build-commands': 'post build commands',
    'post-build': 'post build',
    'install-deps': 'install package build dependencies',
    'explain-bd-uninstallable': 'install package build dependencies',
    'apt-get-update': 'update chroot',
    'arch-check': 'check architectures',
    'check-space': 'cleanup',
}


class Problem(object):

    kind: str
    is_global: bool = False


class DpkgSourceLocalChanges(Problem):

    kind = 'unexpected-local-upstream-changes'

    def __init__(self, files=None):
        self.files = files

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.files == other.files

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.files)

    def __str__(self):
        if self.files:
            return "Tree has local changes: %r" % self.files
        else:
            return "Tree has local changes"


class DpkgSourceUnrepresentableChanges(Problem):

    kind = 'unrepresentable-local-changes'

    def __str__(self):
        return "Tree has unrepresentable local changes."


class DpkgUnwantedBinaryFiles(Problem):

    kind = 'unwanted-binary-files'

    def __str__(self):
        return "Tree has unwanted binary files."


class DpkgBinaryFileChanged(Problem):

    kind = 'changed-binary-files'

    def __init__(self, paths):
        self.paths = paths

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.paths)

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.paths == other.paths

    def __str__(self):
        return "Tree has binary files with changes: %r" % self.paths


class MissingControlFile(Problem):

    kind = 'missing-control-file'

    def __init__(self, path):
        self.path = path

    def __eq__(self, other):
        return isinstance(self, type(other)) and self.path == other.path

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.path)

    def __str__(self):
        return "Tree is missing control file %s" % self.path


class UnableToFindUpstreamTarball(Problem):

    kind = 'unable-to-find-upstream-tarball'

    def __init__(self, package, version):
        self.package = package
        self.version = version

    def __str__(self):
        return ("Unable to find the needed upstream tarball for "
                "%s, version %s." % (self.package, self.version))


class PatchApplicationFailed(Problem):

    kind = 'patch-application-failed'

    def __init__(self, patchname):
        self.patchname = patchname

    def __str__(self):
        return "Patch application failed: %s" % self.patchname


class SourceFormatUnbuildable(Problem):

    kind = 'source-format-unbuildable'

    def __init__(self, source_format):
        self.source_format = source_format

    def __str__(self):
        return "Source format %s unbuildable" % self.source_format


class SourceFormatUnsupported(Problem):

    kind = 'unsupported-source-format'

    def __init__(self, source_format):
        self.source_format = source_format

    def __str__(self):
        return "Source format %r unsupported" % self.source_format


class PatchFileMissing(Problem):

    kind = 'patch-file-missing'

    def __init__(self, path):
        self.path = path

    def __str__(self):
        return "Patch file %s missing" % self.path


class UnknownMercurialExtraFields(Problem):

    kind = 'unknown-mercurial-extra-fields'

    def __init__(self, field):
        self.field = field

    def __str__(self):
        return "Unknown Mercurial extra fields: %s" % self.field


class UpstreamPGPSignatureVerificationFailed(Problem):

    kind = 'upstream-pgp-signature-verification-failed'

    def __init__(self):
        pass

    def __str__(self):
        return "Unable to verify the PGP signature on the upstream source"


class UScanRequestVersionMissing(Problem):

    kind = 'uscan-requested-version-missing'

    def __init__(self, version):
        self.version = version

    def __str__(self):
        return "UScan can not find requested version %s." % self.version

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.version)

    def __eq__(self, other):
        return isinstance(self, type(other)) and self.version == other.version


class DebcargoFailure(Problem):

    kind = 'debcargo-failed'

    def __init__(self):
        pass

    def __str__(self):
        return "Debcargo failed"

    def __repr__(self):
        return "%s()" % type(self).__name__

    def __eq__(self, other):
        return isinstance(other, type(self))


class UScanFailed(Problem):

    kind = 'uscan-failed'

    def __init__(self, url, reason):
        self.url = url
        self.reason = reason

    def __str__(self):
        return "UScan failed to download %s: %s." % (
            self.url, self.reason)

    def __repr__(self):
        return "%s(%r, %r)" % (type(self).__name__, self.url, self.reason)

    def __eq__(self, other):
        return (
            isinstance(self, type(other)) and
            self.url == other.url and
            self.reason == other.reason)


class InconsistentSourceFormat(Problem):

    kind = 'inconsistent-source-format'

    def __init__(self):
        pass

    def __eq__(self, other):
        return isinstance(other, type(self))

    def __str__(self):
        return "Inconsistent source format between version and source format"


class UpstreamMetadataFileParseError(Problem):

    kind = 'debian-upstream-metadata-invalid'

    def __init__(self, path, reason):
        self.path = path
        self.reason = reason

    def __eq__(self, other):
        return (isinstance(other, type(self)) and
                self.path == other.path and
                self.reason == other.reason)

    def __repr__(self):
        return "%s(%r, %r)" % (type(self).__name__, self.path, self.reason)

    def __str__(self):
        return "%s is invalid" % self.path


class DpkgSourcePackFailed(Problem):

    kind = 'dpkg-source-pack-failed'

    def __init__(self, reason=None):
        self.reason = reason

    def __eq__(self, other):
        return isinstance(other, type(self)) and other.reason == self.reason

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.reason)

    def __str__(self):
        if self.reason:
            return "Packing source directory failed: %s" % self.reason
        else:
            return "Packing source directory failed."


def find_preamble_failure_description(lines: List[str]) -> Tuple[
        Optional[int], Optional[str], Optional[Problem]]:
    ret: Tuple[Optional[int], Optional[str], Optional[Problem]] = (
        None, None, None)
    OFFSET = 20
    err: Problem
    for i in range(1, OFFSET):
        lineno = len(lines) - i
        if lineno < 0:
            break
        line = lines[lineno].strip('\n')
        if line.startswith(
                'dpkg-source: error: aborting due to unexpected upstream '
                'changes, see '):
            j = lineno - 1
            files: List[str] = []
            while j > 0:
                if lines[j] == (
                        'dpkg-source: info: local changes detected, '
                        'the modified files are:\n'):
                    error = DpkgSourceLocalChanges(files)
                    return lineno + 1, str(error), error
                files.append(lines[j].strip())
                j -= 1
            err = DpkgSourceLocalChanges()
            return lineno + 1, str(error), err
        if line == 'dpkg-source: error: unrepresentable changes to source':
            err = DpkgSourceUnrepresentableChanges()
            return lineno + 1, line, err
        if re.match('dpkg-source: error: detected ([0-9]+) unwanted binary '
                    'file.*', line):
            err = DpkgUnwantedBinaryFiles()
            return lineno + 1, line, err
        m = re.match('dpkg-source: error: cannot read (.*/debian/control): '
                     'No such file or directory', line)
        if m:
            err = MissingControlFile(m.group(1))
            return lineno + 1, line, err
        m = re.match(
            'dpkg-source: error: .*: No space left on device', line)
        if m:
            err = NoSpaceOnDevice()
            return lineno + 1, line, err
        m = re.match(
            'tar: .*: Cannot write: No space left on device', line)
        if m:
            err = NoSpaceOnDevice()
            return lineno + 1, line, err
        m = re.match(
            'dpkg-source: error: cannot represent change to (.*): '
            'binary file contents changed', line)
        if m:
            err = DpkgBinaryFileChanged([m.group(1)])
            return lineno + 1, line, err

        m = re.match(
            r'dpkg-source: error: source package format \'(.*)\' is not '
            r'supported: Can\'t locate (.*) in \@INC '
            r'\(you may need to install the (.*) module\) '
            r'\(\@INC contains: (.*)\) at \(eval [0-9]+\) line [0-9]+\.', line)
        if m:
            err = SourceFormatUnsupported(m.group(1))
            return lineno + 1, line, err

        m = re.match('dpkg-source: error: (.*)', line)
        if m:
            err = DpkgSourcePackFailed(m.group(1))
            ret = lineno + 1, line, err

        m = re.match(
            'E: Failed to package source directory (.*)', line)
        if m:
            err = DpkgSourcePackFailed()
            ret = lineno + 1, line, err

    return ret


BRZ_ERRORS = [
    ('Unable to find the needed upstream tarball for '
     'package (.*), version (.*)\\.',
     lambda m: UnableToFindUpstreamTarball(m.group(1), m.group(2))),
    ('Unknown mercurial extra fields in (.*): b\'(.*)\'.',
     lambda m: UnknownMercurialExtraFields(m.group(2))),
    ('UScan failed to run: OpenPGP signature did not verify..',
     lambda m: UpstreamPGPSignatureVerificationFailed()),
    (r'Inconsistency between source format and version: '
     r'version is( not)? native, format is( not)? native\.',
     lambda m: InconsistentSourceFormat()),
    (r'UScan failed to run: In (.*) no matching hrefs '
     'for version (.*) in watch line',
     lambda m: UScanRequestVersionMissing(m.group(2))),
    (r'UScan failed to run: In directory ., downloading \s+'
     r'(.*) failed: (.*)',
     lambda m: UScanFailed(m.group(1), m.group(2))),
    (r'UScan failed to run: In watchfile debian/watch, '
     r'reading webpage\n  (.*) failed: (.*)',
     lambda m: UScanFailed(m.group(1), m.group(2))),
    (r'Unable to parse upstream metadata file (.*): (.*)',
     lambda m: UpstreamMetadataFileParseError(m.group(1), m.group(2))),
    (r'Debcargo failed to run\.', lambda m: DebcargoFailure()),
]


_BRZ_ERRORS = [(re.compile(r), fn) for (r, fn) in BRZ_ERRORS]


def parse_brz_error(line: str) -> Tuple[Optional[Problem], str]:
    error: Problem
    line = line.strip()
    for search_re, fn in _BRZ_ERRORS:
        m = search_re.match(line)
        if m:
            error = fn(m)
            return (error, str(error))
    if line.startswith('UScan failed to run'):
        return (None, line)
    return (None, line.split('\n')[0])


class MissingRevision(Problem):

    kind = 'missing-revision'

    def __init__(self, revision):
        self.revision = revision

    def __str__(self):
        return "Missing revision: %r" % self.revision


def find_creation_session_error(lines):
    ret = None, None, None
    for i in range(len(lines) - 1, 0, -1):
        line = lines[i]
        if line.startswith('E: '):
            ret = i + 1, line, None
        if line.endswith(': No space left on device\n'):
            return i + 1, line, NoSpaceOnDevice()

    return ret


def worker_failure_from_sbuild_log(f: BinaryIO) -> SbuildFailure:
    paragraphs = {}
    for title, offsets, lines in parse_sbuild_log(f):
        if title is not None:
            title = title.lower()
        paragraphs[title] = lines
    if len(paragraphs) == 1:
        offset, description, error = find_preamble_failure_description(
            paragraphs[None])
        if error:
            return SbuildFailure('unpack', description, error)

    failed_stage = find_failed_stage(paragraphs.get('summary', []))
    focus_section = SBUILD_FOCUS_SECTION.get(failed_stage)
    if failed_stage in ('run-post-build-commands', 'post-build'):
        # We used to run autopkgtest as the only post build
        # command.
        failed_stage = 'autopkgtest'
    description = None
    context: Optional[Union[Tuple[str], Tuple[str, Optional[str]]]] = None
    error = None
    section_lines = paragraphs.get(focus_section, [])
    if failed_stage == 'create-session':
        offset, description, error = find_creation_session_error(section_lines)
        if error:
            context = ('create-session', )
    if failed_stage == 'build':
        section_lines = strip_useless_build_tail(section_lines)
        offset, description, error = find_build_failure_description(
            section_lines)
        if error:
            description = str(error)
            context = ('build', )
    if failed_stage == 'autopkgtest':
        section_lines = strip_useless_build_tail(section_lines)
        (apt_offset, testname, apt_error, apt_description) = (
            find_autopkgtest_failure_description(section_lines))
        if apt_error and not error:
            error = apt_error
            if not apt_description:
                apt_description = str(apt_error)
        if apt_description and not description:
            description = apt_description
        if apt_offset is not None:
            offset = apt_offset
        context = ('autopkgtest', testname)
    if failed_stage == 'apt-get-update':
        focus_section, offset, description, error = (
                find_apt_get_update_failure(paragraphs))
        if error:
            description = str(error)
    if failed_stage in ('install-deps', 'explain-bd-uninstallable'):
        (focus_section, offset, line,
         error) = find_install_deps_failure_description(paragraphs)
        if error:
            description = str(error)
        elif line:
            if line.startswith('E: '):
                description = line[3:]
            else:
                description = line
    if failed_stage == 'arch-check':
        (offset, line, error) = find_arch_check_failure_description(
                section_lines)
        if error:
            description = str(error)
    if failed_stage == 'check-space':
        (offset, line, error) = find_check_space_failure_description(
                section_lines)
        if error:
            description = str(error)
    if description is None and failed_stage is not None:
        description = 'build failed stage %s' % failed_stage
    if description is None:
        description = 'build failed'
        context = ('buildenv', )
        if list(paragraphs.keys()) == [None]:
            for line in reversed(paragraphs[None]):
                m = re.match(
                    'Patch (.*) does not apply \\(enforce with -f\\)\n', line)
                if m:
                    patchname = m.group(1).split('/')[-1]
                    error = PatchApplicationFailed(patchname)
                    description = 'Patch %s failed to apply' % patchname
                    break
                m = re.match(
                    r'dpkg-source: error: LC_ALL=C patch .* '
                    r'--reject-file=- < .*\/debian\/patches\/([^ ]+) '
                    r'subprocess returned exit status 1',
                    line)
                if m:
                    patchname = m.group(1)
                    error = PatchApplicationFailed(patchname)
                    description = 'Patch %s failed to apply' % patchname
                    break
                m = re.match(
                    'dpkg-source: error: '
                    'can\'t build with source format \'(.*)\': '
                    '(.*)', line)
                if m:
                    error = SourceFormatUnbuildable(m.group(1))
                    description = m.group(2)
                    break
                m = re.match(
                    'dpkg-source: error: cannot read (.*): '
                    'No such file or directory',
                    line)
                if m:
                    error = PatchFileMissing(m.group(1).split('/', 1)[1])
                    description = 'Patch file %s in series but missing' % (
                        error.path)
                    break
                m = re.match(
                    'dpkg-source: error: '
                    'source package format \'(.*)\' is not supported: '
                    '(.*)', line)
                if m:
                    (_, description, error) = find_build_failure_description(
                        [m.group(2)])
                    if error is None:
                        error = SourceFormatUnsupported(m.group(1))
                    if description is None:
                        description = m.group(2)
                    break
                m = re.match('dpkg-source: error: (.*)', line)
                if m:
                    error = None
                    description = m.group(1)
                    break
                m = re.match(
                    'breezy.errors.NoSuchRevision: '
                    '(.*) has no revision b\'(.*)\'', line)
                if m:
                    error = MissingRevision(m.group(2).encode())
                    description = "Revision %r is not present" % (
                        error.revision)
                    break
            else:
                for i in range(len(paragraphs[None]) - 1, 0, -1):
                    line = paragraphs[None][i]
                    if line.startswith('brz: ERROR: '):
                        rest = [line[len('brz: ERROR: '):]]
                        for n in paragraphs[None][i+1:]:
                            if n.startswith(' '):
                                rest.append(n)
                        (error, description) = parse_brz_error(''.join(rest))
                        break

    return SbuildFailure(
        failed_stage, description, error=error, context=context)


def parse_sbuild_log(
        f: BinaryIO
        ) -> Iterator[Tuple[Optional[str], Tuple[int, int], List[str]]]:
    begin_offset = 1
    lines: List[str] = []
    title = None
    sep = b'+' + (b'-' * 78) + b'+'
    lineno = 0
    line = f.readline()
    lineno += 1
    while line:
        if line.strip() == sep:
            l1 = f.readline()
            l2 = f.readline()
            lineno += 2
            if (l1.startswith(b'|') and
                    l1.strip().endswith(b'|') and l2.strip() == sep):
                end_offset = lineno-3
                # Drop trailing empty lines
                while lines and lines[-1] == '\n':
                    lines.pop(-1)
                    end_offset -= 1
                if lines:
                    yield title, (begin_offset, end_offset), lines
                title = l1.rstrip()[1:-1].strip().decode(errors='replace')
                lines = []
                begin_offset = lineno
            else:
                lines.extend([
                    line.decode(errors='replace'),
                    l1.decode(errors='replace'),
                    l2.decode(errors='replace')])
        else:
            lines.append(line.decode(errors='replace'))
        line = f.readline()
        lineno += 1
    yield title, (begin_offset, lineno), lines


def find_failed_stage(lines: List[str]) -> Optional[str]:
    for line in lines:
        if not line.startswith('Fail-Stage: '):
            continue
        (key, value) = line.split(': ', 1)
        return value.strip()
    return None


class MissingPythonModule(Problem):

    kind = 'missing-python-module'

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
        return "%s(%r, python_version=%r, minimum_version=%r)" % (
            type(self).__name__, self.module, self.python_version,
            self.minimum_version)


class MissingPythonDistribution(Problem):

    kind = 'missing-python-distribution'

    def __init__(self, distribution, python_version=None,
                 minimum_version=None):
        self.distribution = distribution
        self.python_version = python_version
        self.minimum_version = minimum_version

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            other.distribution == self.distribution and \
            other.python_version == self.python_version and \
            other.minimum_version == self.minimum_version

    def __str__(self):
        if self.python_version:
            ret = "Missing python %d distribution: " % self.python_version
        else:
            ret = "Missing python distribution: "
        ret += self.distribution
        if self.minimum_version:
            return ret + " (>= %s)" % self.minimum_version
        else:
            return ret

    def __repr__(self):
        return "%s(%r, python_version=%r, minimum_version=%r)" % (
            type(self).__name__, self.distribution, self.python_version,
            self.minimum_version)


def python_module_not_found(m):
    try:
        return MissingPythonModule(m.group(2), python_version=None)
    except IndexError:
        return MissingPythonModule(m.group(1), python_version=None)


def python2_module_not_found(m):
    return MissingPythonModule(m.group(1), python_version=2)


def python3_module_not_found(m):
    return MissingPythonModule(m.group(1), python_version=3)


def sphinx_module_not_found(m):
    module = m.group(1).strip("'")
    return MissingPythonModule(module)


def python_reqs_not_found(m):
    expr = m.group(2)
    if '>=' in expr:
        pkg, minimum = expr.split('>=')
        return MissingPythonDistribution(pkg.strip(), None, minimum.strip())
    expr = expr.split(';')[0]
    if ' ' not in expr:
        return MissingPythonDistribution(expr, None)
    # Hmm
    return None


def python2_reqs_not_found(m):
    expr = m.group(1)
    if '>=' in expr:
        pkg, minimum = expr.split('>=')
        return MissingPythonModule(pkg.strip(), 2, minimum.strip())
    if ' ' not in expr:
        return MissingPythonModule(expr, 2)
    # Hmm
    return None


def pkg_resources_distribution_not_found(m):
    expr = m.group(1)
    if '>=' in expr:
        pkg, minimum = expr.split('>=')
        return MissingPythonDistribution(pkg.strip(), None, minimum.strip())
    return None


class MissingFile(Problem):

    kind = 'missing-file'

    def __init__(self, path):
        self.path = path

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            self.path == other.path

    def __str__(self):
        return "Missing file: %s" % self.path

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.path)


def file_not_found(m):
    if (m.group(1).startswith('/') and
            not m.group(1).startswith('/<<PKGBUILDDIR>>')):
        return MissingFile(m.group(1))
    return None


def webpack_file_missing(m):
    path = posixpath.join(m.group(2), m.group(1))
    if (path.startswith('/') and
            not path.startswith('/<<PKGBUILDDIR>>')):
        return MissingFile(path)
    return None


class MissingJDKFile(Problem):

    kind = 'missing-jdk-file'

    def __init__(self, jdk_path, filename):
        self.jdk_path = jdk_path
        self.filename = filename

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            self.jdk_path == other.jdk_path and \
            self.filename == other.filename

    def __str__(self):
        return "Missing JDK file %s (JDK Path: %s)" % (
            self.filename, self.jdk_path)

    def __repr__(self):
        return "%s(%r, %r)" % (
            type(self).__name__, self.jdk_path, self.filename)


def jdk_file_missing(m):
    return MissingJDKFile(m.group(2), m.group(1))


def interpreter_missing(m):
    if m.group(1).startswith('/'):
        if m.group(1).startswith('/<<PKGBUILDDIR>>'):
            return None
        return MissingFile(m.group(1))
    if '/' in m.group(1):
        return None
    return MissingCommand(m.group(1))


class MissingSprocketsFile(Problem):

    kind = 'missing-sprockets-file'

    def __init__(self, name, content_type):
        self.name = name
        self.content_type = content_type

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            self.name == other.name and \
            self.content_type == other.content_type

    def __str__(self):
        return "Missing sprockets file: %s (type: %s)" % (
            self.name, self.content_type)

    def __repr__(self):
        return "%s(%r, %r)" % (
            type(self).__name__, self.name, self.content_type)


def sprockets_file_not_found(m):
    return MissingSprocketsFile(m.group(1), m.group(2))


class MissingGoPackage(Problem):

    kind = 'missing-go-package'

    def __init__(self, package):
        self.package = package

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            self.package == other.package

    def __str__(self):
        return "Missing Go package: %s" % self.package

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.package)


def missing_go_package(m):
    return MissingGoPackage(m.group(1))


class MissingCHeader(Problem):

    kind = 'missing-c-header'

    def __init__(self, header):
        self.header = header

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            self.header == other.header

    def __str__(self):
        return "Missing C Header: %s" % self.header

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.header)


def c_header_missing(m):
    return MissingCHeader(m.group(1))


class MissingNodeModule(Problem):

    kind = 'missing-node-module'

    def __init__(self, module):
        self.module = module

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            self.module == other.module

    def __str__(self):
        return "Missing Node Module: %s" % self.module

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.module)


def node_module_missing(m):
    if m.group(1).startswith('/<<PKGBUILDDIR>>/'):
        return None
    if m.group(1).startswith('./'):
        return None
    return MissingNodeModule(m.group(1))


class MissingCommand(Problem):

    kind = 'command-missing'

    def __init__(self, command):
        self.command = command

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            self.command == other.command

    def __str__(self):
        return "Missing command: %s" % self.command

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.command)


class MissingConfigure(Problem):

    kind = 'missing-configure'

    def __eq__(self, other):
        return isinstance(other, type(self))

    def __str__(self):
        return "Missing configure script"

    def __repr__(self):
        return "%s()" % (type(self).__name__, )


def command_missing(m):
    command = m.group(1)
    if 'PKGBUILDDIR' in command:
        return None
    if command == './configure':
        return MissingConfigure()
    if command.startswith('./') or command.startswith('../'):
        return None
    if command == 'debian/rules':
        return None
    return MissingCommand(command)


class MissingJavaScriptRuntime(Problem):

    kind = 'javascript-runtime-missing'

    def __init__(self):
        pass

    def __eq__(self, other):
        return isinstance(other, type(self))

    def __repr__(self):
        return "%s()" % (type(self).__name__, )

    def __str__(self):
        return "Missing JavaScript Runtime"


def javascript_runtime_missing(m):
    return MissingJavaScriptRuntime()


class MissingPkgConfig(Problem):

    kind = 'pkg-config-missing'

    def __init__(self, module, minimum_version=None):
        self.module = module
        self.minimum_version = minimum_version

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            self.module == other.module and \
            self.minimum_version == other.minimum_version

    def __str__(self):
        if self.minimum_version:
            return "Missing pkg-config file: %s (>= %s)" % (
                self.module, self.minimum_version)
        else:
            return "Missing pkg-config file: %s" % self.module

    def __repr__(self):
        return "%s(%r, minimum_version=%r)" % (
            type(self).__name__, self.module, self.minimum_version)


def pkg_config_missing(m):
    expr = m.group(1).strip().split('\t')[0]
    if '>=' in expr:
        pkg, minimum = expr.split('>=', 1)
        return MissingPkgConfig(pkg.strip(), minimum.strip())
    if ' ' not in expr:
        return MissingPkgConfig(expr)
    # Hmm
    return None


def meson_pkg_config_missing(m):
    return MissingPkgConfig(m.group(3))


def meson_pkg_config_too_low(m):
    return MissingPkgConfig(m.group(3), m.group(4))


def cmake_pkg_config_missing(m):
    return MissingPkgConfig(m.group(1))


class CMakeFilesMissing(Problem):

    kind = 'cmake-files-missing'

    def __init__(self, filenames):
        self.filenames = filenames

    def __eq__(self, other):
        return (isinstance(self, type(other)) and
                self.filenames == other.filenames)

    def __str__(self):
        return "Missing CMake package configuration files: %r" % (
            self.filenames, )

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.filenames)


class DhWithOrderIncorrect(Problem):

    kind = 'debhelper-argument-order'

    def __eq__(self, other):
        return isinstance(other, type(self))

    def __str__(self):
        return "dh argument order is incorrect"


def dh_with_order(m):
    return DhWithOrderIncorrect()


class NoSpaceOnDevice(Problem):

    kind = 'no-space-on-device'
    is_global = True

    def __eq__(self, other):
        return isinstance(other, type(self))

    def __str__(self):
        return "No space on device"


def install_no_space(m):
    return NoSpaceOnDevice()


class MissingPerlModule(Problem):

    kind = 'missing-perl-module'

    def __init__(self, filename, module, inc=None):
        self.filename = filename
        self.module = module
        self.inc = inc

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            other.module == self.module and \
            other.filename == self.filename and \
            other.inc == self.inc

    def __str__(self):
        return "Missing Perl module: %s (filename: %r, inc: %r)" % (
            self.module, self.filename, self.inc)

    def __repr__(self):
        return "%s(%r, %r, %r)" % (
            type(self).__name__, self.filename, self.module, self.inc)


def perl_missing_module(m):
    return MissingPerlModule(
        m.group(1) + '.pm', m.group(2), m.group(3).split(' '))


def perl_expand_failed(m):
    return MissingPerlModule(None, m.group(1).strip().strip("'"), None)


def perl_missing_plugin(m):
    return MissingPerlModule(None, m.group(1), None)


def perl_missing_author_dep(m):
    return MissingPerlModule(None, m.group(1), None)


class MissingPerlFile(Problem):

    kind = 'missing-perl-file'

    def __init__(self, filename, inc=None):
        self.filename = filename
        self.inc = inc

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
            other.filename == self.filename and \
            other.inc == self.inc

    def __str__(self):
        return "Missing Perl file: %s (inc: %r)" % (
            self.filename, self.inc)

    def __repr__(self):
        return "%s(%r, %r)" % (type(self).__name__, self.filename, self.inc)


def perl_missing_file(m):
    return MissingPerlFile(m.group(1), m.group(2).split(' '))


def perl_file_not_found(m):
    return MissingPerlFile(m.group(1))


class MissingMavenArtifacts(Problem):

    kind = 'missing-maven-artifacts'

    def __init__(self, artifacts):
        self.artifacts = artifacts

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
                self.artifacts == other.artifacts

    def __str__(self):
        return "Missing maven artifacts: %r" % self.artifacts

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.artifacts)


class DhUntilUnsupported(Problem):

    kind = 'dh-until-unsupported'

    def __eq__(self, other):
        return isinstance(other, type(self))

    def __str__(self):
        return "dh --until is no longer supported"

    def __repr__(self):
        return "%s()" % (type(self).__name__, )


def dh_until_unsupported(m):
    return DhUntilUnsupported()


class DhAddonLoadFailure(Problem):

    kind = 'dh-addon-load-failure'

    def __init__(self, name, path):
        self.name = name
        self.path = path

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
                self.name == other.name and \
                self.path == other.path

    def __str__(self):
        return "dh addon loading failed: %s" % self.name

    def __repr__(self):
        return "%s(%r, %r)" % (type(self).__name__, self. name, self.path)


def dh_addon_load_failure(m):
    return DhAddonLoadFailure(m.group(1), m.group(2))


class DhMissingUninstalled(Problem):

    kind = 'dh-missing-uninstalled'

    def __init__(self, missing_file):
        self.missing_file = missing_file

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
                self.missing_file == other.missing_file

    def __str__(self):
        return "File built by Debian not installed: %r" % self.missing_file

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.missing_file)


def dh_missing_uninstalled(m):
    return DhMissingUninstalled(m.group(2))


class DhLinkDestinationIsDirectory(Problem):

    kind = 'dh-link-destination-is-directory'

    def __init__(self, path):
        self.path = path

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.path == other.path

    def __str__(self):
        return "Link destination %s is directory" % self.path

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.path)


def dh_link_destination_is_dir(m):
    return DhLinkDestinationIsDirectory(m.group(1))


def maven_missing_artifact(m):
    artifacts = m.group(1).split(',')
    return MissingMavenArtifacts([a.strip() for a in artifacts])


def maven_missing_plugin(m):
    return MissingMavenArtifacts([m.group(1)])


class MissingXmlEntity(Problem):

    kind = 'missing-xml-entity'

    def __init__(self, url):
        self.url = url

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.url == other.url

    def __str__(self):
        return 'Missing XML entity: %s' % self.url

    def __repr__(self):
        return '%s(%r)' % (type(self).__name__, self.url)


def xsltproc_network_entity(m):
    return MissingXmlEntity(m.group(1))


class CcacheError(Problem):

    kind = 'ccache-error'

    def __init__(self, error):
        self.error = error

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.error == other.error

    def __str__(self):
        return 'ccache error: %s' % self.error

    def __repr__(self):
        return '%s(%r)' % (type(self).__name__, self.error)


def ccache_error(m):
    return CcacheError(m.group(1))


class MissingLibrary(Problem):

    kind = 'missing-library'

    def __init__(self, library):
        self.library = library

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.library == other.library

    def __str__(self):
        return 'missing library: %s' % self.library

    def __repr__(self):
        return '%s(%r)' % (type(self).__name__, self.library)


def ld_missing_lib(m):
    return MissingLibrary(m.group(1))


class MissingRubyGem(Problem):

    kind = 'missing-ruby-gem'

    def __init__(self, gem: str, version: Optional[str] = None):
        self.gem = gem
        self.version = version

    def __eq__(self, other):
        return (isinstance(other, type(self)) and
                self.gem == other.gem and
                self.version == other.version)

    def __str__(self):
        if self.version:
            return 'missing ruby gem: %s (>= %s)' % (self.gem, self.version)
        else:
            return 'missing ruby gem: %s' % self.gem

    def __repr__(self):
        return '%s(%r, %r)' % (type(self).__name__, self.gem, self.version)


def ruby_missing_gem(m):
    minimum_version = None
    for grp in m.group(2).split(','):
        (cond, val) = grp.strip().split(' ', 1)
        if cond == '>=':
            minimum_version = val
            break
        if cond == '~>':
            minimum_version = val
    return MissingRubyGem(m.group(1), minimum_version)


class MissingRubyFile(Problem):

    kind = 'missing-ruby-file'

    def __init__(self, filename):
        self.filename = filename

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.filename)

    def __str__(self):
        return "Missing ruby file: %s" % (self.filename, )

    def __eq__(self, other):
        return isinstance(self, type(other)) and \
            self.filename == other.filename


def ruby_missing_name(m):
    return MissingRubyFile(m.group(1))


class MissingPhpClass(Problem):

    kind = 'missing-php-class'

    def __init__(self, php_class):
        self.php_class = php_class

    def __eq__(self, other):
        return (isinstance(other, type(self)) and
                self.php_class == other.php_class)

    def __str__(self):
        return 'missing PHP class: %s' % self.php_class

    def __repr__(self):
        return '%s(%r)' % (type(self).__name__, self.php_class)


def php_missing_class(m):
    return MissingPhpClass(m.group(1))


class MissingJavaClass(Problem):

    kind = 'missing-java-class'

    def __init__(self, classname):
        self.classname = classname

    def __eq__(self, other):
        return (isinstance(other, type(self)) and
                self.classname == other.classname)

    def __str__(self):
        return 'missing java class: %s' % self.classname

    def __repr__(self):
        return '%s(%r)' % (type(self).__name__, self.classname)


def java_missing_class(m):
    return MissingJavaClass(m.group(1))


class MissingRPackage(Problem):

    kind = 'missing-r-package'

    def __init__(self, package, minimum_version=None):
        self.package = package
        self.minimum_version = minimum_version

    def __eq__(self, other):
        return (isinstance(other, type(self)) and
                self.package == other.package and
                self.minimum_version == other.minimum_version)

    def __str__(self):
        if self.minimum_version:
            return 'missing R package: %s (>= %s)' % (
                self.package, self.minimum_version)
        else:
            return 'missing R package: %s' % self.package

    def __repr__(self):
        return '%s(%r, %r)' % (
            type(self).__name__, self.package, self.minimum_version)


def r_missing_package(m):
    fragment = m.group(1)
    deps = [dep.strip('‘’ ') for dep in fragment.split(',')]
    return MissingRPackage(deps[0])


def r_too_old(m):
    package = m.group(1)
    new_version = m.group(3)
    return MissingRPackage(package, new_version)


class FailedGoTest(Problem):

    kind = 'failed-go-test'

    def __init__(self, test):
        self.test = test

    def __eq__(self, other):
        return (isinstance(other, type(self)) and
                self.test == other.test)

    def __str__(self):
        return 'failed go test: %s' % self.test

    def __repr__(self):
        return '%s(%r)' % (type(self).__name__, self.test)


def go_test_failed(m):
    return FailedGoTest(m.group(1))


class DebhelperPatternNotFound(Problem):

    kind = 'debhelper-pattern-not-found'

    def __init__(self, pattern, tool, directories):
        self.pattern = pattern
        self.tool = tool
        self.directories = directories

    def __eq__(self, other):
        return (isinstance(other, type(self)) and
                self.pattern == other.pattern and
                self.tool == other.tool and
                self.directories == other.directories)

    def __str__(self):
        return 'debhelper (%s) expansion failed for %r (directories: %r)' % (
            self.tool, self.pattern, self.directories)

    def __repr__(self):
        return '%s(%r, %r, %r)' % (
            type(self).__name__, self.pattern, self.tool, self.directories)


def dh_pattern_no_matches(m):
    return DebhelperPatternNotFound(
        m.group(2), m.group(1), [d.strip() for d in m.group(3).split(',')])


class GnomeCommonMissing(Problem):

    kind = 'gnome-common-missing'

    def __init__(self) -> None:
        pass

    def __eq__(self, other):
        return isinstance(other, type(self))

    def __str__(self):
        return 'gnome-common is not installed'

    def __repr__(self):
        return '%s()' % (type(self).__name__, )


def gnome_common_missing(m):
    return GnomeCommonMissing()


class MissingXfceDependency(Problem):

    kind = 'missing-xfce-dependency'

    def __init__(self, package):
        self.package = package

    def __eq__(self, other):
        return (isinstance(other, type(self)) and
                self.package == other.package)

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.package)

    def __str__(self):
        return "Missing XFCE build dependency: %s" % (self.package)


def xfce_dependency_missing(m):
    return MissingXfceDependency(m.group(1))


class MissingAutomakeInput(Problem):

    kind = 'missing-automake-input'

    def __init__(self, path):
        self.path = path

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.path == other.path

    def __str__(self):
        return 'automake input file %s missing' % self.path

    def __repr__(self):
        return '%s(%r)' % (type(self).__name__, self.path)


def automake_input_missing(m):
    return MissingAutomakeInput(m.group(1))


class MissingAutoconfMacro(Problem):

    kind = 'missing-autoconf-macro'

    def __init__(self, macro):
        self.macro = macro

    def __eq__(self, other):
        return (
            isinstance(other, type(self)) and
            self.macro == other.macro)

    def __str__(self):
        return "autoconf macro %s missing" % self.macro

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.macro)


def autoconf_undefined_macro(m):
    return MissingAutoconfMacro(m.group(2))


class MissingGnomeCommonDependency(Problem):

    kind = 'missing-gnome-common-dependency'

    def __init__(self, package, minimum_version=None):
        self.package = package
        self.minimum_version = minimum_version

    def __eq__(self, other):
        return (isinstance(other, type(self)) and
                self.package == other.package and
                self.minimum_version == other.minimum_version)

    def __repr__(self):
        return "%s(%r, %r)" % (
            type(self).__name__, self.package, self.minimum_version)

    def __str__(self):
        return "Missing gnome-common dependency: %s: (>= %s)" % (
            self.package, self.minimum_version)


def missing_glib_gettext(m):
    return MissingGnomeCommonDependency('glib-gettext', m.group(1))


class MissingConfigStatusInput(Problem):

    kind = 'missing-config.status-input'

    def __init__(self, path):
        self.path = path

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.path == other.path

    def __str__(self):
        return "missing config.status input %s" % self.path

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.path)


def config_status_input_missing(m):
    return MissingConfigStatusInput(m.group(1))


class MissingJVM(Problem):

    kind = 'missing-jvm'

    def __init__(self):
        pass

    def __eq__(self, other):
        return isinstance(self, type(other))

    def __str__(self):
        return "Missing JVM"

    def __repr__(self):
        return "%s()" % (type(self).__name__)


def jvm_missing(m):
    return MissingJVM()


class UpstartFilePresent(Problem):

    kind = 'upstart-file-present'

    def __init__(self, filename):
        self.filename = filename

    def __eq__(self, other):
        return isinstance(self, type(other))

    def __str__(self):
        return "Upstart file present: %s" % self.filename

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.filename)


def dh_installinit_upstart_file(m):
    return UpstartFilePresent(m.group(1))


class NeedPgBuildExtUpdateControl(Problem):

    kind = 'need-pg-buildext-updatecontrol'

    def __init__(self, generated_path, template_path):
        self.generated_path = generated_path
        self.template_path = template_path

    def __eq__(self, other):
        return isinstance(self, type(self)) and \
            self.generated_path == other.generated_path and \
            self.template_path == other.template_path

    def __str__(self):
        return "Need to run 'pg_buildext updatecontrol' to update %s" % (
            self.generated_path)

    def __repr__(self):
        return "%s(%r, %r)" % (
            type(self).__name__, self.generated_path, self.template_path)


def need_pg_buildext_updatecontrol(m):
    return NeedPgBuildExtUpdateControl(m.group(1), m.group(2))


class MissingValaPackage(Problem):

    kind = 'missing-vala-package'

    def __init__(self, package):
        self.package = package

    def __str__(self):
        return "Missing Vala package: %s" % self.package

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.package)

    def __eq__(self, other):
        return (isinstance(other, type(self)) and
                self.package == other.package)


def vala_package_missing(m):
    return MissingValaPackage(m.group(1))


MAVEN_ERROR_PREFIX = '(?:\\[ERROR\\]|\\[\x1b\\[1;31mERROR\x1b\\[m\\]) '


class DirectoryNonExistant(Problem):

    kind = 'local-directory-not-existing'

    def __init__(self, path):
        self.path = path

    def __str__(self):
        return "Directory does not exist: %s" % self.path

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.path)

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.path == other.path


def directory_not_found(m):
    return DirectoryNonExistant(m.group(1))


class ImageMagickDelegateMissing(Problem):

    kind = 'imagemagick-delegate-missing'

    def __init__(self, delegate):
        self.delegate = delegate

    def __eq__(self, other):
        return (isinstance(other, type(self)) and
                other.delegate == self.delegate)

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.delegate)

    def __str__(self):
        return "Imagemagick missing delegate: %s" % self.delegate


def imagemagick_delegate_missing(m):
    return ImageMagickDelegateMissing(m.group(1))


class DebianVersionRejected(Problem):

    kind = 'debian-version-rejected'

    def __init__(self, version):
        self.version = version

    def __eq__(self, other):
        return isinstance(other, type(self)) and other.version == self.version

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.version)

    def __str__(self):
        return "Debian Version Rejected; %s" % self.version


def debian_version_rejected(m):
    return DebianVersionRejected(m.group(1))


def dh_missing_addon(m):
    return DhAddonLoadFailure(
        'pybuild', 'Debian/Debhelper/Buildsystem/pybuild.pm')


class MissingHaskellDependencies(Problem):

    kind = 'missing-haskell-dependencies'

    def __init__(self, deps):
        self.deps = deps

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.deps == other.deps

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.deps)

    def __str__(self):
        return "Missing Haskell dependencies: %r" % self.deps


class Matcher(object):

    def match(self, line: List[str], i: int) -> Tuple[
            List[int], Optional[Problem]]:
        raise NotImplementedError(self.match)


class SingleLineMatcher(Matcher):

    def __init__(self, regexp, cb=None):
        self.regexp = re.compile(regexp)
        self.cb = cb

    def match(self, lines, i):
        m = self.regexp.match(lines[i].rstrip('\n'))
        if not m:
            return [], None
        if self.cb:
            err = self.cb(m)
        else:
            err = None
        return [i], err


class HaskellMissingDependencyMatcher(Matcher):

    regexp = re.compile(
        'hlibrary\.setup: Encountered missing or private dependencies:\n')

    def match(self, lines, i):
        m = self.regexp.fullmatch(lines[i])
        if not m:
            return [], None
        deps = []
        linenos = [i]
        for line in lines[i+1:]:
            if not line.strip('\n'):
                break
            deps.append(tuple(line.rstrip('\n').split(' ', 1)))
            linenos.append(linenos[-1]+1)
        return linenos, MissingHaskellDependencies(deps)


def cmake_command_missing(m):
    return MissingCommand(m.group(1).lower())


def cmake_file_missing(m):
    return MissingFile(m.group(2))


def cmake_config_file_missing(m):
    return MissingPkgConfig(m.group(1), m.group(3))


def cmake_package_config_file_missing(m):
    return CMakeFilesMissing(
        [e.strip() for e in m.group(2).splitlines()])


def cmake_compiler_failure(m):
    compiler_output = textwrap.dedent(m.group(3))
    offset, description, error = find_build_failure_description(
        compiler_output.splitlines(True))
    return error


def cmake_compiler_missing(m):
    if m.group(1) == 'Fortran':
        return MissingFortranCompiler()
    return None


class CMakeErrorMatcher(Matcher):

    regexp = re.compile(r'CMake Error at (.*):([0-9]+) \((.*)\):\n')

    cmake_errors = [
        (r'--  Package \'(.*)\', required by \'(.*)\', not found',
         cmake_pkg_config_missing),
        (r'Could NOT find (.*) \(missing: .*\)', cmake_command_missing),
        (r'The (.+) compiler\n\n  "(.*)"\n\nis not able to compile a '
         r'simple test program\.\n\nIt fails with the following output:\n\n'
         r'(.*)\n\n'
         r'CMake will not be able to correctly generate this project.\n$',
         cmake_compiler_failure),
        (r'The imported target \"(.*)\" references the file\n\n\s*"(.*)"\n\n'
         r'but this file does not exist\.(.*)', cmake_file_missing),
        (r'Could not find a configuration file for package "(.*)".*'
         r'.*requested version "(.*)"\.', cmake_config_file_missing),
        (r'.*Could not find a package configuration file provided by "(.*)"\s'
         r'with\sany\sof\sthe\sfollowing\snames:\n\n(  .*\n)+\n.*$',
         cmake_package_config_file_missing),
        (r'No CMAKE_(.*)_COMPILER could be found.\n'
         r'\n'
         r'Tell CMake where to find the compiler by setting either'
         r'\sthe\senvironment\svariable\s"(.*)"\sor\sthe\sCMake\scache'
         r'\sentry\sCMAKE_(.*)_COMPILER\sto\sthe\sfull\spath\sto'
         r'\sthe\scompiler,\sor\sto\sthe\scompiler\sname\sif\sit\sis\sin\s'
         r'the\sPATH.\n', cmake_command_missing),
        (r'file INSTALL cannot find\s"(.*)".\n',
         lambda m: MissingFile(m.group(1))),
        (r'file INSTALL cannot copy file\n"(.*)"\sto\s"(.*)":\s'
         r'No space left on device.\n', lambda m: NoSpaceOnDevice()),
        (r'file INSTALL cannot copy file\n"(.*)"\nto\n"(.*)"\.\n', None),
    ]

    @classmethod
    def _extract_error_lines(cls, lines, i):
        linenos = [i]
        error_lines = []
        for j, line in enumerate(lines[i+1:]):
            if line != '\n' and not line.startswith(' '):
                break
            error_lines.append(line)
            linenos.append(i + 1 + j)
        while error_lines and error_lines[-1] == '\n':
            error_lines.pop(-1)
            linenos.pop(-1)
        return linenos, textwrap.dedent(''.join(error_lines)).splitlines(True)

    def match(self, lines, i):
        m = self.regexp.fullmatch(lines[i])
        if not m:
            return [], None

        path = m.group(1)   # noqa: F841
        start_lineno = int(m.group(2))   # noqa: F841
        linenos, error_lines = self._extract_error_lines(lines, i)

        error = None
        for r, fn in self.cmake_errors:
            if fn is None:
                error = None
                break
            m = re.match(r, ''.join(error_lines), flags=re.DOTALL)
            if m:
                error = fn(m)
                break

        return linenos, error


class MissingFortranCompiler(Problem):

    kind = 'missing-fortran-compiler'

    def __init__(self):
        pass

    def __eq__(self, other):
        return isinstance(other, type(self))

    def __str__(self):
        return "No Fortran compiler found"

    def __repr__(self):
        return "%s()" % type(self).__name__


class MissingCSharpCompiler(Problem):

    kind = 'missing-c#-compiler'

    def __init__(self):
        pass

    def __eq__(self, other):
        return isinstance(other, type(self))

    def __str__(self):
        return "No C# compiler found"

    def __repr__(self):
        return "%s()" % type(self).__name__


def c_sharp_compiler_missing(m):
    return MissingCSharpCompiler()


class MissingCargoCrate(Problem):

    kind = 'missing-cargo-crate'

    def __init__(self, crate, requirement):
        self.crate = crate
        self.requirement = requirement

    def __eq__(self, other):
        return (
            isinstance(other, type(self)) and
            self.crate == other.crate and
            self.requirement == other.requirement)

    def __str__(self):
        if self.requirement:
            return "Missing crate: %s (%s)" % (
                self.crate, self.requirement)
        else:
            return "Missing crate: %s" % self.crate

    def __repr__(self):
        return "%s(%r, %r)" % (
            type(self).__name__, self.crate, self.requirement)


def cargo_missing_requirement(m):
    try:
        crate, requirement = m.group(1).split(' ', 1)
    except ValueError:
        crate = m.group(1)
        requirement = None
    return MissingCargoCrate(crate, requirement)


class MissingDHCompatLevel(Problem):

    kind = 'missing-dh-compat-level'

    def __init__(self, command):
        self.command = command

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.command)

    def __str__(self):
        return "Missing DH Compat Level (command: %s)" % self.command

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.command == other.command


class DuplicateDHCompatLevel(Problem):

    kind = 'duplicate-dh-compat-level'

    def __init__(self, command):
        self.command = command

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.command)

    def __str__(self):
        return "DH Compat Level specified twice (command: %s)" % self.command

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.command == other.command


build_failure_regexps = [
    (r'make\[[0-9]+\]: \*\*\* No rule to make target '
        r'\'(.*)\', needed by \'.*\'\.  Stop\.', file_not_found),
    (r'[^:]+:\d+: (.*): No such file or directory', file_not_found),
    (r'(distutils.errors.DistutilsError|error): '
     r'Could not find suitable distribution '
     r'for Requirement.parse\(\'([^\']+)\'\)', python_reqs_not_found),
    (r'pkg_resources.DistributionNotFound: The \'([^\']+)\' '
     r'distribution was not found and is required by (.*)',
     pkg_resources_distribution_not_found),
    (r'pluggy.manager.PluginValidationError: '
     r'Plugin \'.*\' could not be loaded: '
     r'\(.* \(/usr/lib/python2.[0-9]/dist-packages\), '
     r'Requirement.parse\(\'(.*)\'\)\)\!', python2_reqs_not_found),
    ('E   ImportError: cannot import name \'(.*)\' from \'(.*)\'',
     python_module_not_found),
    ('E   ImportError: cannot import name ([^\']+)', python_module_not_found),
    (r'django.core.exceptions.ImproperlyConfigured: Error loading .* module: '
     r'No module named \'(.*)\'', python_module_not_found),
    ('E   ImportError: No module named (.*)', python_module_not_found),
    ('ModuleNotFoundError: No module named \'(.*)\'',
     python3_module_not_found),
    (r'Could not import extension .* \(exception: No module named (.*)\)',
     sphinx_module_not_found),
    ('E   ModuleNotFoundError: No module named \'(.*)\'',
     python3_module_not_found),
    (r'/usr/bin/python3: No module named (.*)', python3_module_not_found),
    ('.*: cannot find package "(.*)" in any of:', missing_go_package),
    (r'ImportError: Error importing plugin ".*": No module named (.*)',
     python_module_not_found),
    ('ImportError: No module named (.*)', python_module_not_found),
    (r'[^:]+:\d+:\d+: fatal error: (.+\.h|.+\.hpp): No such file or directory',
     c_header_missing),
    (r'[^:]+\.[ch]:\d+:\d+: fatal error: (.+): No such file or directory',
     c_header_missing),
    ('✖ \x1b\\[31mERROR:\x1b\\[39m Cannot find module \'(.*)\'',
     node_module_missing),
    ('\\[31mError: No test files found: "(.*)"\\[39m', None),
    ('\x1b\[31mError: No test files found: "(.*)"\x1b\[39m', None),
    (r'\s*Error: Cannot find module \'(.*)\'', node_module_missing),
    (r'>> Error: Cannot find module \'(.*)\'', node_module_missing),
    (r'>> Got an unexpected exception from the coffee-script compiler. '
     r'The original exception was: Error: Cannot find module \'(.*)\'',
     node_module_missing),
    (r'\s*Module not found: Error: Can\'t resolve \'(.*)\' in \'(.*)\'',
     node_module_missing),
    (r'>> Local Npm module \"(.*)" not found. Is it installed?',
     node_module_missing),
    (r'.*: line \d+: ([^ ]+): command not found', command_missing),
    (r'.*: line \d+: ([^ ]+): Permission denied', None),
    (r'\/bin\/sh: \d+: ([^ ]+): not found', command_missing),
    (r'sh: \d+: ([^ ]+): not found', command_missing),
    (r'.*: 1: cd: can\'t cd to (.*)', directory_not_found),
    (r'\/bin\/bash: (.*): command not found', command_missing),
    (r'bash: (.*): command not found', command_missing),
    (r'env: ‘(.*)’: No such file or directory', interpreter_missing),
    (r'\/bin\/bash: .*: (.*): bad interpreter: No such file or directory',
     interpreter_missing),
    # SH error
    (r'.*: [0-9]+: (.*): not found', command_missing),
    (r'/usr/bin/env: ‘(.*)’: No such file or directory',
     command_missing),
    (r'/usr/bin/env: \'(.*)\': No such file or directory',
     command_missing),
    (r'make\[[0-9]+\]: (.*): Command not found', command_missing),
    (r'make: (.*): Command not found', command_missing),
    (r'make: (.*): No such file or directory', command_missing),
    (r'make\[[0-9]+\]: ([^/ :]+): No such file or directory', command_missing),
    (r'.*: failed to exec \'(.*)\': No such file or directory',
     command_missing),
    (r'No package \'([^\']+)\' found', pkg_config_missing),
    (r'configure: error: No package \'([^\']+)\' found', pkg_config_missing),
    (r'configure: error: (doxygen|asciidoc) is not available '
     r'and maintainer mode is enabled',
     lambda m: MissingCommand(m.group(1))),
    (r'configure: error: Documentation enabled but rst2html not found.',
     lambda m: MissingCommand('rst2html')),
    (r'Error: pkg-config not found\!', lambda m: MissingCommand('pkg-config')),
    (r' ERROR: BLAS not found\!', lambda m: MissingLibrary('blas')),
    (r'\./configure: [0-9]+: \.: Illegal option .*', None),
    (r'Requested \'(.*)\' but version of ([^ ]+) is ([^ ]+)',
     pkg_config_missing),
    (r'configure: error: Package requirements \((.*)\) were not met:',
     pkg_config_missing),
    (r'configure: error: [a-z0-9_-]+-pkg-config (.*) couldn\'t be found',
     pkg_config_missing),
    (r'configure: error: C preprocessor "/lib/cpp" fails sanity check',
     None),
    (r'configure: error: .*\. Please install (bison|flex)',
     lambda m: MissingCommand(m.group(1))),
    (r'configure: error: No C\# compiler found. You need to install either '
     'mono \(>=(.*)\) or \.Net', c_sharp_compiler_missing),
    (r'configure: error: gmcs Not found', c_sharp_compiler_missing),
    (r'configure: error: You need to install gmcs', c_sharp_compiler_missing),
    (r'configure: error: (.*) requires libkqueue \(or system kqueue\). .*',
     lambda m: MissingPkgConfig('libkqueue')),
    ('.*meson.build:([0-9]+):([0-9]+): ERROR: Dependency "(.*)" not found, '
     'tried pkgconfig', meson_pkg_config_missing),
    ('.*meson.build:([0-9]+):([0-9]+): ERROR: Invalid version of dependency, '
     'need \'([^\']+)\' \\[\'>= ([^\']+)\'\\] found \'([^\']+)\'\\.',
     meson_pkg_config_too_low),
    (r'dh: Unknown sequence --(.*) '
     r'\(options should not come before the sequence\)', dh_with_order),
    (r'dh: Compatibility levels before [0-9]+ are no longer supported '
     r'\(level [0-9]+ requested\)', None),
    (r'dh: Unknown sequence (.*) \(choose from: .*\)', None),
    (r'.*: .*: No space left on device', install_no_space),
    (r'^No space left on device.', install_no_space),
    (r'.*Can\'t locate (.*).pm in @INC \(you may need to install the '
     r'(.*) module\) \(@INC contains: (.*)\) at .* line .*.',
     perl_missing_module),
    (r'>\(error\): Could not expand \[(.*)\'',
     perl_expand_failed),
    (r'\[DZ\] could not load class (.*) for license (.*)',
     lambda m: MissingPerlModule(None, m.group(1), None)),
    (r'Required plugin bundle ([^ ]+) isn\'t installed.', perl_missing_plugin),
    (r'Required plugin ([^ ]+) isn\'t installed.', perl_missing_plugin),
    (r'.*Can\'t locate (.*) in @INC \(@INC contains: (.*)\) at .* line .*.',
     perl_missing_file),
    (r'Can\'t find author dependency (.*) at (.*) line ([0-9]+).',
     perl_missing_author_dep),
    (r'> Could not find (.*). Please check that (.*) contains a valid JDK '
     r'installation.', jdk_file_missing),
    (r'(?:/usr/bin/)?install: cannot create regular file \'(.*)\': '
     r'No such file or directory', None),
    (r'python[0-9.]*: can\'t open file \'(.*)\': \[Errno 2\] '
     r'No such file or directory', file_not_found),
    (r'OSError: No such file (.*)', file_not_found),
    (r'Could not open \'(.*)\': No such file or directory at '
     r'\/usr\/share\/perl\/[0-9.]+\/ExtUtils\/MM_Unix.pm line [0-9]+.',
     perl_file_not_found),
    (r'Can\'t open perl script "(.*)": No such file or directory',
     perl_file_not_found),
    # Maven
    (MAVEN_ERROR_PREFIX + r'Failed to execute goal on project .*: '
     r'Could not resolve dependencies for project .*: '
     r'The following artifacts could not be resolved: (.*): '
     r'Cannot access central \(https://repo\.maven\.apache\.org/maven2\) '
     r'in offline mode and the artifact .* has not been downloaded from '
     r'it before..*', maven_missing_artifact),
    (MAVEN_ERROR_PREFIX + r'Unresolveable build extension: '
     r'Plugin (.*) or one of its dependencies could not be resolved: '
     r'Cannot access central \(https://repo.maven.apache.org/maven2\) '
     r'in offline mode and the artifact .* has not been downloaded '
     'from it before. @', maven_missing_plugin),
    (MAVEN_ERROR_PREFIX + r'Non-resolvable import POM: Cannot access central '
     r'\(https://repo.maven.apache.org/maven2\) in offline mode and the '
     r'artifact (.*) has not been downloaded from it before. '
     r'@ line [0-9]+, column [0-9]+', maven_missing_artifact),
    (r'\[FATAL\] Non-resolvable parent POM for .*: Cannot access central '
     r'\(https://repo.maven.apache.org/maven2\) in offline mode and the '
     'artifact (.*) has not been downloaded from it before. .*',
     maven_missing_artifact),
    (MAVEN_ERROR_PREFIX + r'Plugin (.*) or one of its dependencies could '
     r'not be resolved: Cannot access central '
     r'\(https://repo.maven.apache.org/maven2\) in offline mode and the '
     r'artifact .* has not been downloaded from it before. -> \[Help 1\]',
     maven_missing_plugin),
    (MAVEN_ERROR_PREFIX + r'Failed to execute goal on project .*: '
     r'Could not resolve dependencies for project .*: Cannot access '
     r'.* \([^\)]+\) in offline mode and the artifact '
     r'(.*) has not been downloaded from it before. -> \[Help 1\]',
     maven_missing_artifact),
    (MAVEN_ERROR_PREFIX + r'Failed to execute goal on project .*: '
     r'Could not resolve dependencies for project .*: Cannot access central '
     r'\(https://repo.maven.apache.org/maven2\) in offline mode and the '
     r'artifact (.*) has not been downloaded from it before..*',
     maven_missing_artifact),
    (MAVEN_ERROR_PREFIX +
     'Failed to execute goal (.*) on project (.*): (.*)', None),
    (MAVEN_ERROR_PREFIX +
     r'Error resolving version for plugin \'(.*)\' from the repositories '
     r'\[.*\]: Plugin not found in any plugin repository -> \[Help 1\]',
     maven_missing_plugin),
    (r'dh_missing: (warning: )?(.*) exists in debian/.* but is not '
     r'installed to anywhere', dh_missing_uninstalled),
    (r'dh_link: link destination (.*) is a directory',
     dh_link_destination_is_dir),
    (r'I/O error : Attempt to load network entity (.*)',
     xsltproc_network_entity),
    (r'ccache: error: (.*)', ccache_error),
    (r'dh: The --until option is not supported any longer \(#932537\). '
     r'Use override targets instead.', dh_until_unsupported),
    (r'dh: unable to load addon (.*): (.*) did not return a true '
     r'value at \(eval 11\) line ([0-9]+).', dh_addon_load_failure),
    ('ERROR: dependencies (.*) are not available for package ‘(.*)’',
     r_missing_package),
    ('ERROR: dependency ‘(.*)’ is not available for package ‘(.*)’',
     r_missing_package),
    (r'Error in library\(.*\) : there is no package called \'(.*)\'',
     r_missing_package),
    (r'there is no package called \'(.*)\'', r_missing_package),
    (r'  namespace ‘(.*)’ ([^ ]+) is being loaded, but >= ([^ ]+) is required',
     r_too_old),
    (r'  namespace ‘(.*)’ ([^ ]+) is already loaded, but >= ([^ ]+) '
     r'is required', r_too_old),
    (r'mv: cannot stat \'(.*)\': No such file or directory',
     file_not_found),
    (r'mv: cannot move \'.*\' to \'(.*)\': No such file or directory',
     None),
    (r'(/usr/bin/install|mv): '
     r'will not overwrite just-created \'(.*)\' with \'(.*)\'', None),
    (r'IOError: \[Errno 2\] No such file or directory: \'(.*)\'',
     file_not_found),
    (r'E   IOError: \[Errno 2\] No such file or directory: \'(.*)\'',
     file_not_found),
    ('FAIL\t(.+\\/.+\\/.+)\t([0-9.]+)s', go_test_failed),
    (r'dh_(.*): Cannot find \(any matches for\) "(.*)" \(tried in (.*)\)',
     dh_pattern_no_matches),
    (r'Can\'t exec "(.*)": No such file or directory at '
     r'/usr/share/perl5/Debian/Debhelper/Dh_Lib.pm line [0-9]+.',
     command_missing),
    (r'.*: error: (.*) command not found', command_missing),
    (r'dh_install: Please use dh_missing '
     '--list-missing/--fail-missing instead', None),
    (r'dh([^:]*): Please use the third-party "pybuild" build system '
     'instead of python-distutils', None),
    # A Python error, but not likely to be actionable. The previous
    # line will have the actual line that failed.
    (r'ImportError: cannot import name (.*)', None),
    (r'/usr/bin/ld: cannot find -l(.*)', ld_missing_lib),
    (r'Could not find gem \'([^ ]+) \(([^)]+)\)\', '
     r'which is required by gem.*', ruby_missing_gem),
    (r'Could not find gem \'([^ \']+)\', '
     r'which is required by gem.*', lambda m: MissingRubyGem(m.group(1))),
    (r'[^:]+:[0-9]+:in \`to_specs\': Could not find \'(.*)\' \(([^)]+)\) '
     r'among [0-9]+ total gem\(s\) \(Gem::MissingSpecError\)',
     ruby_missing_gem),
    (r'[^:]+:[0-9]+:in \`to_specs\': Could not find \'(.*)\' \(([^)]+)\) '
     r'- .* \(Gem::MissingSpecVersionError\)', ruby_missing_gem),
    (r'[^:]+:[0-9]+:in \`block in verify_gemfile_dependencies_are_found\!\': '
     r'Could not find gem \'(.*)\' in any of the gem sources listed in '
     r'your Gemfile\. \(Bundler::GemNotFound\)',
     lambda m: MissingRubyGem(m.group(1))),
    (r'[^:]+:[0-9]+:in \`find_spec_for_exe\': can\'t find gem '
     r'(.*) \(([^)]+)\) with executable (.*) \(Gem::GemNotFoundException\)',
     ruby_missing_gem),
    (r'PHP Fatal error:  Uncaught Error: Class \'(.*)\' not found in '
     r'(.*):([0-9]+)', php_missing_class),
    (r'Caused by: java.lang.ClassNotFoundException: (.*)',
     java_missing_class),
    (r'\[(.*)\] \t\t:: (.*)\#(.*);\$\{(.*)\}: not found',
     lambda m: MissingMavenArtifacts([
         '%s:%s:jar:debian' % (m.group(2), m.group(3))])),
    (r'Caused by: java.lang.IllegalArgumentException: '
     r'Cannot find JAR \'(.*)\' required by module \'(.*)\' '
     r'using classpath or distribution directory \'(.*)\'', None),
    (r'.*\.xml:[0-9]+: Unable to find a javac compiler;',
     lambda m: MissingJavaClass('com.sun.tools.javac.Main')),
    (r'python3.[0-9]+: can\'t open file \'(.*)\': '
     '[Errno 2] No such file or directory', file_not_found),
    (r'g\+\+: error: (.*): No such file or directory', file_not_found),
    (r'strip: \'(.*)\': No such file', file_not_found),
    (r'Sprockets::FileNotFound: couldn\'t find file \'(.*)\' '
     r'with type \'(.*)\'', sprockets_file_not_found),
    (r'xdt-autogen: You must have "(.*)" installed. You can get if from',
     xfce_dependency_missing),
    (r'You need to install the gnome-common module and make.*',
     gnome_common_missing),
    (r'You need to install gnome-common from the GNOME (git|CVS|SVN)',
     gnome_common_missing),
    (r'automake: error: cannot open < (.*): No such file or directory',
     automake_input_missing),
    (r'configure.(in|ac):[0-9]+: error: possibly undefined macro: (.*)',
     autoconf_undefined_macro),
    (r'config.status: error: cannot find input file: `(.*)\'',
     config_status_input_missing),
    (r'\*\*\*Error\*\*\*: You must have glib-gettext >= (.*) installed.*',
     missing_glib_gettext),
    (r'ERROR: JAVA_HOME is set to an invalid directory: '
     r'/usr/lib/jvm/default-java/', jvm_missing),
    (r'dh_installdocs: --link-doc not allowed between (.*) and (.*) '
     r'\(one is arch:all and the other not\)', None),
    (r'dh: unable to load addon systemd: dh: The systemd-sequence is '
     'no longer provided in compat >= 11, please rely on dh_installsystemd '
     'instead', None),
    (r'dh: The --before option is not supported any longer \(#932537\). '
     r'Use override targets instead.', None),
    ('(.*):([0-9]+): undefined reference to `(.*)\'', None),
    ('(.*):([0-9]+): error: undefined reference to \'(.*)\'', None),
    (r'\/usr\/bin\/ld: (.*): multiple definition of `*.\'; '
     r'(.*): first defined here', None),
    (r'\/usr\/bin\/ld: (.*): undefined reference to `(.*)\'', None),
    (r'\/usr\/bin\/ld: (.*): undefined reference to symbol \'(.*)\'', None),
    (r'\/usr\/bin\/ld: (.*): relocation (.*) against symbol `(.*)\' '
     r'can not be used when making a shared object; recompile with -fPIC',
     None),
    ('(.*):([0-9]+): multiple definition of `(.*)\'; (.*):([0-9]+): '
     'first defined here', None),
    ('(dh.*): debhelper compat level specified both in debian/compat '
     'and via build-dependency on debhelper-compat', 
     lambda m: DuplicateDHCompatLevel(m.group(1))),
    ('(dh.*): (error: )?Please specify the compatibility level in debian/compat',
     lambda m: MissingDHCompatLevel(m.group(1))),
    ('dh_makeshlibs: The udeb (.*) does not contain any shared libraries '
     'but --add-udeb=(.*) was passed!?', None),
    ('dpkg-gensymbols: error: some symbols or patterns disappeared in the '
     'symbols file: see diff output below', None),
    (r'Failed to copy \'(.*)\': No such file or directory at '
     r'/usr/share/dh-exec/dh-exec-install-rename line [0-9]+.*',
     file_not_found),
    (r'Invalid gemspec in \[.*\]: No such file or directory - (.*)',
     command_missing),
    (r'.*meson.build:[0-9]+:[0-9]+: ERROR: Program\(s\) \[\'(.*)\'\] not '
     r'found or not executable', command_missing),
    (r'.*meson.build:[0-9]+:[0-9]: ERROR: Git program not found\.',
     lambda m: MissingCommand('git')),
    (r'dpkg-gensymbols: error: some symbols or patterns disappeared in '
     r'the symbols file: see diff output below',
     None),
    (r'Failed: [pytest] section in setup.cfg files is no longer '
     r'supported, change to [tool:pytest] instead.', None),
    (r'cp: cannot stat \'(.*)\': No such file or directory', None),
    (r'cp: \'(.*)\' and \'(.*)\' are the same file', None),
    (r'PHP Fatal error: (.*)', None),
    (r'sed: no input files', None),
    (r'sed: can\'t read (.*): No such file or directory',
     file_not_found),
    (r'ERROR in Entry module not found: Error: Can\'t resolve '
     r'\'(.*)\' in \'(.*)\'', webpack_file_missing),
    (r'.*:([0-9]+): element include: XInclude error : '
     r'could not load (.*), and no fallback was found', None),
    (r'E: The Debian version .* cannot be used as an ELPA version.',
     None),
    # ImageMagick
    (r'convert convert: Image pixel limit exceeded '
     r'\(see -limit Pixels\) \(-1\).',
     None),
    (r'convert convert: Improper image header \(.*\).',
     None),
    (r'convert convert: invalid primitive argument \([0-9]+\).', None),
    (r'convert convert: Unexpected end-of-file \(\)\.', None),
    (r'ERROR: Sphinx requires at least Python (.*) to run.',
     None),
    (r'convert convert: No encode delegate for this image format \((.*)\) '
     r'\[No such file or directory\].', imagemagick_delegate_missing),
    (r'Can\'t find (.*) directory in (.*)', None),
    (r'/bin/sh: [0-9]: cannot create (.*): Directory nonexistent',
     lambda m: DirectoryNonExistant(os.path.dirname(m.group(1)))),
    (r'dh: Unknown sequence (.*) \(choose from: .*\)', None),
    (r'.*\.vala:[0-9]+\.[0-9]+-[0-9]+.[0-9]+: error: (.*)',
     None),
    (r'error: Package `(.*)\' not found in specified Vala API directories '
     r'or GObject-Introspection GIR directories', vala_package_missing),
    (r'.*.scala:[0-9]+: error: (.*)', None),
    # JavaScript
    (r'error TS6053: File \'(.*)\' not found.',
     file_not_found),
    (r'(.*\.ts)\([0-9]+,[0-9]+\): error TS[0-9]+: (.*)', None),
    (r'(.*.nim)\([0-9]+, [0-9]+\) Error: .*', None),
    (r'dh_installinit: upstart jobs are no longer supported\!  '
     r'Please remove (.*) and check if you need to add a conffile removal',
     dh_installinit_upstart_file),
    (r'dh_installinit: --no-restart-on-upgrade has been renamed to '
     '--no-stop-on-upgrade', None),
    (r'find: paths must precede expression: .*', None),
    (r'find: ‘(.*)’: No such file or directory', file_not_found),
    (r'ninja: fatal: posix_spawn: Argument list too long', None),
    ('ninja: fatal: chdir to \'(.*)\' - No such file or directory',
     directory_not_found),
    # Java
    (r'error: Source option [0-9] is no longer supported. Use [0-9] or later.',
     None),
    (r'(dh.*|jh_build): -s/--same-arch has been removed; '
     r'please use -a/--arch instead', None),
    (r'dh_systemd_start: dh_systemd_start is no longer used in '
     r'compat >= 11, please use dh_installsystemd instead', None),
    (r'Trying patch (.*) at level 1 \.\.\. 0 \.\.\. 2 \.\.\. failure.', None),
    # QMake
    (r'Project ERROR: Unknown module\(s\) in QT: (.*)', None),
    (r'Project ERROR: (.*) development package not found',
     pkg_config_missing),
    (r'Package \'(.*)\', required by \'(.*)\', not found\n',
     pkg_config_missing),
    (r'pkg-config cannot find (.*)', pkg_config_missing),
    (r'configure: error: .* not found: Package dependency requirement '
     r'\'([^\']+)\' could not be satisfied.', pkg_config_missing),
    (r'configure: error: xsltproc is required to build documentation',
     lambda m: MissingCommand('xsltproc')),
    (r'.*:[0-9]+: (.*) does not exist.', file_not_found),
    # uglifyjs
    (r'ERROR: can\'t read file: (.*)', file_not_found),
    (r'jh_build: Cannot find \(any matches for\) "(.*)" \(tried in .*\)',
     None),
    (r'--   Package \'(.*)\', required by \'(.*)\', not found',
     lambda m: MissingPkgConfig(m.group(1))),
    (r'.*.rb:[0-9]+:in `require_relative\': cannot load such file '
     r'-- (.*) \(LoadError\)', None),
    (r'.*.rb:[0-9]+:in `require\': cannot load such file '
     r'-- (.*) \(LoadError\)', ruby_missing_name),
    (r'LoadError: cannot load such file -- (.*)', ruby_missing_name),
    (r'  cannot load such file -- (.*)',
     ruby_missing_name),
    # TODO(jelmer): This is a fairly generic string; perhaps combine with other
    # checks for ruby?
    (r'File does not exist: ([a-z/]+)$', ruby_missing_name),
    (r'.*:[0-9]+:in `do_check_dependencies\': E: '
     r'dependency resolution check requested but no working '
     r'gemspec available \(RuntimeError\)', None),
    (r'rm: cannot remove \'(.*)\': Is a directory', None),
    (r'rm: cannot remove \'(.*)\': No such file or directory', None),
    # Invalid option from Python
    (r'error: option .* not recognized', None),
    # Invalid option from go
    (r'flag provided but not defined: .*', None),
    (r'CMake Error: The source directory "(.*)" does not exist.',
     directory_not_found),
    (r'.*: [0-9]+: cd: can\'t cd to (.*)', directory_not_found),
    (r'/bin/sh: 0: Can\'t open (.*)', file_not_found),
    (r'/bin/sh: [0-9]+: cannot open (.*): No such file',
     file_not_found),
    (r'.*: line [0-9]+: (.*): No such file or directory', file_not_found),
    (r'error: No member named \$memberName', None),
    (r'(?:/usr/bin/)?install: cannot create regular file \'(.*)\': '
     r'Permission denied', None),
    (r'(?:/usr/bin/)?install: cannot create directory .(.*).: File exists',
     None),
    (r'/usr/bin/install: missing destination file operand after .*', None),
    # Ruby
    (r'rspec .*\.rb:[0-9]+ # (.*)', None),
    # help2man
    (r'Addendum (.*) does NOT apply to (.*) \(translation discarded\).',
     None),
    (r'dh_installchangelogs: copy\((.*), (.*)\): No such file or directory',
     file_not_found),
    (r'dh_installman: mv (.*) (.*): No such file or directory',
     file_not_found),
    (r'dh_installman: Could not determine section for (.*)', None),
    (r'failed to initialize build cache at (.*): mkdir (.*): '
     r'permission denied', None),
    (r'Can\'t exec "(.*)": No such file or directory at (.*) line ([0-9]+).',
     command_missing),
    # PHPUnit
    (r'Cannot open file "(.*)".', file_not_found),
    (r'.*Could not find a JavaScript runtime\. See '
     r'https://github.com/rails/execjs for a list of available runtimes\..*',
     javascript_runtime_missing),
    (r'^(?:E  +)?FileNotFoundError: \[Errno 2\] '
     r'No such file or directory: \'(.*)\'', file_not_found),
    # ruby
    (r'Errno::ENOENT: No such file or directory - (.*)',
     file_not_found),
    (r'(.*.rb):[0-9]+:in `.*\': .* \(.*\) ', None),
    # JavaScript
    (r'.*: ENOENT: no such file or directory, open \'(.*)\'',
     file_not_found),
    (r'\[Error: ENOENT: no such file or directory, stat \'(.*)\'\] \{',
     file_not_found),
    # libtoolize
    (r'libtoolize:   error: \'(.*)\' does not exist.',
     file_not_found),
    # Seen in python-cogent
    ('RuntimeError: Numpy required but not found.',
     lambda m: MissingPythonModule('numpy')),
    # Seen in cpl-plugin-giraf
    (r'ImportError: Numpy version (.*) or later must be '
     r'installed to use .*', lambda m: MissingPythonModule(
         'numpy', minimum_version=m.group(1))),
    # autoconf
    (r'configure.ac:[0-9]+: error: required file \'(.*)\' not found',
     file_not_found),
    # automake
    (r'Makefile.am: error: required file \'(.*)\' not found', file_not_found),
    # sphinx
    (r'config directory doesn\'t contain a conf.py file \((.*)\)',
     None),
    # vcversioner
    (r'vcversioner: no VCS could be detected in \'/<<PKGBUILDDIR>>\' '
     r'and \'/<<PKGBUILDDIR>>/version.txt\' isn\'t present.', None),
    # rst2html (and other Python?)
    (r'  InputError: \[Errno 2\] No such file or directory: \'(.*)\'',
     file_not_found),
    # gpg
    (r'gpg: can\'t connect to the agent: File name too long', None),
    (r'(.*.lua):[0-9]+: assertion failed', None),
    (r'\*\*\* error: gettext infrastructure mismatch: .*', None),
    (r'\s+\^\-\-\-\-\^ SC[0-4][0-9][0-9][0-9]: .*', None),
    (r'Error: (.*) needs updating from (.*)\. '
     r'Run \'pg_buildext updatecontrol\'.', need_pg_buildext_updatecontrol),
    (r'Patch (.*) does not apply \(enforce with -f\)', None),
    (r'convert convert: Unable to read font \((.*)\) '
     r'\[No such file or directory\].', file_not_found),
    (r'java.io.FileNotFoundException: (.*) \(No such file or directory\)',
     file_not_found),
    # Pytest
    (r'INTERNALERROR> PluginValidationError: (.*)', None),
    (r'[0-9]+ out of [0-9]+ hunks FAILED -- saving rejects to file (.*\.rej)',
     None),
    (r'pkg_resources.UnknownExtra: (.*) has no such extra feature \'(.*)\'',
     None),
    (r'dh_auto_configure: invalid or non-existing path '
     r'to the source directory: .*', None),
    # Sphinx
    (r'sphinx_rtd_theme is no longer a hard dependency since version (.*). '
     r'Please install it manually.\(pip install (.*)\)',
     lambda m: MissingPythonModule('sphinx_rtd_theme')),
    (r'There is a syntax error in your configuration file: (.*)',
     None),
    (r'E: The Debian version (.*) cannot be used as an ELPA version.',
     debian_version_rejected),
    (r'"(.*)" is not exported by the ExtUtils::MakeMaker module', None),
    (r'E: Please add appropriate interpreter package to Build-Depends, '
     r'see pybuild\(1\) for details\..*',
     dh_missing_addon),
    (r'dpkg: error: .*: No space left on device', lambda m: NoSpaceOnDevice()),
    (r'You need the GNU readline library(ftp://ftp.gnu.org/gnu/readline/ ) '
     r'to build', lambda m: MissingLibrary('readline')),
    HaskellMissingDependencyMatcher(),
    CMakeErrorMatcher(),
    (r'error: failed to select a version for the requirement `(.*)`',
     cargo_missing_requirement),
    (r'^Environment variable \$SOURCE_DATE_EPOCH: No digits were found: $',
     None),
]


compiled_build_failure_regexps = []
for entry in build_failure_regexps:
    try:
        if isinstance(entry, tuple):
            (regexp, cb) = entry
            matcher = SingleLineMatcher(regexp, cb)
        else:
            matcher = entry
        compiled_build_failure_regexps.append(matcher)
    except re.error as e:
        raise Exception('Error in %s: %s' % (regexp, e))


# Regexps that hint at an error of some sort, but not the error itself.
secondary_build_failure_regexps = [
    # Java
    r'Exception in thread "(.*)" (.*): (.*);',
    r'error: Unrecognized option: \'.*\'',
    r'.*: No space left on device',
    r'Segmentation fault',
    r'make\[[0-9]+\]: \*\*\* \[.*:[0-9]+: .*\] Segmentation fault',
    (r'make\[[0-9]+\]: \*\*\* No rule to make target '
     r'\'(?!maintainer-clean)(?!clean)(.*)\'\.  Stop\.'),
    r'.*:[0-9]+: \*\*\* empty variable name.  Stop.',
    # QMake
    r'Project ERROR: .*',
    # pdflatex
    r'\!  ==> Fatal error occurred, no output PDF file produced\!',
    # latex
    r'\! Undefined control sequence\.',
    r'\! Emergency stop\.',
    r'\!pdfTeX error: pdflatex: fwrite\(\) failed',
    # inkscape
    r'Unknown option .*',
    # CTest
    r'Errors while running CTest',
    r'dh_auto_install: error: .*',
    r'dh.*: Aborting due to earlier error',
    r'dh.*: unknown option or error during option parsing; aborting',
    r'Could not import extension .* \(exception: .*\)',
    r'configure.ac:[0-9]+: error: (.*)',
    r'dwz: Too few files for multifile optimization',
    r'dh_dwz: dwz -q -- .* returned exit code [0-9]+',
    r'help2man: can\'t get `-?-help\' info from .*',
    r'[^:]+: line [0-9]+:\s+[0-9]+ Segmentation fault.*',
    r'.*(No space left on device).*',
    r'dpkg-gencontrol: error: (.*)',
    r'.*:[0-9]+:[0-9]+: (error|ERROR): (.*)',
    r'FAIL: (.*)',
    r'FAIL (.*) \(.*\)',
    r'FAIL\s+(.*) \[.*\] ?',
    r'TEST FAILURE',
    r'make\[[0-9]+\]: \*\*\* \[.*\] Error [0-9]+',
    r'make\[[0-9]+\]: \*\*\* \[.*\] Aborted',
    r'E: pybuild pybuild:[0-9]+: test: plugin [^ ]+ failed with:'
    r'exit code=[0-9]+: .*',
    r'chmod: cannot access \'.*\': No such file or directory',
    r'dh_autoreconf: autoreconf .* returned exit code [0-9]+',
    r'make: \*\*\* \[.*\] Error [0-9]+',
    r'.*:[0-9]+: \*\*\* missing separator\.  Stop\.',
    r'[^:]+: cannot stat \'.*\': No such file or directory',
    r'[0-9]+ tests: [0-9]+ ok, [0-9]+ failure\(s\), [0-9]+ test\(s\) skipped',
    r'\*\*Error:\*\* (.*)',
    r'^Error: (.*)',
    r'Failed [0-9]+ tests? out of [0-9]+, [0-9.]+% okay.',
    r'Failed [0-9]+\/[0-9]+ test programs. [0-9]+/[0-9]+ subtests failed.',
    r'Original error was: (.*)',
    r'[^:]+: error: (.*)',
    r'[^:]+:[0-9]+: error: (.*)',
    r'^FAILED \(.*\)',
    r'cat: (.*): No such file or directory',
    # Random Python errors
    '^(E  +)?(SyntaxError|TypeError|ValueError|AttributeError|NameError|'
    r'django.core.exceptions..*|RuntimeError|subprocess.CalledProcessError|'
    r'testtools.matchers._impl.MismatchError|'
    r'PermissionError|IndexError|TypeError|AssertionError|IOError|ImportError|'
    r'SerialException|OSError|qtawesome.iconic_font.FontError|'
    'redis.exceptions.ConnectionError|builtins.OverflowError|ArgumentError|'
    'httptools.parser.errors.HttpParserInvalidURLError|HypothesisException|'
    'SSLError|KeyError|Exception|rnc2rng.parser.ParseError|'
    'pkg_resources.UnknownExtra|tarfile.ReadError|'
    'numpydoc.docscrape.ParseError|'
    'datalad.support.exceptions.IncompleteResultsError'
    r'): .*',
    '^E   DeprecationWarning: .*',
    '^E       fixture \'(.*)\' not found',
    # Rake
    r'[0-9]+ runs, [0-9]+ assertions, [0-9]+ failures, [0-9]+ errors, '
    r'[0-9]+ skips',
    # Node
    r'# failed [0-9]+ of [0-9]+ tests',
    # Pytest
    r'(.*).py:[0-9]+: AssertionError',
    # Perl
    r'  Failed tests:  [0-9-]+',
    # Go
    'FAIL\t(.*)\t[0-9.]+s',
    r'.*.go:[0-9]+:[0-9]+: (?!note:).*',
    r'can\'t load package: package \.: no Go files in /<<PKGBUILDDIR>>/(.*)',
    # Ld
    r'\/usr\/bin\/ld: cannot open output file (.*): No such file or directory',
    r'configure: error: (.*)',
    r'config.status: error: (.*)',
    r'E: Build killed with signal TERM after ([0-9]+) minutes of inactivity',
    r'    \[javac\] [^: ]+:[0-9]+: error: (.*)',
    r'1\) TestChannelFeature: ([^:]+):([0-9]+): assert failed',
    r'cp: target \'(.*)\' is not a directory',
    r'cp: cannot create regular file \'(.*)\': No such file or directory',
    r'couldn\'t determine home directory at (.*)',
    r'ln: failed to create symbolic link \'(.*)\': File exists',
    r'ln: failed to create symbolic link \'(.*)\': No such file or directory',
    r'ln: failed to create symbolic link \'(.*)\': Permission denied',
    r'ln: invalid option -- .*',
    r'mkdir: cannot create directory ‘(.*)’: No such file or directory',
    r'mkdir: cannot create directory ‘(.*)’: File exists',
    r'mkdir: missing operand',
    r'Fatal error: .*',
    'Fatal Error: (.*)',
    r'ERROR: Test "(.*)" failed. Exiting.',
    # scons
    r'ERROR: test\(s\) failed in (.*)',
    r'./configure: line [0-9]+: syntax error near unexpected token `.*\'',
    r'scons: \*\*\* \[.*\] ValueError : unsupported pickle protocol: .*',
    # yarn
    r'ERROR: There are no scenarios; must have at least one.',
    # perl
    r'Execution of (.*) aborted due to compilation errors.',
    r'ls: cannot access \'(.*)\': No such file or directory',
    r'Problem opening (.*): No such file or directory at (.*) line ([0-9]+)\.',
    # Mocha
    r'     AssertionError \[ERR_ASSERTION\]: Missing expected exception.',
    # lt (C++)
    r'.*: .*:[0-9]+: .*: Assertion `.*\' failed.',
    r'(.*).xml: FAILED:',
    # ninja
    r'ninja: build stopped: subcommand failed.',
    r'.*\.s:[0-9]+: Error: .*',
    # rollup
    r'\[\!\] Error: Unexpected token',
    # glib
    r'\(.*:[0-9]+\): [a-zA-Z0-9]+-CRITICAL \*\*: [0-9:.]+: .*',
    r'tar: option requires an argument -- \'.\'',
    # rsvg-convert
    r'Could not render file (.*.svg)',
    # pybuild tests
    r'ERROR: file not found: (.*)',
    # msgfmt
    r'/usr/bin/msgfmt: found [0-9]+ fatal errors',
    # Docker
    r'Cannot connect to the Docker daemon at '
    r'unix:///var/run/docker.sock. Is the docker daemon running\?',
    r'dh_makeshlibs: failing due to earlier errors',
    # Ruby
    r'([^:]+)\.rb:[0-9]+:in `([^\'])+\': (.*) \((.*)\)',
    r'.*: \*\*\* ERROR: '
    r'There where errors/warnings in server logs after running test cases.',
    r'Errno::EEXIST: File exists @ dir_s_mkdir - .*',
    r'Test environment was found to be incomplete at configuration time,',
    r'libtool:   error: cannot find the library \'(.*)\' or '
    r'unhandled argument \'(.*)\'',
    r'npm ERR\! (.*)',
    r'install: failed to access \'(.*)\': (.*)',
    r'MSBUILD: error MSBUILD[0-9]+: Project file \'(.*)\' not found.',
    r'E: (.*)',
    r'(.*)\(([0-9]+),([0-9]+)\): Error: .*',
    # C #
    r'(.*)\.cs\([0-9]+,[0-9]+\): error CS[0-9]+: .*',
    r'.*Segmentation fault.*',
    r'a2x: ERROR: (.*) returned non-zero exit status ([0-9]+)',
    r'-- Configuring incomplete, errors occurred\!',
    r'Error opening link script "(.*)"',
    r'cc: error: (.*)',
    r'\[ERROR\] .*',
    r'dh_auto_(test|build): error: (.*)',
]

compiled_secondary_build_failure_regexps = [
    re.compile(regexp) for regexp in secondary_build_failure_regexps]

DEFAULT_LOOK_BACK = 50


def strip_useless_build_tail(lines, look_back=None):
    if look_back is None:
        look_back = DEFAULT_LOOK_BACK

    # Strip off unuseful tail
    for i, line in enumerate(lines[-look_back:]):
        if line.startswith('Build finished at '):
            lines = lines[:len(lines)-(look_back-i)]
            if lines and lines[-1] == ('-' * 80 + '\n'):
                lines = lines[:-1]
            break
    try:
        end_offset = lines.index('==> config.log <==\n')
    except ValueError:
        pass
    else:
        lines = lines[:end_offset]

    return lines


def find_build_failure_description(
        lines: List[str]
        ) -> Tuple[Optional[int], Optional[str], Optional['Problem']]:
    """Find the key failure line in build output.

    Returns:
      tuple with (line offset, line, error object)
    """
    OFFSET = 150
    # Is this cmake-specific, or rather just kf5 / qmake ?
    cmake = False
    # We search backwards for clear errors.
    for i in range(1, OFFSET):
        lineno = len(lines) - i
        if lineno < 0:
            break
        if 'cmake' in lines[lineno]:
            cmake = True
        for matcher in compiled_build_failure_regexps:
            linenos, err = matcher.match(lines, lineno)
            if linenos:
                lineno = linenos[-1]  # For now
                return lineno + 1, lines[lineno].rstrip('\n'), err

    # TODO(jelmer): Remove this in favour of CMakeErrorMatcher above.
    if cmake:
        missing_file_pat = re.compile(
            r'\s*The imported target \"(.*)\" references the file')
        conf_file_pat = re.compile(
            r'\s*Could not find a configuration file for package "(.*)".*')
        binary_pat = re.compile(r'  Could NOT find (.*) \(missing: .*\)')
        cmake_files_pat = re.compile(
            '^  Could not find a package configuration file provided '
            'by "(.*)" with any of the following names:')
        # Urgh, multi-line regexes---
        for lineno in range(len(lines)):
            m = re.fullmatch(binary_pat, lines[lineno].rstrip('\n'))
            if m:
                return (lineno + 1, lines[lineno],
                        MissingCommand(m.group(1).lower()))
            m = re.fullmatch(missing_file_pat, lines[lineno].rstrip('\n'))
            if m:
                lineno += 1
                while lineno < len(lines) and not lines[lineno].strip('\n'):
                    lineno += 1
                if lines[lineno+2].startswith(
                        '  but this file does not exist.'):
                    m = re.fullmatch(r'\s*"(.*)"', lines[lineno].rstrip('\n'))
                    if m:
                        filename = m.group(1)
                    else:
                        filename = lines[lineno].rstrip('\n')
                    return lineno + 1, lines[lineno], MissingFile(filename)
                continue
            m = re.fullmatch(conf_file_pat, lines[lineno].rstrip('\n'))
            if m:
                package = m.group(1)
                m = re.match(
                    r'.*requested version "(.*)"\.',
                    lines[lineno+1].rstrip('\n'))
                if not m:
                    logger.warn(
                        'expected version string in line %r', lines[lineno+1])
                    continue
                version = m.group(1)
                return (
                    lineno + 1, lines[lineno],
                    MissingPkgConfig(package, version))
            if lineno+1 < len(lines):
                m = re.fullmatch(
                    cmake_files_pat,
                    lines[lineno].strip('\n') + ' ' +
                    lines[lineno+1].lstrip(' ').strip('\n'))
                if m and lines[lineno+2] == '\n':
                    i = 3
                    filenames = []
                    while lines[lineno + i].strip():
                        filenames.append(lines[lineno + i].strip())
                        i += 1
                    return (
                        lineno + 1, lines[lineno],
                        CMakeFilesMissing(filenames))

    # And forwards for vague ("secondary") errors.
    for lineno in range(max(0, len(lines) - OFFSET), len(lines)):
        line = lines[lineno].strip('\n')
        for regexp in compiled_secondary_build_failure_regexps:
            m = regexp.fullmatch(line.rstrip('\n'))
            if m:
                return lineno + 1, line, None
    return None, None, None


class AutopkgtestDepsUnsatisfiable(Problem):

    kind = 'badpkg'

    def __init__(self, args):
        self.args = args

    @classmethod
    def from_blame_line(cls, line):
        args = []
        entries = line[len('blame: '):].rstrip('\n').split(' ')
        for entry in entries:
            try:
                (kind, arg) = entry.split(':', 1)
            except ValueError:
                kind = None
                arg = entry
            args.append((kind, arg))
            if kind not in ('deb', 'arg', 'dsc', None):
                logger.warn('unknown entry %s on badpkg line', entry)
        return cls(args)

    def __eq__(self, other):
        return type(self) == type(other) and \
               self.args == other.args

    def __repr__(self):
        return "%s(args=%r)" % (type(self).__name__, self.args)


class AutopkgtestTimedOut(Problem):

    kind = 'timed-out'

    def __init__(self):
        pass

    def __str__(self):
        return "Timed out"

    def __repr__(self):
        return "%s()" % (type(self).__name__)

    def __eq__(self, other):
        return isinstance(self, type(other))


class AutopkgtestTestbedFailure(Problem):

    kind = 'testbed-failure'

    def __init__(self, reason):
        self.reason = reason

    def __eq__(self, other):
        return type(self) == type(other) and self.reason == other.reason

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.reason)

    def __str__(self):
        return self.reason


class AutopkgtestDepChrootDisappeared(Problem):

    kind = 'testbed-chroot-disappeared'

    def __init__(self):
        pass

    def __str__(self):
        return "chroot disappeared"

    def __repr__(self):
        return "%s()" % (type(self).__name__)

    def __eq__(self, other):
        return isinstance(self, type(other))


class AutopkgtestErroneousPackage(Problem):

    kind = 'erroneous-package'

    def __init__(self, reason):
        self.reason = reason

    def __eq__(self, other):
        return type(self) == type(other) and self.reason == other.reason

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.reason)

    def __str__(self):
        return self.reason


class AutopkgtestStderrFailure(Problem):

    kind = 'stderr-output'

    def __init__(self, stderr_line):
        self.stderr_line = stderr_line

    def __eq__(self, other):
        return (isinstance(self, type(other)) and
                self.stderr_line == other.stderr_line)

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.stderr_line)

    def __str__(self):
        return "output on stderr: %s" % self.stderr_line


def parse_autopgktest_line(line: str) -> Union[
        str, Tuple[str, Union[Tuple[str, ...]]]]:
    m = re.match(r'autopkgtest \[([0-9:]+)\]: (.*)', line)
    if not m:
        return line
    timestamp = m.group(1)
    message = m.group(2)
    if message.startswith('@@@@@@@@@@@@@@@@@@@@ source '):
        return (timestamp, ('source', ))
    elif message.startswith('@@@@@@@@@@@@@@@@@@@@ summary'):
        return (timestamp, ('summary', ))
    elif message.startswith('test '):
        (testname, test_status) = message[len('test '):].rstrip(
                '\n').split(': ', 1)
        if test_status == '[-----------------------':
            return (timestamp, ('test', testname, 'begin output', ))
        elif test_status == '-----------------------]':
            return (timestamp, ('test', testname, 'end output', ))
        elif test_status == (
                ' - - - - - - - - - - results - - - - - - - - - -'):
            return (timestamp, ('test', testname, 'results', ))
        elif test_status == (
                ' - - - - - - - - - - stderr - - - - - - - - - -'):
            return (timestamp, ('test', testname, 'stderr', ))
        elif test_status == 'preparing testbed':
            return (timestamp, ('test', testname, 'prepare testbed'))
        else:
            return (timestamp, ('test', testname, test_status))
    elif message.startswith('ERROR:'):
        return (timestamp, ('error', message[len('ERROR: '):]))
    else:
        return (timestamp, (message, ))


def parse_autopkgtest_summary(lines):
    i = 0
    while i < len(lines):
        line = lines[i]
        m = re.match('([^ ]+)(?:[ ]+)PASS', line)
        if m:
            yield i, m.group(1), 'PASS', None, []
            i += 1
            continue
        m = re.match('([^ ]+)(?:[ ]+)(FAIL|PASS|SKIP) (.+)', line)
        if not m:
            i += 1
            continue
        testname = m.group(1)
        result = m.group(2)
        reason = m.group(3)
        offset = i
        extra = []
        if reason == 'badpkg':
            while (i+1 < len(lines) and (
                    lines[i+1].startswith('badpkg:') or
                    lines[i+1].startswith('blame:'))):
                extra.append(lines[i+1])
                i += 1
        yield offset, testname, result, reason, extra
        i += 1


class AutopkgtestTestbedSetupFailure(Problem):

    kind = 'testbed-setup-failure'

    def __init__(self, command, exit_status, error):
        self.command = command
        self.exit_status = exit_status
        self.error = error

    def __str__(self):
        return "Error setting up testbed %r failed (%d): %s" % (
            self.command, self.exit_status, self.error)

    def __repr__(self):
        return "%s(%r, %r, %r)" % (
            type(self).__name__, self.command, self.exit_status, self.error)

    def __eq__(self, other):
        return (
            isinstance(other, type(self)) and
            self.command == other.command and
            self.exit_status == other.exit_status and
            self.error == other.error)


class ChrootNotFound(Problem):

    kind = 'chroot-not-found'

    def __init__(self, chroot):
        self.chroot = chroot

    def __str__(self):
        return "Chroot not found: %s" % self.chroot

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.chroot)

    def __eq__(self, other):
        return isinstance(self, type(other)) and self.chroot == other.chroot


def find_testbed_setup_failure(lines):
    for i in range(len(lines)-1, 0, -1):
        line = lines[i]
        m = re.fullmatch(
            r'\[(.*)\] failed \(exit status ([0-9]+), stderr \'(.*)\'\)\n',
            line)
        if m:
            command = m.group(1)
            status_code = int(m.group(2))
            stderr = m.group(3)
            m = re.fullmatch(r'E: (.*): Chroot not found\\n', stderr)
            if m:
                return (i + 1, line, ChrootNotFound(m.group(1)))
            return (
                i + 1, line,
                AutopkgtestTestbedSetupFailure(command, status_code, stderr))
    return None, None, None


def find_autopkgtest_failure_description(
        lines: List[str]) -> Tuple[
                Optional[int], Optional[str], Optional['Problem'],
                Optional[str]]:
    """Find the autopkgtest failure in output.

    Returns:
      tuple with (line offset, testname, error, description)
    """
    error: Optional['Problem']
    test_output: Dict[Tuple[str, ...], List[str]] = {}
    test_output_offset: Dict[Tuple[str, ...], int] = {}
    current_field: Optional[Tuple[str, ...]] = None
    i = -1
    while i < len(lines) - 1:
        i += 1
        line = lines[i]
        parsed = parse_autopgktest_line(line)
        if isinstance(parsed, tuple):
            (timestamp, content) = parsed
            if content[0] == 'test':
                if content[2] == 'end output':
                    current_field = None
                    continue
                elif content[2] == 'begin output':
                    current_field = (content[1], 'output')
                else:
                    current_field = (content[1], content[2])
                if current_field in test_output:
                    logger.warn(
                        'duplicate output fields for %r', current_field)
                test_output[current_field] = []
                test_output_offset[current_field] = i + 1
            elif content == ('summary', ):
                current_field = ('summary', )
                test_output[current_field] = []
                test_output_offset[current_field] = i + 1
            elif content[0] == 'error':
                if content[1].startswith('"') and content[1].count('"') == 1:
                    sublines = [content[1]]
                    while i < len(lines):
                        i += 1
                        sublines += lines[i]
                        if lines[i].count('"') == 1:
                            break
                    content = (content[0], ''.join(sublines))
                last_test: Optional[str]
                if current_field is not None:
                    last_test = current_field[0]
                else:
                    last_test = None
                msg = content[1]
                m = re.fullmatch('"(.*)" failed with stderr "(.*)("?)', msg)
                if m:
                    stderr = m.group(2)
                    m = re.fullmatch(
                        'W: (.*): '
                        'Failed to stat file: No such file or directory',
                        stderr)
                    if m:
                        error = AutopkgtestDepChrootDisappeared()
                        return (i + 1, last_test, error, stderr)
                m = re.fullmatch(r'testbed failure: (.*)', msg)
                if m:
                    testbed_failure_reason = m.group(1)
                    if (current_field is not None and
                            testbed_failure_reason ==
                            'testbed auxverb failed with exit code 255'):
                        field = (current_field[0], 'output')
                        (offset, description, error) = (
                            find_build_failure_description(test_output[field]))
                        if error is not None:
                            assert offset is not None
                            return (
                                test_output_offset[field] + offset, last_test,
                                error, description)

                    if (testbed_failure_reason ==
                            "sent `auxverb_debug_fail', got `copy-failed', "
                            "expected `ok...'"):
                        (offset, description, error) = (
                            find_build_failure_description(lines))
                        if error is not None:
                            assert offset is not None
                            return (offset, last_test, error, description)

                    if (testbed_failure_reason ==
                            'cannot send to testbed: [Errno 32] Broken pipe'):
                        offset, line, error = find_testbed_setup_failure(lines)
                        if error and offset:
                            return (offset, last_test, error, line)
                    if (testbed_failure_reason ==
                            'apt repeatedly failed to download packages'):
                        offset, line, error = find_apt_get_failure(lines)
                        if error and offset:
                            return (offset, last_test, error, line)
                        return (i + 1, last_test,
                                AptFetchFailure(None, testbed_failure_reason),
                                None)
                    return (i + 1, last_test,
                            AutopkgtestTestbedFailure(testbed_failure_reason),
                            None)
                m = re.fullmatch(r'erroneous package: (.*)', msg)
                if m:
                    (offset, description, error) = (
                        find_build_failure_description(lines[:i]))
                    if error:
                        return (offset, last_test, error, description)
                    return (i + 1, last_test,
                            AutopkgtestErroneousPackage(m.group(1)), None)
                if current_field is not None:
                    offset, line, error = find_apt_get_failure(
                        test_output[current_field])
                    if (error is not None and offset is not None and
                            current_field in test_output_offset):
                        return (test_output_offset[current_field] + offset,
                                last_test, error, line)
                return (i + 1, last_test, None, msg)
        else:
            if current_field:
                test_output[current_field].append(line)

    try:
        summary_lines = test_output[('summary', )]
        summary_offset = test_output_offset[('summary', )]
    except KeyError:
        while lines and not lines[-1].strip():
            lines.pop(-1)
        if not lines:
            return (None, None, None, None)
        else:
            return (len(lines), lines[-1], None, None)
    else:
        for (lineno, testname, result, reason,
             extra) in parse_autopkgtest_summary(summary_lines):
            if result in ('PASS', 'SKIP'):
                continue
            assert result == 'FAIL'
            if reason == 'timed out':
                error = AutopkgtestTimedOut()
                return (summary_offset+lineno+1, testname, error, reason)
            elif reason.startswith('stderr: '):
                output = reason[len('stderr: '):]
                stderr_lines = test_output.get((testname, 'stderr'), [])
                stderr_offset = test_output_offset.get((testname, 'stderr'))
                if stderr_lines:
                    (offset, description, error) = (
                        find_build_failure_description(stderr_lines))
                    if offset is not None and stderr_offset is not None:
                        offset += stderr_offset - 1
                else:
                    (_, description, error) = find_build_failure_description(
                        [output])
                    offset = None
                if offset is None:
                    offset = summary_offset + lineno
                if error is None:
                    error = AutopkgtestStderrFailure(output)
                    if description is None:
                        description = (
                            'Test %s failed due to '
                            'unauthorized stderr output: %s' % (
                                testname, error.stderr_line))
                return offset + 1, testname, error, description
            elif reason == 'badpkg':
                output_lines = test_output.get(
                    (testname, 'prepare testbed'), [])
                output_offset = test_output_offset.get(
                    (testname, 'prepare testbed'))
                if output_lines and output_offset:
                    offset, line, error = find_apt_get_failure(output_lines)
                    if error and offset:
                        return (offset + output_offset + 1, testname, error,
                                None)
                badpkg = None
                blame = None
                for line in extra:
                    if line.startswith('badpkg: '):
                        badpkg = line[len('badpkg: '):]
                    if line.startswith('blame: '):
                        blame = line
                if badpkg is not None:
                    description = 'Test %s failed: %s' % (
                        testname, badpkg.rstrip('\n'))
                else:
                    description = 'Test %s failed' % testname

                error = AutopkgtestDepsUnsatisfiable.from_blame_line(blame)
                return (summary_offset + lineno + 1, testname, error,
                        description)
            else:
                output_lines = test_output.get((testname, 'output'), [])
                output_offset = test_output_offset.get((testname, 'output'))
                (error_offset, description, error) = (
                    find_build_failure_description(output_lines))
                if error_offset is None or output_offset is None:
                    offset = summary_offset + lineno
                else:
                    offset = error_offset + output_offset
                if description is None:
                    description = 'Test %s failed: %s' % (testname, reason)
                return offset+1, testname, error, description  # type: ignore

    return None, None, None, None


class AptUpdateError(Problem):
    """Apt update error."""

    kind = 'apt-update-error'


class AptFetchFailure(AptUpdateError):
    """Apt file fetch failed."""

    kind = 'apt-file-fetch-failure'

    def __init__(self, url, error):
        self.url = url
        self.error = error

    def __eq__(self, other):
        if not isinstance(other, type(self)):
            return False
        if self.url != other.url:
            return False
        if self.error != other.error:
            return False
        return True

    def __str__(self):
        return 'Apt file fetch error: %s' % self.error


class AptMissingReleaseFile(AptUpdateError):

    kind = 'missing-release-file'

    def __init__(self, url):
        self.url = url

    def __eq__(self, other):
        if not isinstance(other, type(self)):
            return False
        if self.url != self.url:
            return False
        return True

    def __str__(self):
        return 'Missing release file: %s' % self.url


class AptPackageUnknown(Problem):

    kind = 'apt-package-unknown'

    def __init__(self, package):
        self.package = package

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.package == other.package

    def __str__(self):
        return "Unknown package: %s" % self.package

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.package)


class DpkgError(Problem):

    kind = 'dpkg-error'

    def __init__(self, error):
        self.error = error

    def __eq__(self, other):
        return isinstance(other, type(self)) and self.error == other.error

    def __str__(self):
        return "Dpkg Error: %s" % self.error

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.error)


def find_cudf_output(lines):
    for i in range(len(lines)-1, 0, -1):
        if lines[i].startswith('output-version: '):
            break
    else:
        return None
    output = []
    while lines[i].strip():
        output.append(lines[i])
        i += 1

    return yaml.safe_load('\n'.join(output))


class UnsatisfiedDependencies(Problem):

    kind = 'unsatisfied-dependencies'

    def __init__(self, relations):
        self.relations = relations

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
                self.relations == other.relations

    def __str__(self):
        return "Unsatisfied dependencies: %s" % PkgRelation.str(self.relations)

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.relations)


class UnsatisfiedConflicts(Problem):

    kind = 'unsatisfied-conflicts'

    def __init__(self, relations):
        self.relations = relations

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
                self.relations == other.relations

    def __str__(self):
        return "Unsatisfied conflicts: %s" % PkgRelation.str(self.relations)

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.relations)


def error_from_dose3_report(report):
    packages = [entry['package'] for entry in report]
    assert packages == ['sbuild-build-depends-main-dummy']
    if report[0]['status'] != 'broken':
        return None
    missing = []
    conflict = []
    for reason in report[0]['reasons']:
        if 'missing' in reason:
            relation = PkgRelation.parse_relations(
                reason['missing']['pkg']['unsat-dependency'])
            missing.extend(relation)
        if 'conflict' in reason:
            relation = PkgRelation.parse_relations(
                reason['conflict']['pkg1']['unsat-conflict'])
            conflict.extend(relation)
    if missing:
        return UnsatisfiedDependencies(missing)
    if conflict:
        return UnsatisfiedConflicts(conflict)


class AptBrokenPackages(Problem):

    kind = 'apt-broken-packages'

    def __init__(self, description):
        self.description = description

    def __str__(self):
        return "Broken apt packages: %s" % (self.description, )

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, self.description)

    def __eq__(self, other):
        return isinstance(other, type(self)) and \
                self.description == other.description


def find_apt_get_failure(lines):
    """Find the key failure line in apt-get-output.

    Returns:
      tuple with (line offset, line, error object)
    """
    ret = (None, None, None)
    OFFSET = 50
    for i in range(1, OFFSET):
        lineno = len(lines) - i
        if lineno < 0:
            break
        line = lines[lineno].strip('\n')
        if line.startswith('E: Failed to fetch '):
            m = re.match(
                '^E: Failed to fetch ([^ ]+)  (.*)', line)
            if m:
                return lineno + 1, line, AptFetchFailure(
                    m.group(1), m.group(2))
            return lineno + 1, line, None
        if line in (
                'E: Broken packages',
                'E: Unable to correct problems, you have held broken '
                'packages.'):
            error = AptBrokenPackages(lines[lineno-1].strip())
            return lineno, lines[lineno-1].strip(), error
        m = re.match(
            'E: The repository \'([^\']+)\' does not have a Release file.',
            line)
        if m:
            return lineno + 1, line, AptMissingReleaseFile(m.group(1))
        m = re.match(
            'dpkg-deb: error: unable to write file \'(.*)\': '
            'No space left on device', line)
        if m:
            return lineno + 1, line, NoSpaceOnDevice()
        m = re.match('E: You don\'t have enough free space in (.*)\.', line)
        if m:
            return lineno + 1, line, NoSpaceOnDevice()
        if line.startswith('E: ') and ret[0] is None:
            ret = (lineno + 1, line, None)
        m = re.match('E: Unable to locate package (.*)', line)
        if m:
            return lineno + 1, line, AptPackageUnknown(m.group(1))
        m = re.match('dpkg: error: (.*)', line)
        if m:
            if m.group(1).endswith(': No space left on device'):
                return lineno + 1, line, NoSpaceOnDevice()
            return lineno + 1, line, DpkgError(m.group(1))
        m = re.match(r'dpkg: error processing package (.*) \((.*)\):', line)
        if m:
            return lineno + 2, lines[lineno + 1].strip(), DpkgError(
                'processing package %s (%s)' % (m.group(1), m.group(2)))

    for i, line in enumerate(lines):
        m = re.match(
            ' cannot copy extracted data for \'(.*)\' to '
            '\'(.*)\': failed to write \(No space left on device\)',
            line)
        if m:
            return lineno + i, line, NoSpaceOnDevice()
        m = re.match(' .*: No space left on device', line)
        if m:
            return lineno + i, line, NoSpaceOnDevice()

    return ret


class ArchitectureNotInList(Problem):

    kind = 'arch-not-in-list'

    def __init__(self, arch, arch_list):
        self.arch = arch
        self.arch_list = arch_list

    def __repr__(self):
        return "%s(%r, %r)" % (
            type(self).__name__, self.arch, self.arch_list)

    def __str__(self):
        return "Architecture %s not a build arch" % (self.arch, )

    def __eq__(self, other):
        return (
            isinstance(other, type(self)) and
            self.arch == other.arch and
            self.arch_list == other.arch_list)


def find_arch_check_failure_description(lines):
    for offset, line in enumerate(lines):
        m = re.match(
            r'E: dsc: (.*) not in arch list or does not match any arch '
            r'wildcards: (.*) -- skipping', line)
        if m:
            error = ArchitectureNotInList(m.group(1), m.group(2))
            return offset, line, error
    return len(lines) - 1, lines[-1], None


class InsufficientDiskSpace(Problem):

    kind = 'insufficient-disk-space'

    def __init__(self, needed, free):
        self.needed = needed
        self.free = free

    def __eq__(self, other):
        return (isinstance(other, type(self)) and
                self.needed == other.needed and
                self.free == other.free)

    def __repr__(self):
        return "%s(%r, %r)" % (type(self).__name__, self.needed, self.free)

    def __str__(self):
        return ("Insufficient disk space for build. "
                "Need: %d KiB, free: %s KiB" % (self.needed, self.free))


def find_check_space_failure_description(lines):
    for offset, line in enumerate(lines):
        if line == 'E: Disk space is probably not sufficient for building.\n':
            m = re.fullmatch(
                r'I: Source needs ([0-9]+) KiB, '
                r'while ([0-9]+) KiB is free.\)\n',
                lines[offset+1])
            if m:
                return (offset + 1, line,
                        InsufficientDiskSpace(
                            int(m.group(1)), int(m.group(2))))
            return (offset + 1, line, None)


def find_install_deps_failure_description(paragraphs):
    error = None
    DOSE3_SECTION = 'install dose3 build dependencies (aspcud-based resolver)'
    dose3_lines = paragraphs.get(DOSE3_SECTION)
    if dose3_lines:
        dose3_output = find_cudf_output(dose3_lines)
        if dose3_output:
            error = error_from_dose3_report(dose3_output['report'])

    for focus_section, lines in paragraphs.items():
        if focus_section is None:
            continue
        if re.match('install (.*) build dependencies.*', focus_section):
            offset, line, v_error = find_apt_get_failure(lines)
            if error is None:
                error = v_error
            if offset is not None:
                return focus_section, offset, line, error

    return focus_section, None, None, error


def find_apt_get_update_failure(paragraphs):
    focus_section = 'update chroot'
    lines = paragraphs.get(focus_section, [])
    offset, line, error = find_apt_get_failure(lines)
    return focus_section, offset, line, error

