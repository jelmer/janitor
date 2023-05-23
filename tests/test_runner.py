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

import os
from datetime import datetime, timedelta
from io import BytesIO
from typing import Dict, Tuple

import aiozipkin
from aiohttp import MultipartWriter
from fakeredis.aioredis import FakeRedis

from janitor.config import Config
from janitor.debian import dpkg_vendor
from janitor.logs import LogFileManager
from janitor.runner import (
    ActiveRun,
    Backchannel,
    QueueProcessor,
    committer_env,
    create_app,
    is_log_filename,
    store_change_set,
    store_run,
)
from janitor.vcs import get_vcs_managers


class MemoryLogFileManager(LogFileManager):

    def __init__(self) -> None:
        self.m: Dict[Tuple[str, str], Dict[str, bytes]] = {}

    async def has_log(self, pkg: str, run_id: str, name: str, timeout=None):
        return name in self.m.get((pkg, run_id), {})

    async def get_log(self, pkg: str, run_id: str, name: str, timeout=None):
        try:
            return BytesIO(self.m.get((pkg, run_id), {})[name])
        except KeyError as e:
            raise FileNotFoundError from e

    async def get_ctime(self, pkg: str, run_id: str, name: str):
        if self.has_log(pkg, run_id, name):
            return datetime.utcnow()
        raise FileNotFoundError

    async def import_log(self, pkg, run_id, orig_path, timeout=None, mtime=None):
        with open(orig_path, 'rb') as f:
            self.m.setdefault((pkg, run_id), {})[os.path.basename(orig_path)] = f.read()

    async def delete_log(self, pkg, run_id, name):
        try:
            del self.m.setdefault((pkg, run_id), {})[name]
        except KeyError as e:
            raise FileNotFoundError from e

    async def iter_logs(self):
        for (pkg, run_id), logs in self.m.items():
            for name in logs:
                yield (pkg, run_id, name)


async def create_client(aiohttp_client, queue_processor=None, *, campaigns=None):
    endpoint = aiozipkin.create_endpoint("janitor.runner", ipv4='127.0.0.1', port=80)
    tracer = await aiozipkin.create_custom(endpoint)
    config = Config()
    unstable = config.distribution.add()
    unstable.name = "unstable"
    if campaigns:
        for name in campaigns:
            campaign = config.campaign.add()
            campaign.name = name
            campaign.debian_build.base_distribution = "unstable"
            campaign.default_empty = True
    return await aiohttp_client(await create_app(
        queue_processor, config,
        queue_processor.database if queue_processor else None, tracer))


async def test_status(aiohttp_client, db):
    qp = await create_queue_processor(db)
    client = await create_client(aiohttp_client, qp)
    resp = await client.get("/status")
    assert resp.status == 200
    assert {'avoid_hosts': [], 'processing': [], 'rate_limit_hosts': {}} == await resp.json()


async def test_get_active_runs(aiohttp_client, db):
    qp = await create_queue_processor(db)
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


async def create_queue_processor(db=None, vcs_managers=None):
    redis = FakeRedis()
    return QueueProcessor(
        db, redis, run_timeout=30, logfile_manager=MemoryLogFileManager(),
        public_vcs_managers=vcs_managers)


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
        campaign='test', change_set=None, command='blah',
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


async def test_submit_codebase(aiohttp_client, db):
    qp = await create_queue_processor(db)
    client = await create_client(aiohttp_client, qp)
    resp = await client.post("/codebases", json=[{
        "name": "foo",
        "branch_url": "https://example.com/foo.git"
    }])
    assert resp.status == 200
    assert {} == await resp.json()

    resp = await client.get("/codebases")
    assert resp.status == 200
    assert [{
        "name": "foo",
        "branch_url": "https://example.com/foo.git",
        'url': 'https://example.com/foo.git',
        'branch': None,
        'subpath': None,
        'vcs_type': None,
        'vcs_last_revision': None,
        'value': None,
        'web_url': None,
    }] == await resp.json()


async def test_candidate_invalid_value(aiohttp_client, db, tmp_path):
    vcs = tmp_path / "vcs"
    vcs.mkdir()
    qp = await create_queue_processor(db, vcs_managers=get_vcs_managers(str(vcs)))
    client = await create_client(aiohttp_client, qp, campaigns=['mycampaign'])
    resp = await client.post("/codebases", json=[{
        "name": "foo",
        "branch_url": "https://example.com/foo.git"
    }])
    assert resp.status == 200

    resp = await client.post("/candidates", json=[{
        "campaign": "mycampaign",
        "codebase": "foo",
        "command": "true",
        "value": 0,
    }])
    assert resp.status == 200
    assert (await resp.json())['invalid_value'] == [0]


