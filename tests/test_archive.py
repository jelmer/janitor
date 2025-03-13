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


import hashlib
import os
from tempfile import TemporaryDirectory

from debian.deb822 import Release

from janitor.config import Config
from janitor.debian.archive import HashedFileWriter, create_app


async def create_client(aiohttp_client, config=None):
    if config is None:
        config = Config()
    return await aiohttp_client(
        await create_app(None, config, "/tmp", None, gpg_context=None)
    )


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
    assert text == ""


def test_hash_file_writer():
    with TemporaryDirectory() as td:
        r = Release()
        with HashedFileWriter(r, td, "foo/bar") as w:
            w.write(b"chunk1")
            w.write(b"chunk2")
            w.done()
        md5hex = hashlib.md5(b"chunk1chunk2").hexdigest()
        with open(os.path.join(td, "foo", "by-hash", "MD5Sum", md5hex), "rb") as f:
            assert f.read() == b"chunk1chunk2"
        with open(os.path.join(td, "foo", "bar"), "rb") as f:
            assert f.read() == b"chunk1chunk2"
        assert r["MD5Sum"] == [{"md5sum": md5hex, "name": "foo/bar", "size": 12}]
