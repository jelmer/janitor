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

import asyncio
import os
import tempfile
from threading import Thread

from breezy.controldir import ControlDir

from janitor.bzr_store import create_web_app
from janitor.config import read_string as read_config_string


async def create_client(aiohttp_client, tmp_path="/tmp", codebases=None):
    config = read_config_string("""\
campaign {
    name: "campaign"
}
""")

    if codebases is None:
        codebases = set()

    async def check_codebase(n):
        return n in codebases

    app, public_app = await create_web_app(
        "127.0.0.1", 80, tmp_path, check_codebase, allow_writes=True, config=config
    )
    return (await aiohttp_client(app), await aiohttp_client(public_app))


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

    temp_dir = tempfile.TemporaryDirectory()
    client.app["local_path"] = temp_dir.name
    try:
        resp = await client.get("/")
        assert resp.status == 200
        text = await resp.text()
        assert text == ""
    finally:
        temp_dir.cleanup()


async def test_fetch_format(aiohttp_client):
    client, public_client = await create_client(aiohttp_client, codebases={"foo"})

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


async def test_index(aiohttp_client, tmp_path):
    client, public_client = await create_client(
        aiohttp_client, tmp_path, codebases={"foo"}
    )

    resp = await client.post("/foo/campaign/main/.bzr/smart")
    assert resp.status == 200

    resp = await public_client.get("/bzr/", headers={"Accept": "application/json"})
    assert resp.status == 200
    assert await resp.json() == ["foo"]


async def test_push(aiohttp_server, tmp_path):
    server = None

    done = False

    def serve():
        nonlocal server, done
        loop = asyncio.new_event_loop()
        config = read_config_string("""\
campaign {
    name: "campaign"
}
""")

        codebases = {"foo"}

        async def check_codebase(n):
            return n in codebases

        os.mkdir(tmp_path / "bzr")

        app, public_app = loop.run_until_complete(
            create_web_app(
                "127.0.0.1",
                80,
                tmp_path / "bzr",
                check_codebase,
                allow_writes=True,
                config=config,
            )
        )

        server = loop.run_until_complete(aiohttp_server(public_app))
        loop.run_until_complete(server.start_server())
        while not done:
            loop.run_until_complete(asyncio.sleep(0.01))
        loop.run_until_complete(server.close())

    t = Thread(target=serve)
    t.start()
    try:
        for _i in range(20):
            await asyncio.sleep(0.1)
            if server:
                break
        else:
            raise Exception("server did not start")
        wt = ControlDir.create_standalone_workingtree(str(tmp_path / "wt"))
        (tmp_path / "wt" / "afile").write_text("foo")
        wt.add("afile")
        wt.commit("A change", committer="Joe Example <joe@example.com>")

        cd = ControlDir.open(str(server.make_url("/bzr/foo/")))
        cd.find_repository()

        # Currently broken:
        # url = 'bzr+' + str(server.make_url('/bzr/foo/campaign/'))
        # branch_controldir = ControlDir.create(url, format=RemoteBzrDirFormat())
        # branch = controldir.create_branch(name='')
        # wt.branch.push(branch)
    finally:
        done = True
        t.join()
