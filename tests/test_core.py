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


from janitor import splitout_env


def test_splitout_env():
    assert splitout_env("ls") == ({}, "ls")
    assert splitout_env("PATH=/bin ls") == ({"PATH": "/bin"}, "ls")
    assert splitout_env("PATH=/bin FOO=bar ls") == (
        {"PATH": "/bin", "FOO": "bar"},
        "ls",
    )
    assert splitout_env("PATH=/bin FOO=bar ls -l") == (
        {"PATH": "/bin", "FOO": "bar"},
        "ls -l",
    )
