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

import os
import tempfile
from io import BytesIO

import breezy.bzr  # noqa: F401
import breezy.git  # noqa: F401
import pytest
from breezy.config import GlobalStack
from breezy.controldir import ControlDir, format_registry

from janitor.worker import (
    Metadata,
    _WorkerFailure,
    create_app,
    run_worker,
)


@pytest.fixture
def brz_identity(tmp_path):
    os.mkdir(tmp_path / "brz_home")
    os.environ['BRZ_HOME'] = str(tmp_path / "brz_home")
    identity = 'Joe Example <joe@example.com>'
    os.environ['BRZ_EMAIL'] = identity
    return identity


def test_brz_identity(brz_identity):
    assert brz_identity == 'Joe Example <joe@example.com>'
    c = GlobalStack()
    assert c.get('email') == brz_identity


class AsyncBytesIO:
    def __init__(self) -> None:
        self._io = BytesIO()

    def seek(self, pos):
        self._io.seek(pos)

    async def write(self, chunk):
        self._io.write(chunk)

    async def readline(self):
        return self._io.readline()

    async def read(self, size=None):
        return self._io.read(size)


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


async def test_intermediate_result(aiohttp_client):
    app, client = await create_client(aiohttp_client)

    resp = await client.get("/intermediate-result")
    assert resp.status == 200
    data = await resp.json()
    assert data is None

    app['workitem']['metadata'] = {'id': 'my-id'}

    resp = await client.get("/intermediate-result")
    assert resp.status == 200
    data = await resp.json()
    assert data == app['workitem']['metadata']


async def test_artifact_index(aiohttp_client):
    app, client = await create_client(aiohttp_client)

    resp = await client.get("/artifacts/")
    assert resp.status == 404

    app['workitem']['directory'] = '/nonexistent'
    resp = await client.get("/artifacts/")
    assert resp.status == 404

    with tempfile.TemporaryDirectory() as td:
        app['workitem']['directory'] = td

        resp = await client.get("/artifacts/")
        assert resp.status == 200
        text = await resp.text()
        assert "<body>" in text


@pytest.mark.parametrize("vcs_type", ['git', 'bzr'])
def test_run_worker_existing(tmp_path, vcs_type, brz_identity):
    wt = ControlDir.create_standalone_workingtree(
        str(tmp_path / "main"),
        format=format_registry.make_controldir(vcs_type))
    (tmp_path / "main" / "Makefile").write_text("""\

all:

test:

check:

""")
    wt.add("Makefile")
    old_revid = wt.commit("Add makefile")
    os.mkdir(tmp_path / "target")
    output_dir = tmp_path / "output"
    os.mkdir(output_dir)
    metadata = Metadata()
    run_worker(
        codebase='mycodebase',
        campaign='mycampaign',
        run_id='run-id',
        command=['sh', '-c', 'echo foo > bar'],
        metadata=metadata,
        main_branch_url=wt.controldir.user_url,
        build_config={},
        target="generic",
        output_directory=output_dir,
        target_repo_url=str(tmp_path / "target"),
        vendor="foo",
        env={})
    assert {e.name for e in os.scandir(output_dir)} == {'codemod.log', 'worker.log', 'build.log', 'test.log'}
    if vcs_type == 'git':
        cd = ControlDir.open(str(tmp_path / "target"))
        b = cd.open_branch(name='mycampaign/main')
        branch_name = 'master'
    elif vcs_type == 'bzr':
        b = ControlDir.open(str(tmp_path / "target" / "mycampaign")).open_branch()
        branch_name = ''
    assert metadata.json() == {
        'branch_url': wt.branch.user_url,
        'branches': [['main', branch_name, old_revid.decode('utf-8'), b.last_revision().decode('utf-8')]],
        'codebase': 'mycodebase',
        'codemod': None,
        'command': ['sh', '-c', 'echo foo > bar'],
        'description': '',
        'main_branch_revision': old_revid.decode('utf-8'),
        'refreshed': False,
        'remotes': {'origin': {'url': wt.branch.user_url}},
        'revision': b.last_revision().decode('utf-8'),
        'subpath': '',
        'tags': [],
        'target': {'details': {}, 'name': 'generic'},
        'target_branch_url': None,
        'value': None,
        'vcs_type': vcs_type
    }


