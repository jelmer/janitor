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
from fakeredis.aioredis import FakeRedis
from janitor.queue import QueueItem
from janitor.runner import (
    create_app,
    is_log_filename,
    committer_env,
    QueueProcessor,
    ActiveRun,
    Backchannel,
    queue_item_env,
)


async def create_client(aiohttp_client, queue_processor=None):
    endpoint = aiozipkin.create_endpoint("janitor.runner", ipv4='127.0.0.1', port=80)
    tracer = await aiozipkin.create_custom(endpoint)
    return await aiohttp_client(await create_app(queue_processor, None, None, tracer))


async def test_status(aiohttp_client):
    qp = await create_queue_processor()
    client = await create_client(aiohttp_client, qp)
    resp = await client.get("/status")
    assert resp.status == 200
    assert {'avoid_hosts': [], 'processing': [], 'rate_limit_hosts': {}} == await resp.json()


async def test_get_active_runs(aiohttp_client):
    qp = await create_queue_processor()
    client = await create_client(aiohttp_client, qp)
    resp = await client.get("/active-runs")
    assert resp.status == 200
    assert [] == await resp.json()



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
    assert committer_env("") == {}
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
    redis = FakeRedis()
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


async def test_register_run():
    qp = await create_queue_processor()
    assert await qp.active_run_count() == 0
    active_run = ActiveRun(
        campaign='test', package='pkg', change_set=None, command='blah',
        queue_id=12, log_id='some-id', start_time=datetime.utcnow(),
        codebase='test-1.1',
        vcs_info={}, backchannel=Backchannel(), worker_name='tester',
        instigated_context=None, estimated_duration=timedelta(seconds=10))
    await qp.register_run(active_run)
    assert await qp.active_run_count() == 1
    assert await qp.redis.hkeys('active-runs') == [b'some-id']
    assert await qp.redis.hkeys('assigned-queue-items') == [b'12']
    assert await qp.redis.hkeys('last-keepalive') == [b'some-id']

    assert await qp.get_run('nonexistent-id') is None
    assert (await qp.get_run('some-id')).queue_id == 12
    await qp.unclaim_run('unknown-id')
    await qp.unclaim_run('some-id')
    assert await qp.redis.hkeys('active-runs') == []
    assert await qp.redis.hkeys('assigned-queue-items') == []
    assert await qp.redis.hkeys('last-keepalive') == []
    assert await qp.active_run_count() == 0


def test_queue_item_env():
    item = QueueItem(id='some-id', package='package', context={}, command='ls', estimated_duration=timedelta(seconds=30), campaign='campaign', refresh=False, requestor='somebody', change_set=None, codebase='codebase')
    assert queue_item_env(item) == {'PACKAGE': 'package'}
