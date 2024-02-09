#!/usr/bin/python
# Copyright (C) 2022 Jelmer Vernooij <jelmer@jelmer.uk>
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

import tempfile

from janitor.worker import DebianTarget

from . import TestCaseWithTransport


class DebianTargetTests(TestCaseWithTransport):
    def setUp(self):
        super().setUp()
        self.tree = self.make_branch_and_tree(".")

    def test_create(self):
        target = DebianTarget(
            {
                "COMMITTER": "Joe Example <joe@example.com>",
                "DEB_UPDATE_CHANGELOG": "auto",
            }
        )
        self.assertEqual(target.committer, "Joe Example <joe@example.com>")
        self.assertEqual(target.update_changelog, None)

    def test_make_chagnes(self):
        target = DebianTarget(
            {
                "COMMITTER": "Joe Example <joe@example.com>",
                "DEB_UPDATE_CHANGELOG": "auto",
            }
        )
        self.build_tree_contents([
            ("debian/", ),
            ("debian/changelog", "foo (0.1) unstable; urgency=low\n\n  * Initial release.\n\n -- Joe Example <joe@example.com>  Mon, 01 Jan 2001 00:00:00 +0000\n"),
            ])
        result = target.make_changes(
            self.tree, "", ["sh", "-c", "touch foo; echo Do a thing"], log_directory=tempfile.mkdtemp()
        )
        self.assertIs(result.value, None)
        self.assertEqual(result.description, "Do a thing\n")
        self.assertIs(result.context, None)
        self.assertEqual(result.tags, [])