@pytest.mark.parametrize("vcs_type", ['git', 'bzr'])
def test_run_worker_new(tmp_path, vcs_type, brz_identity):
    os.mkdir(tmp_path / "target")
    output_dir = tmp_path / "output"
    os.mkdir(output_dir)
    metadata = Metadata()
    run_worker(
        codebase='mycodebase',
        campaign='mycampaign',
        run_id='run-id',
        command=['sh', '-c', 'echo all check test: > Makefile'],
        metadata=metadata,
        main_branch_url=None,
        build_config={},
        target="generic",
        output_directory=output_dir,
        target_repo_url=str(tmp_path / "target"),
        vendor="foo",
        vcs_type=vcs_type,
        env={})
    assert {e.name for e in os.scandir(output_dir)} == {'codemod.log', 'worker.log', 'build.log', 'test.log', 'mycodebase'}
    if vcs_type == 'git':
        cd = ControlDir.open(str(tmp_path / "target"))
        b = cd.open_branch(name='mycampaign/main')
        tags = b.tags.get_tag_dict()
        assert tags == {'run/run-id/main': b.last_revision()}
    elif vcs_type == 'bzr':
        b = ControlDir.open(str(tmp_path / "target" / "mycampaign")).open_branch()
        tags = b.tags.get_tag_dict()
        assert tags == {'run-id': b.last_revision()}
    assert metadata.json() == {
        'branch_url': None,
        'branches': [['main', '', 'null:', b.last_revision().decode('utf-8')]],
        'codebase': 'mycodebase',
        'codemod': None,
        'command': ['sh', '-c', 'echo all check test: > Makefile'],
        'description': '',
        'main_branch_revision': 'null:',
        'refreshed': False,
        'remotes': {},
        'revision': b.last_revision().decode('utf-8'),
        'subpath': '',
        'tags': [],
        'target': {'details': {}, 'name': 'generic'},
        'target_branch_url': None,
        'value': None,
        'vcs_type': vcs_type
    }


@pytest.mark.parametrize("vcs_type", ['git', 'bzr'])
def test_run_worker_build_failure(tmp_path, vcs_type, brz_identity):
    os.mkdir(tmp_path / "target")
    output_dir = tmp_path / "output"
    os.mkdir(output_dir)
    metadata = Metadata()
    with pytest.raises(_WorkerFailure, match='.*no-build-tools.*'):
        run_worker(
            codebase='mycodebase',
            campaign='mycampaign',
            run_id='run-id',
            command=['sh', '-c', 'echo foo > bar'],
            metadata=metadata,
            main_branch_url=None,
            build_config={},
            target="generic",
            output_directory=output_dir,
            target_repo_url=str(tmp_path / "target"),
            vendor="foo",
            vcs_type=vcs_type,
            env={})
    assert {e.name for e in os.scandir(output_dir)} == {'codemod.log', 'worker.log', 'mycodebase'}
    if vcs_type == 'git':
        repo = ControlDir.open(str(tmp_path / "target")).open_repository()
        assert list(repo._git.get_refs().keys()) == [b'refs/tags/run/run-id/main']  # type: ignore
        run_id_revid = repo.lookup_foreign_revision_id(repo._git.get_refs()[b'refs/tags/run/run-id/main'])  # type: ignore
    elif vcs_type == 'bzr':
        b = ControlDir.open(str(tmp_path / "target" / "mycampaign")).open_branch()
        tags = b.tags.get_tag_dict()
        assert list(tags.keys()) == ['run-id']
        run_id_revid = tags['run-id']
    assert metadata.json() == {
        'branch_url': None,
        'branches': [['main', '', 'null:', run_id_revid.decode('utf-8')]],
        'codebase': 'mycodebase',
        'codemod': None,
        'command': ['sh', '-c', 'echo foo > bar'],
        'description': '',
        'main_branch_revision': 'null:',
        'refreshed': False,
        'remotes': {},
        'revision': run_id_revid.decode('utf-8'),
        'subpath': '',
        'tags': [],
        'target': {'details': None, 'name': 'generic'},
        'target_branch_url': None,
        'value': None,
        'vcs_type': vcs_type
    }
