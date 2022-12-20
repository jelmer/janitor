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

from janitor import config_pb2
from janitor.bzr_store import (
    create_web_app,
)


async def create_client(aiohttp_client, codebases=None):
    config = config_pb2.Config()
    campaign = config.campaign.add()
    campaign.name = 'campaign'

    if codebases is None:
        codebases = set()

    async def check_codebase(n):
        return n in codebases

    app, public_app = await create_web_app(
        '127.0.0.1', 80, '/tmp', check_codebase, allow_writes=True, config=config)
    return (
        await aiohttp_client(app),
        await aiohttp_client(public_app))


async def test_health(aiohttp_client):
    client, public_client = await create_client(aiohttp_client)

    resp = await client.get("/health")
    assert resp.status == 200
    text = await resp.text()
    assert text == "ok"


async def test_ready(aiohttp_client):
    client, public_client = await create_client(aiohttp_client)

    resp = await client.get("/ready")
    assert resp.status == 200
    text = await resp.text()
    assert text == "ok"


async def test_home(aiohttp_client):
    client, public_client = await create_client(aiohttp_client)

    with tempfile.TemporaryDirectory() as client.app["local_path"]:
        resp = await client.get("/")
        assert resp.status == 200
        text = await resp.text()
        assert text == ""


async def test_fetch_format(aiohttp_client):
    client, public_client = await create_client(aiohttp_client, codebases={'foo'})

    resp = await client.post("/foo/.bzr/smart")
    assert resp.status == 200

    resp = await client.post("/foo/campaign/.bzr/smart")
    assert resp.status == 200

    resp = await client.post("/foo/campaign/main/.bzr/smart")
    assert resp.status == 200

    resp = await client.post("/bar/.bzr/smart")
    assert resp.status == 404

    resp = await client.post("/foo/notcampaign/.bzr/smart")
    assert resp.status == 404
