#!/usr/bin/python
# Copyright (C) 2020 Jelmer Vernooij <jelmer@jelmer.uk>
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

from ..vcs import bzr_to_browse_url, LocalVcsManager, get_run_diff
from . import TestCaseWithTransport

from breezy import controldir

import os
import unittest


class BzrToBrowseUrlTests(unittest.TestCase):

    def test_simple(self):
        self.assertEqual(
            'https://github.com/jelmer/dulwich',
            bzr_to_browse_url(
                'https://github.com/jelmer/dulwich'))

    def test_branch(self):
        self.assertEqual(
            'https://github.com/jelmer/dulwich/tree/master',
            bzr_to_browse_url(
                'https://github.com/jelmer/dulwich,branch=master'))
        self.assertEqual(
            'https://github.com/jelmer/dulwich/tree/debian/master',
            bzr_to_browse_url(
                'https://github.com/jelmer/dulwich,branch=debian%2Fmaster'))


class GetRunDiffsTests(TestCaseWithTransport):

    def test_diff(self):
        vcs_manager = LocalVcsManager('.')
        os.mkdir('bzr')
        self.make_repository('bzr/pkg', shared=True)
        branch = controldir.ControlDir.create_branch_convenience(
            'bzr/pkg/trunk', force_new_repo=False)
        tree = branch.controldir.open_workingtree()
        self.build_tree_contents([('bzr/pkg/trunk/a', """\
File a
""")])
        tree.add('a')
        old_revid = tree.commit('base')

        self.build_tree_contents([('bzr/pkg/trunk/a', """\
File a
File b
""")])
        new_revid = tree.commit('actual')

        class Run(object):

            package = 'pkg'
            main_branch_revision = old_revid
            revision = new_revid

        lines = get_run_diff(vcs_manager, Run(), 'main').splitlines()
        self.assertEqual(b"=== modified file 'a'", lines[0])
        self.assertEqual(b"@@ -1,1 +1,2 @@", lines[3])
        self.assertEqual(b" File a", lines[4])
        self.assertEqual(b"+File b", lines[5])
        self.assertEqual(b"", lines[6])
