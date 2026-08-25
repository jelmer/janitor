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

from janitor.config import read_string as read_config_string
from janitor.site.simple import create_app


def create_config():
    return read_config_string("""
campaign {
  name: "lintian-fixes"
}
""")


async def test_create_app():
    await create_app(config=create_config())


async def test_codebase_query_redirect(aiohttp_client):
    client = await aiohttp_client(await create_app(config=create_config()))
    resp = await client.get(
        "/lintian-fixes/c", params={"codebase": "foo"}, allow_redirects=False
    )
    assert resp.status == 302
    assert resp.headers["Location"] == "/lintian-fixes/c/foo/"


async def test_codebase_query_redirect_legacy_package_param(aiohttp_client):
    client = await aiohttp_client(await create_app(config=create_config()))
    resp = await client.get(
        "/lintian-fixes/c", params={"package": "foo"}, allow_redirects=False
    )
    assert resp.status == 302
    assert resp.headers["Location"] == "/lintian-fixes/c/foo/"


async def test_codebase_query_redirect_prefers_codebase_over_package(aiohttp_client):
    client = await aiohttp_client(await create_app(config=create_config()))
    resp = await client.get(
        "/lintian-fixes/c",
        params={"codebase": "new", "package": "old"},
        allow_redirects=False,
    )
    assert resp.status == 302
    assert resp.headers["Location"] == "/lintian-fixes/c/new/"


async def test_codebase_query_redirect_missing_param_returns_404(aiohttp_client):
    client = await aiohttp_client(await create_app(config=create_config()))
    resp = await client.get("/lintian-fixes/c", allow_redirects=False)
    assert resp.status == 404


async def test_codebase_query_redirect_empty_param_returns_404(aiohttp_client):
    client = await aiohttp_client(await create_app(config=create_config()))
    resp = await client.get(
        "/lintian-fixes/c", params={"codebase": ""}, allow_redirects=False
    )
    assert resp.status == 404
