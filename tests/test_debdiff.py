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

from janitor.debian.debdiff import filter_boring


def test_just_versions():
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
    newdebdiff = filter_boring(debdiff, "0.143-4~jan+unchanged1", "0.143-5~jan+lint1")
    assert (
        newdebdiff
        == """\
File lists identical (after any substitutions)

No differences were encountered between the control files of package \
acpi-fakekey

No differences were encountered between the control files of package \
acpi-fakekey-dbgsym

No differences were encountered between the control files of package \
acpi-support

No differences were encountered between the control files of package \
acpi-support-base
"""
    )
