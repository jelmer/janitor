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

import sys
import unittest

from breezy.tests import TestCaseWithTransport
from debian.changelog import Version

from janitor.debian import dpkg_vendor, tree_set_changelog_version


class TreeSetChangelogVersionTests(TestCaseWithTransport):
    def test_set(self):
        tree = self.make_branch_and_tree(".")
        self.build_tree_contents(
            [
                ("debian/",),
                (
                    "debian/changelog",
                    """\
blah (0.39) UNRELEASED; urgency=medium

  * Properly cope with trailing commas when adding dependencies.

 -- Jelmer Vernooij <jelmer@debian.org>  Sat, 19 Oct 2019 15:50:25 +0000
""",
                ),
            ]
        )
        tree.add(["debian", "debian/changelog"])
        tree.commit("add changelog")
        tree_set_changelog_version(tree, Version("0.39~jan+lint1"), "")
        self.assertFileEqual(
            """\
blah (0.39~jan+lint1) UNRELEASED; urgency=medium

  * Properly cope with trailing commas when adding dependencies.

 -- Jelmer Vernooij <jelmer@debian.org>  Sat, 19 Oct 2019 15:50:25 +0000
""",
            "debian/changelog",
        )

    def test_is_higher(self):
        tree = self.make_branch_and_tree(".")
        self.build_tree_contents(
            [
                ("debian/",),
                (
                    "debian/changelog",
                    """\
blah (0.40) UNRELEASED; urgency=medium

  * Properly cope with trailing commas when adding dependencies.

 -- Jelmer Vernooij <jelmer@debian.org>  Sat, 19 Oct 2019 15:50:25 +0000
""",
                ),
            ]
        )
        tree.add(["debian", "debian/changelog"])
        tree.commit("add changelog")
        tree_set_changelog_version(tree, Version("0.39~jan+lint1"), "")
        self.assertFileEqual(
            """\
blah (0.40) UNRELEASED; urgency=medium

  * Properly cope with trailing commas when adding dependencies.

 -- Jelmer Vernooij <jelmer@debian.org>  Sat, 19 Oct 2019 15:50:25 +0000
""",
            "debian/changelog",
        )


def test_runs():
    if sys.platform != "linux":
        raise unittest.SkipTest("Only runs on Linux")
    assert isinstance(dpkg_vendor(), str)
