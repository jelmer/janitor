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
from datetime import datetime, timedelta
import mockaioredis
from janitor.runner import (
    create_app,
    is_log_filename,
    committer_env,
    QueueProcessor,
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
    assert text == "ok"


async def test_ready(aiohttp_client):
    client = await create_client(aiohttp_client)

    resp = await client.get("/ready")
    assert resp.status == 200
    text = await resp.text()
    assert text == "ok"


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


async def create_queue_processor():
    redis = await mockaioredis.create_redis_pool('redis://localhost')
    return QueueProcessor(None, redis, run_timeout=30)


async def test_watch_dog():
    qp = await create_queue_processor()
    qp.start_watchdog()
    assert qp._watch_dog is not None
    qp.stop_watchdog()
    assert qp._watch_dog is None


async def test_rate_limit_hosts():
    qp = await create_queue_processor()
    assert [x async for x in qp.rate_limited_hosts()] == []

    retry_after = datetime.utcnow() - timedelta(seconds=30)
    await qp.rate_limited("expired.com", retry_after)
    assert [x async for x in qp.rate_limited_hosts()] == []

    retry_after = datetime.utcnow() + timedelta(seconds=30)
    await qp.rate_limited("github.com", retry_after)

    assert [x async for x in qp.rate_limited_hosts()] == [('github.com', retry_after)]


async def test_status_json():
    qp = await create_queue_processor()
    data = await qp.status_json()
    assert data == {'avoid_hosts': [], 'processing': [], 'rate_limit_hosts': {}}
