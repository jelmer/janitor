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

import sys

import mock
from fakeredis.aioredis import FakeRedis
from google.protobuf import text_format  # type: ignore

from janitor.config import Config
from janitor.publish import (PublishWorker, create_app,
                             find_campaign_by_branch_name)
from janitor.publish_one import _drop_env
from janitor.vcs import VcsManager


async def create_client(aiohttp_client):
    return await aiohttp_client(await create_app(
        vcs_managers={}, db=None,
        redis=FakeRedis(),
        config=None))


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


class DummyVcsManager(VcsManager):

    def get_branch_url(self, pkg, name):
        return 'file://foo'


async def test_publish_worker():
    with mock.patch('janitor.publish.run_worker_process', return_value=(0, {})) as e:
        pw = PublishWorker()
        await pw.publish_one(
            campaign='test-campaign', codebase='pkg', command='blah --foo',
            codemod_result={}, target_branch_url='https://example.com/',
            mode='attempt-push', role='main', revision=b'main-revid',
            log_id='some-id', unchanged_id='unchanged-id',
            derived_branch_name='branch-name',
            rate_limit_bucket='jelmer@jelmer.uk',
            vcs_manager=DummyVcsManager(), extra_context={'package': 'pkg'})
        e.assert_called_with(
            [sys.executable, '-m', 'janitor.publish_one'], {
                'dry-run': False,
                'campaign': 'test-campaign',
                'extra_context': {
                    'package': 'pkg',
                },
                'command': 'blah --foo',
                'codemod_result': {},
                'target_branch_url': 'https://example.com',
                'source_branch_url': 'file://foo',
                'existing_mp_url': None,
                'derived_branch_name': 'branch-name',
                'mode': 'attempt-push',
                'role': 'main',
                'log_id': 'some-id',
                'unchanged_id': 'unchanged-id',
                'require-binary-diff': False,
                'allow_create_proposal': False,
                'external_url': None,
                'differ_url': None,
                'revision': 'main-revid',
                'reviewers': None,
                'commit_message_template': None,
                'title_template': None,
                'tags': {}
            })


def test_drop_env():
    args = ['PATH=foo', 'BAR=foo', 'ls', 'bar']
    _drop_env(args)
    assert args == ['ls', 'bar']
    _drop_env(args)
    assert args == ['ls', 'bar']
