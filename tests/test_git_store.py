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


from dulwich.repo import Repo

from janitor import config_pb2
from janitor.git_store import create_web_app

try:
    from dulwich.test_utils import build_commit_graph  # type: ignore
except ImportError:
    from dulwich.tests.utils import build_commit_graph  # type: ignore


async def create_client(aiohttp_client, path, dulwich_server=False):
    config = config_pb2.Config()
    app, public_app = await create_web_app(
        '127.0.0.1',
        80,
        path,
        None,
        config,
        dulwich_server=dulwich_server,
    )
    return (
        await aiohttp_client(app),
        await aiohttp_client(public_app))


async def test_health(aiohttp_client):
    client, public_client = await create_client(aiohttp_client, '/tmp')

    resp = await client.get("/health")
    assert resp.status == 200
    text = await resp.text()
    assert text == "ok"


async def test_ready(aiohttp_client):
    client, public_client = await create_client(aiohttp_client, '/tmp')

    resp = await client.get("/ready")
    assert resp.status == 200
    text = await resp.text()
    assert text == "ok"


async def test_diff_nonexistent(aiohttp_client, tmp_path):
    client, public_client = await create_client(aiohttp_client, tmp_path)

    resp = await client.get("/codebase/diff?old=oldrev&new=newrev")
    assert resp.status == 503
    text = await resp.text()
    assert text == "Local VCS repository for codebase temporarily inaccessible"


async def test_diff(aiohttp_client, tmp_path):
    client, public_client = await create_client(aiohttp_client, tmp_path)

    r = Repo.init_bare(str(tmp_path / "codebase"), mkdir=True)

    c1, c2 = build_commit_graph(r.object_store, [[1], [2, 1]])

    resp = await client.get(f"/codebase/diff?old={c1.id.decode()}&new={c2.id.decode()}")
    assert resp.status == 200, await resp.text()
    text = await resp.text()
    assert text == ""
