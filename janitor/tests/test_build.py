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

    def run_test(self, lines, line):
        self.assertEqual(
            line, find_build_failure_description(lines))

    def test_make_missing_rule(self):
        self.run_test([
            'make[1]: *** No rule to make target \'nno.autopgen.bin\', '
            'needed by \'dan-nno.autopgen.bin\'.  Stop.'],
            'make[1]: *** No rule to make target \'nno.autopgen.bin\', '
            'needed by \'dan-nno.autopgen.bin\'.  Stop.')

    def test_installdocs_missing(self):
        self.run_test([
            'dh_installdocs: Cannot find (any matches for) "README.txt" '
            '(tried in ., debian/tmp)'],
            'dh_installdocs: Cannot find (any matches for) "README.txt" '
            '(tried in ., debian/tmp)')

    def test_distutils_missing(self):
        self.run_test([
            'distutils.errors.DistutilsError: Could not find suitable '
            'distribution for Requirement.parse(\'pytest-runner\')'],
            'distutils.errors.DistutilsError: Could not find suitable '
            'distribution for Requirement.parse(\'pytest-runner\')')

    def test_pytest_import(self):
        self.run_test([
            'E   ImportError: cannot import name cmod'],
            'E   ImportError: cannot import name cmod')
