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

from janitor.debian.debdiff import iter_sections, filter_boring

import unittest


class IterSectionsTests(unittest.TestCase):
    def test_nothing(self):
        self.assertEqual([(None, ["foo"])], list(iter_sections("foo\n")))

    def test_simple(self):
        self.maxDiff = None
        self.assertEqual(
            [
                (
                    None,  # noqa
                    [
                        "[The following lists of changes regard files as different if they have",
                        "different names, permissions or owners.]",
                    ],
                ),
                (
                    "Files in second .changes but not in first",  # noqa
                    [
                        "-rw-r--r--  root/root   /usr/lib/debug/.build-id/e4/3520e0f1e.debug"
                    ],
                ),
                (
                    "Files in first .changes but not in second",  # noqa
                    [
                        "-rw-r--r--  root/root   /usr/lib/debug/.build-id/28/0303571bd.debug"
                    ],
                ),
                (  # noqa
                    "Control files of package xserver-blah: lines which differ (wdiff format)",
                    [
                        "Installed-Size: [-174-] {+170+}",
                        "Version: [-1:1.7.9-2~jan+unchanged1-] {+1:1.7.9-3~jan+lint1+}",
                    ],
                ),
                (  # noqa
                    "Control files of package xserver-dbgsym: lines which differ"
                    " (wdiff format)",
                    [
                        "Build-Ids: [-280303571bd7f8-] {+e43520e0f1eb+}",
                        "Depends: xserver-blah (= [-1:1.7.9-2~jan+unchanged1)-] "
                        "{+1:1.7.9-3~jan+lint1)+}",
                        "Installed-Size: [-515-] {+204+}",
                        "Version: [-1:1.7.9-2~jan+unchanged1-] {+1:1.7.9-3~jan+lint1+}",
                    ],
                ),
            ],
            list(
                iter_sections(
                    """\
[The following lists of changes regard files as different if they have
different names, permissions or owners.]

Files in second .changes but not in first
-----------------------------------------
-rw-r--r--  root/root   /usr/lib/debug/.build-id/e4/3520e0f1e.debug

Files in first .changes but not in second
-----------------------------------------
-rw-r--r--  root/root   /usr/lib/debug/.build-id/28/0303571bd.debug

Control files of package xserver-blah: lines which differ (wdiff format)
------------------------------------------------------------------------
Installed-Size: [-174-] {+170+}
Version: [-1:1.7.9-2~jan+unchanged1-] {+1:1.7.9-3~jan+lint1+}

Control files of package xserver-dbgsym: lines which differ (wdiff format)
--------------------------------------------------------------------------
Build-Ids: [-280303571bd7f8-] {+e43520e0f1eb+}
Depends: xserver-blah (= [-1:1.7.9-2~jan+unchanged1)-] {+1:1.7.9-3~jan+lint1)+}
Installed-Size: [-515-] {+204+}
Version: [-1:1.7.9-2~jan+unchanged1-] {+1:1.7.9-3~jan+lint1+}
"""
                )
            ),
        )


class FilterBoringTests(unittest.TestCase):
    def test_just_versions(self):
        debdiff = """\
File lists identical (after any substitutions)

Control files of package acpi-fakekey: lines which differ (wdiff format)
------------------------------------------------------------------------
Version: [-0.143-4~jan+unchanged1-] {+0.143-5~jan+lint1+}

Control files of package acpi-fakekey-dbgsym: lines which differ (wdiff format)
-------------------------------------------------------------------------------
Depends: acpi-fakekey (= [-0.143-4~jan+unchanged1)-] {+0.143-5~jan+lint1)+}
Version: [-0.143-4~jan+unchanged1-] {+0.143-5~jan+lint1+}

Control files of package acpi-support: lines which differ (wdiff format)
------------------------------------------------------------------------
Version: [-0.143-4~jan+unchanged1-] {+0.143-5~jan+lint1+}

Control files of package acpi-support-base: lines which differ (wdiff format)
-----------------------------------------------------------------------------
Version: [-0.143-4~jan+unchanged1-] {+0.143-5~jan+lint1+}
"""
        newdebdiff = filter_boring(
            debdiff, "0.143-4~jan+unchanged1", "0.143-5~jan+lint1"
        )
        self.maxDiff = None
        self.assertEqual(
            newdebdiff,
            """\
File lists identical (after any substitutions)

No differences were encountered between the control files of package \
acpi-fakekey

No differences were encountered between the control files of package \
acpi-fakekey-dbgsym

No differences were encountered between the control files of package \
acpi-support

No differences were encountered between the control files of package \
acpi-support-base
""",
        )
