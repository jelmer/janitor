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

from janitor.worker import (
    bundle_results,
    create_app,
    _convert_codemod_script_failed,
    WorkerFailure,
)
from aiohttp.multipart import MultipartReader
from silver_platter.apply import ScriptFailed

from io import BytesIO

import asyncio
import os
import shutil
import tempfile
import unittest


class AsyncBytesIO:
    def __init__(self):
        self._io = BytesIO()

    def seek(self, pos):
        self._io.seek(pos)

    async def write(self, chunk):
        self._io.write(chunk)

    async def readline(self):
        return self._io.readline()

    async def read(self, size=None):
        return self._io.read(size)


class BundleResultsTests(unittest.TestCase):
    def setUp(self):
        super(BundleResultsTests, self).setUp()
        self.test_dir = tempfile.mkdtemp()
        old_dir = os.getcwd()
        os.chdir(self.test_dir)
        self.addCleanup(os.chdir, old_dir)
        self.addCleanup(shutil.rmtree, self.test_dir)

    def test_simple(self):
        loop = asyncio.get_event_loop()
        with open("a", "w") as f:
            f.write("some data\n")
        with bundle_results({"result_code": "success"}, self.test_dir) as writer:
            self.assertEqual(["Content-Type"], list(writer.headers.keys()))
            b = AsyncBytesIO()
            loop.run_until_complete(writer.write(b))
            b.seek(0)
            reader = MultipartReader(writer.headers, b)
            part = loop.run_until_complete(reader.next())
            self.assertEqual(
                part.headers,
                {
                    "Content-Disposition": 'attachment; filename="result.json"; '
                    "filename*=utf-8''result.json",
                    "Content-Length": "26",
                    "Content-Type": "application/json",
                },
            )
            self.assertEqual("result.json", part.filename)
            self.assertEqual(
                b'{"result_code": "success"}', bytes(loop.run_until_complete(part.read())))
            part = loop.run_until_complete(reader.next())
            self.assertEqual(
                part.headers,
                {
                    "Content-Disposition": 'attachment; filename="a"; '
                    "filename*=utf-8''a",
                    "Content-Length": "10",
                    "Content-Type": "application/octet-stream",
                },
            )
            self.assertEqual("a", part.filename)
            self.assertEqual(b"some data\n", bytes(loop.run_until_complete(part.read())))
            self.assertTrue(part.at_eof())


async def create_client(aiohttp_client):
    app = await create_app()
    client = await aiohttp_client(app)
    return app, client


async def test_health(aiohttp_client):
    app, client = await create_client(aiohttp_client)

    resp = await client.get("/health")
    assert resp.status == 200
    text = await resp.text()
    assert text == "ok"


async def test_index(aiohttp_client):
    app, client = await create_client(aiohttp_client)

    resp = await client.get("/")
    assert resp.status == 200
    text = await resp.text()
    assert "No current assignment." in text


async def test_log_id(aiohttp_client):
    app, client = await create_client(aiohttp_client)

    resp = await client.get("/log-id")
    assert resp.status == 200
    text = await resp.text()
    assert text == ""

    app['workitem']['assignment'] = {'id': 'my-id'}

    resp = await client.get("/log-id")
    assert resp.status == 200
    text = await resp.text()
    assert text == "my-id"


async def test_assignment(aiohttp_client):
    app, client = await create_client(aiohttp_client)

    resp = await client.get("/assignment")
    assert resp.status == 200
    data = await resp.json()
    assert data is None

    app['workitem']['assignment'] = {'id': 'my-id'}

    resp = await client.get("/assignment")
    assert resp.status == 200
    data = await resp.json()
    assert data == app['workitem']['assignment']


def test_convert_codemod_script_failed():
    assert _convert_codemod_script_failed(ScriptFailed("foobar", 127)) == WorkerFailure(
        'codemod-command-not-found',
        'Command foobar not found',
        stage=("codemod", ))
    assert _convert_codemod_script_failed(ScriptFailed("foobar", 137)) == WorkerFailure(
        'out-of-memory', 'Ran out of memory running command', stage=('codemod', ))
    assert _convert_codemod_script_failed(ScriptFailed("foobar", 1)) == WorkerFailure(
        'codemod-command-failed', 'Script foobar failed to run with code 1',
        stage=('codemod', ))
