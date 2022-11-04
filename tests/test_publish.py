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


from janitor.config import Config
from janitor.publish import (
    create_app,
    find_campaign_by_branch_name,
)

from google.protobuf import text_format  # type: ignore

from fakeredis.aioredis import FakeRedis


async def create_client(aiohttp_client):
    return await aiohttp_client(await create_app(
        vcs_managers={}, db=None,
        redis=FakeRedis(),
        lock_manager=None, config=None,
        differ_url="https://differ/"))


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


def test_find_campaign_by_branch_name():
    config = text_format.Parse("""\
campaign {
 name: "bar"
 branch_name: "fo"
}
""", Config())

    assert find_campaign_by_branch_name(config, "fo") == ("bar", "main")
    assert find_campaign_by_branch_name(config, "bar") == (None, None)
    assert find_campaign_by_branch_name(config, "lala") == (None, None)
