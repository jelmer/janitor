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

from janitor.sbuild_log import (
    find_build_failure_description,
    MissingCHeader,
    MissingPythonModule,
    MissingGoPackage,
    MissingFile,
    MissingNodeModule,
    MissingCommand,
    )
import unittest


class FindBuildFailureDescriptionTests(unittest.TestCase):

    def run_test(self, lines, lineno, err=None):
        (offset, actual_line, actual_err) = find_build_failure_description(
            lines)
        self.assertEqual(actual_line, lines[lineno-1])
        self.assertEqual(lineno, offset)
        if err:
            self.assertEqual(actual_err, err)
        else:
            self.assertIs(None, actual_err)

    def test_make_missing_rule(self):
        self.run_test([
            'make[1]: *** No rule to make target \'nno.autopgen.bin\', '
            'needed by \'dan-nno.autopgen.bin\'.  Stop.'],
            1)
        self.run_test([
            'make[1]: *** No rule to make target \'/usr/share/blah/blah\', '
            'needed by \'dan-nno.autopgen.bin\'.  Stop.'],
            1, MissingFile('/usr/share/blah/blah'))

    def test_installdocs_missing(self):
        self.run_test([
            'dh_installdocs: Cannot find (any matches for) "README.txt" '
            '(tried in ., debian/tmp)'],
            1)

    def test_distutils_missing(self):
        self.run_test([
            'distutils.errors.DistutilsError: Could not find suitable '
            'distribution for Requirement.parse(\'pytest-runner\')'],
            1, MissingPythonModule('pytest-runner', None))
        self.run_test([
            "distutils.errors.DistutilsError: Could not find suitable "
            "distribution for Requirement.parse('certifi>=2019.3.9')"],
            1, MissingPythonModule('certifi', None, '2019.3.9'))
        self.run_test([
            'error: Could not find suitable distribution for '
            'Requirement.parse(\'gitlab\')'], 1,
            MissingPythonModule('gitlab', None))

    def test_pytest_import(self):
        self.run_test([
            'E   ImportError: cannot import name cmod'], 1,
            MissingPythonModule('cmod', 2))
        self.run_test([
            'E   ImportError: No module named mock'], 1,
            MissingPythonModule('mock', 2))

    def test_python2_import(self):
        self.run_test(
                ['ImportError: No module named pytz'], 1,
                MissingPythonModule('pytz', 2))

    def test_python3_import(self):
        self.run_test([
            'ModuleNotFoundError: No module named \'django_crispy_forms\''], 1,
            MissingPythonModule('django_crispy_forms', 3))
        self.run_test([
            'ModuleNotFoundError: No module named \'distro\''], 1,
            MissingPythonModule('distro', 3))

    def test_go_missing(self):
        self.run_test([
            'src/github.com/vuls/config/config.go:30:2: cannot find package '
            '"golang.org/x/xerrors" in any of:'], 1,
            MissingGoPackage('golang.org/x/xerrors'))

    def test_c_header_missing(self):
        self.run_test([
            'cdhit-common.h:39:9: fatal error: zlib.h: No such file '
            'or directory'], 1,
            MissingCHeader('zlib.h'))

    def test_node_module_missing(self):
        self.run_test([
            'Error: Cannot find module \'tape\''], 1,
            MissingNodeModule('tape'))

    def test_command_missing(self):
        self.run_test([
            './ylwrap: line 176: yacc: command not found'], 1,
            MissingCommand('yacc'))
