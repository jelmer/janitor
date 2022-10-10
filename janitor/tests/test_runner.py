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


import aiozipkin
from janitor.runner import (
    create_app,
    is_log_filename,
    committer_env,
)


async def create_client(aiohttp_client):
    endpoint = aiozipkin.create_endpoint("janitor.runner", ipv4='127.0.0.1', port=80)
    tracer = await aiozipkin.create_custom(endpoint)
    return await aiohttp_client(await create_app(None, tracer))


async def test_health(aiohttp_client):
    client = await create_client(aiohttp_client)

    resp = await client.get("/health")
    assert resp.status == 200
    text = await resp.text()
    assert text == "OK"


async def test_ready(aiohttp_client):
    client = await create_client(aiohttp_client)

    resp = await client.get("/ready")
    assert resp.status == 200
    text = await resp.text()
    assert text == "OK"


def test_committer_env():
    assert committer_env("Joe Example <joe@example.com>") == {
        "DEBFULLNAME": "Joe Example",
        "DEBEMAIL": "joe@example.com",
        "COMMITTER": "Joe Example <joe@example.com>",
        "BRZ_EMAIL": "Joe Example <joe@example.com>",
        "GIT_COMMITTER_NAME": "Joe Example",
        "GIT_COMMITTER_EMAIL": "joe@example.com",
        "GIT_AUTHOR_NAME": "Joe Example",
        "GIT_AUTHOR_EMAIL": "joe@example.com",
        "EMAIL": "joe@example.com"}


def test_is_log_filename():
    assert is_log_filename("foo.log")
    assert is_log_filename("foo.log.1")
    assert not is_log_filename("foo.deb")
