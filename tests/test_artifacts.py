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

import os
import shutil
import tempfile

import pytest

from janitor.artifacts import ArtifactsMissing, LocalArtifactManager


@pytest.fixture
def local_artifact_manager():
    path = tempfile.mkdtemp()
    try:
        yield LocalArtifactManager(path)
    finally:
        shutil.rmtree(path)


@pytest.mark.asyncio
async def test_store_twice(local_artifact_manager):
    manager = local_artifact_manager
    with tempfile.TemporaryDirectory() as td:
        with open(os.path.join(td, "somefile"), "w") as f:
            f.write("lalala")
        await manager.store_artifacts("some-run-id", td)
        await manager.store_artifacts("some-run-id", td)


@pytest.mark.asyncio
async def test_store_and_retrieve(local_artifact_manager):
    manager = local_artifact_manager
    with tempfile.TemporaryDirectory() as td:
        with open(os.path.join(td, "somefile"), "w") as f:
            f.write("lalala")
        await manager.store_artifacts("some-run-id", td)
    with tempfile.TemporaryDirectory() as td:
        await manager.retrieve_artifacts("some-run-id", td)
        assert ["somefile"] == os.listdir(td)


@pytest.mark.asyncio
async def test_retrieve_nonexistent(local_artifact_manager):
    manager = local_artifact_manager
    with tempfile.TemporaryDirectory() as td:
        with pytest.raises(ArtifactsMissing):
            await manager.retrieve_artifacts("some-run-id", td)