async def test_submit_candidate(aiohttp_client, db, tmp_path):
    vcs = tmp_path / "vcs"
    vcs.mkdir()
    qp = await create_queue_processor(db, vcs_managers=get_vcs_managers(str(vcs)))
    client = await create_client(aiohttp_client, qp, campaigns=['mycampaign'])
    resp = await client.post("/codebases", json=[{
        "name": "foo",
        "branch_url": "https://example.com/foo.git"
    }])
    assert resp.status == 200
    resp = await client.post("/candidates", json=[{
        "campaign": "mycampaign",
        "codebase": "foo",
        "command": "true",
    }])
    assert resp.status == 200
    [result] = (await resp.json())['success']
    assert result == {
        'bucket': 'default',
        'campaign': 'mycampaign',
        'change_set': None,
        'codebase': 'foo',
        'estimated_duration': 15.0,
        'offset': 35000.0,
        'queue-id': 1,
        'refresh': False,
    }

    resp = await client.post("/active-runs", json={})
    assert resp.status == 201
    assignment = await resp.json()
    assert assignment == {
        'branch': {
            'additional_colocated_branches': None,
            'cached_url': None,
            'default-empty': True,
            'subpath': None,
            'url': 'https://example.com/foo.git',
            'vcs_type': None
        },
        'build': {
            'config': {
                'build-distribution': 'mycampaign',
                'build-extra-repositories': [],
                'build-suffix': '',
                'dep_server_url': None,
                'lintian': {'profile': ''}
            },
            'environment': {
                'DEB_VENDOR': dpkg_vendor(),
                'DISTRIBUTION': 'unstable',
            },
            'target': 'debian',
        },
        'campaign': 'mycampaign',
        'codebase': 'foo',
        'codemod': {'command': 'true', 'environment': {}},
        'command': 'true',
        'description': 'mycampaign on foo',
        'env': {
            'DEB_VENDOR': dpkg_vendor(),
            'DISTRIBUTION': 'unstable',
        },
        'force-build': False,
        'id': assignment['id'],
        'queue_id': 1,
        'resume': None,
        'skip-setup-validation': False,
        'target_repository': {'url': None, 'vcs_type': None},
    }

    ts = datetime.utcnow().isoformat()

    with MultipartWriter("form-data") as mpwriter:
        mpwriter.append_json(
            {"finish_time": ts, "start_time": ts},
            headers=[  # type: ignore
                (
                    "Content-Disposition",
                    'attachment; filename="result.json"; '
                    "filename*=utf-8''result.json",
                )
            ],
        )  # type: ignore

    resp = await client.post(f"/active-runs/{assignment['id']}/finish", data=mpwriter)
    assert resp.status == 201
    ret = await resp.json()
    cs = ret['result']['change_set']
    assert ret == {
        "id": assignment["id"],
        'artifacts': None,
        'filenames': [],
        'logs': [],
        'result': {
            'branches': None,
            'branch_url': None,
            'campaign': 'mycampaign',
            'change_set': cs,
            'code': 'missing-result-code',
            'codebase': 'foo',
            'codemod': None,
            'description': None,
            'duration': 0.0,
            'failure_details': None,
            'failure_stage': None,
            'finish_time': ts,
            'log_id': assignment['id'],
            'logfilenames': [],
            'main_branch_revision': None,
            'remotes': None,
            'resume': None,
            'revision': None,
            'start_time': ts,
            'tags': None,
            'target': {},
            'transient': None,
            'value': None,
        },
    }

    await qp.stop()


async def test_submit_unknown_candidate_codebase(aiohttp_client, db):
    qp = await create_queue_processor(db)
    client = await create_client(aiohttp_client, qp, campaigns=['mycampaign'])
    resp = await client.post("/candidates", json=[{
        "codebase": "foo",
        "command": "true",
        "campaign": "mycampaign",
    }])
    assert resp.status == 200
    assert ('unknown_codebases', ['foo']) in (await resp.json()).items()


async def test_submit_unknown_candidate_publish_policy(aiohttp_client, db):
    qp = await create_queue_processor(db)
    client = await create_client(aiohttp_client, qp, campaigns=['mycampaign'])
    resp = await client.post("/codebases", json=[{
        "name": "foo",
        "branch_url": "https://example.com/foo.git"
    }])
    assert resp.status == 200
    resp = await client.post("/candidates", json=[{
        "codebase": "foo",
        "command": "true",
        "campaign": "mycampaign",
        "publish-policy": "some-policy",
    }])
    assert resp.status == 200
    assert ('unknown_publish_policies', ['some-policy']) in (await resp.json()).items()


async def test_submit_unknown_campaign(aiohttp_client, db):
    qp = await create_queue_processor(db)
    client = await create_client(aiohttp_client, qp)
    resp = await client.post("/codebases", json=[{
        "name": "foo",
        "branch_url": "https://example.com/foo.git"
    }])
    assert resp.status == 200

    resp = await client.post("/candidates", json=[{
        "campaign": "mycampaign",
        "codebase": "foo"
    }])
    assert resp.status == 200
    assert ('unknown_campaigns', ['mycampaign']) in (await resp.json()).items()


def test_serialize_active_run():
    run = ActiveRun(
        worker_name='myworker',
        worker_link='http://example.com/',
        campaign='mycampaign',
        codebase='foo',
        change_set=None,
        command='ls',
        instigated_context='instigated-context',
        estimated_duration=timedelta(seconds=2),
        queue_id=4242,
        log_id='some-log-id',
        backchannel=Backchannel(),
        start_time=datetime.utcnow(),
        vcs_info={
            'vcs_type': 'git',
            'branch_url': 'http://example.com/foo'
        })
    orig_json = run.json()
    run_copy = ActiveRun.from_json(orig_json)
    run_copy_json = run_copy.json()
    run_copy_json['current_duration'] = orig_json['current_duration']
    assert run_copy_json == orig_json
    assert run_copy == run


