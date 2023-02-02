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

from unittest import mock

from janitor.artifacts import LocalArtifactManager
from janitor.differ import create_app


class DummyPool:

    async def acquire(self):
        return None


async def create_pool(loc):
    return DummyPool()


async def create_client(aiohttp_client):
    td = tempfile.TemporaryDirectory().name
    atd = tempfile.TemporaryDirectory().name
    afm = LocalArtifactManager(atd)
    database_location = None
    with mock.patch('janitor.state.create_pool', create_pool):
        return await aiohttp_client(create_app(td, afm, database_location))


async def test_health(aiohttp_client):
    client = await create_client(aiohttp_client)

    resp = await client.get("/health")
    assert resp.status == 200
    text = await resp.text()
    assert text == "ok"


async def test_ready(aiohttp_client):
    client = await create_client(aiohttp_client)

    resp = await client.get("/ready")
    assert resp.status == 200
    text = await resp.text()
    assert text == "ok"
