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

import asyncio
import os

from breezy.tests import TestCaseWithTransport

from janitor.runner import run_subprocess


class RunSubprocessTests(TestCaseWithTransport):

    def test_run_simple(self):
        self.build_tree_contents([('test', '')])

        asyncio.run(run_subprocess(
            ['cat', os.path.join(self.test_dir, 'test')], {}))

    def test_read_from_stdin(self):
        self.build_tree_contents([('test', '')])

        asyncio.run(run_subprocess(
            ['cat'], {}))