async def create_dummy_run(conn, campaign="mycampaign", run_id="run-id", codebase="foo"):
    await store_change_set(conn, "run-id", campaign="mycampaign")
    await store_run(
        conn, run_id=run_id,
        codebase=codebase, campaign=campaign,
        vcs_type="git", subpath="",
        start_time=datetime.utcnow(),
        finish_time=datetime.utcnow(),
        command="true",
        result_code="missing-result-code",
        codemod_result={},
        main_branch_revision=b'some-revid',
        revision=b'revid',
        description='Did a thing',
        context=None,
        instigated_context=None,
        logfilenames=[],
        value=1,
        change_set=run_id,
        worker_name=None,
        branch_url='https://example.com/blah')
    return run_id


async def test_tweak_run(aiohttp_client, db, tmp_path):
    vcs = tmp_path / "vcs"
    vcs.mkdir()
    qp = await create_queue_processor(db, vcs_managers=get_vcs_managers(str(vcs)))
    campaign = "mycampaign"
    codebase = "foo"
    client = await create_client(aiohttp_client, qp, campaigns=[campaign])
    resp = await client.post("/codebases", json=[{
        "name": codebase,
        "branch_url": "https://example.com/foo.git"
    }])
    assert resp.status == 200

    async with db.acquire() as conn:
        run_id = await create_dummy_run(conn, campaign=campaign, codebase=codebase)

    resp = await client.get(f"/runs/{run_id}")
    assert resp.status == 200
    assert {'campaign': campaign, 'codebase': codebase, 'publish_status': 'unknown'} == await resp.json()

    resp = await client.post(f"/runs/{run_id}", json={'publish_status': 'approved'})
    assert resp.status == 200
    assert {'campaign': campaign, 'codebase': codebase, 'publish_status': 'approved', 'run_id': run_id} == await resp.json()

    resp = await client.get(f"/runs/{run_id}")
    assert resp.status == 200
    assert {'campaign': campaign, 'codebase': codebase, 'publish_status': 'approved'} == await resp.json()


async def test_tweak_unknown_run(aiohttp_client, db, tmp_path):
    vcs = tmp_path / "vcs"
    vcs.mkdir()
    qp = await create_queue_processor(db, vcs_managers=get_vcs_managers(str(vcs)))
    client = await create_client(aiohttp_client, qp, campaigns=['mycampaign'])

    resp = await client.get("/runs/run-id")
    assert resp.status == 404

    resp = await client.post("/runs/run-id", json={'publish_status': 'approved'})
    assert resp.status == 404


async def test_assignment_with_only_vcs(aiohttp_client, db, tmp_path):
    vcs = tmp_path / "vcs"
    vcs.mkdir()
    qp = await create_queue_processor(db, vcs_managers=get_vcs_managers(str(vcs)))
    client = await create_client(aiohttp_client, qp, campaigns=['mycampaign'])
    resp = await client.post("/codebases", json=[{
        "name": "foo",
        "vcs_type": "hg",
    }])
    assert resp.status == 200
    resp = await client.post("/candidates", json=[{
        "campaign": "mycampaign",
        "codebase": "foo",
        "command": "true",
    }])
    assert resp.status == 200
    [result] = (await resp.json())['success']
    assert result == {
        'bucket': 'default',
        'campaign': 'mycampaign',
        'change_set': None,
        'codebase': 'foo',
        'estimated_duration': 15.0,
        'offset': 35000.0,
        'queue-id': 1,
        'refresh': False,
    }

    resp = await client.post("/active-runs", json={})
    assert resp.status == 201, await resp.json()
    assignment = await resp.json()
    assert assignment == {
        'branch': {
            'additional_colocated_branches': None,
            'cached_url': None,
            'default-empty': True,
            'subpath': None,
            'url': None,
            'vcs_type': "hg",
        },
        'build': {
            'config': {
                'build-distribution': 'mycampaign',
                'build-extra-repositories': [],
                'build-suffix': '',
                'dep_server_url': None,
                'lintian': {'profile': ''}
            },
            'environment': {
                'DEB_VENDOR': dpkg_vendor(),
                'DISTRIBUTION': 'unstable',
            },
            'target': 'debian',
        },
        'campaign': 'mycampaign',
        'codebase': 'foo',
        'codemod': {'command': 'true', 'environment': {}},
        'command': 'true',
        'description': 'mycampaign on foo',
        'env': {
            'DEB_VENDOR': dpkg_vendor(),
            'DISTRIBUTION': 'unstable',
        },
        'force-build': False,
        'id': assignment['id'],
        'queue_id': 1,
        'resume': None,
        'skip-setup-validation': False,
        'target_repository': {'url': None, 'vcs_type': 'hg'},
    }
    await qp.stop()
