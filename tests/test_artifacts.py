#!/usr/bin/python
# Copyright (C) 2021 Jelmer Vernooij <jelmer@jelmer.uk>
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
import shutil
import tempfile
import unittest
from typing import Callable

from janitor.artifacts import ArtifactManager, ArtifactsMissing, LocalArtifactManager


class ArtifactManagerTests:
    manager: ArtifactManager

    assertEqual: Callable
    assertRaises: Callable

    async def test_store_twice(self):
        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "somefile"), "w") as f:
                f.write("lalala")
            await self.manager.store_artifacts("some-run-id", td)
            await self.manager.store_artifacts("some-run-id", td)

    async def test_store_and_retrieve(self):
        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "somefile"), "w") as f:
                f.write("lalala")
            loop = asyncio.get_event_loop()
            loop.run_until_complete(self.manager.store_artifacts("some-run-id", td))
        with tempfile.TemporaryDirectory() as td:
            loop.run_until_complete(self.manager.retrieve_artifacts("some-run-id", td))
            self.assertEqual(["somefile"], os.listdir(td))

    async def test_retrieve_nonexistent(self):
        loop = asyncio.get_event_loop()
        with tempfile.TemporaryDirectory() as td:
            self.assertRaises(
                ArtifactsMissing,
                loop.run_until_complete,
                self.manager.retrieve_artifacts("some-run-id", td),
            )


class LocalArtifactManagerTests(ArtifactManagerTests, unittest.TestCase):
    def setUp(self):
        super().setUp()
        self.path = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.path)
        self.manager = LocalArtifactManager(self.path)
