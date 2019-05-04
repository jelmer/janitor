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

from janitor.build import find_build_failure_description
import unittest


class FindBuildFailureDescriptionTests(unittest.TestCase):

    def run_test(self, lines, lineno):
        self.assertEqual(
            (lineno, lines[lineno-1]), find_build_failure_description(lines))

    def test_make_missing_rule(self):
        self.run_test([
            'make[1]: *** No rule to make target \'nno.autopgen.bin\', '
            'needed by \'dan-nno.autopgen.bin\'.  Stop.'],
            1)

    def test_installdocs_missing(self):
        self.run_test([
            'dh_installdocs: Cannot find (any matches for) "README.txt" '
            '(tried in ., debian/tmp)'],
            1)

    def test_distutils_missing(self):
        self.run_test([
            'distutils.errors.DistutilsError: Could not find suitable '
            'distribution for Requirement.parse(\'pytest-runner\')'],
            1)
        self.run_test([
            'error: Could not find suitable distribution for '
            'Requirement.parse(\'gitlab\')'], 1)

    def test_pytest_import(self):
        self.run_test([
            'E   ImportError: cannot import name cmod'], 1)
        self.run_test([
            'E   ImportError: No module named mock'], 1)

    def test_python2_import(self):
        self.run_test(['ImportError: No module named pytz'], 1)

    def test_python3_import(self):
        self.run_test([
            'ModuleNotFoundError: No module named \'django_crispy_forms\''], 1)
        self.run_test([
            'ModuleNotFoundError: No module named \'distro\''], 1)

    def test_go_missing(self):
        self.run_test([
            'src/github.com/vuls/config/config.go:30:2: cannot find package '
            '"golang.org/x/xerrors" in any of:'], 1)
