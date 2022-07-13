#!/usr/bin/python3
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

"""Manage VCS repositories."""

import aiozipkin
import asyncpg.pool
import asyncio
from io import BytesIO
import logging
import os
from typing import Optional
import warnings

from aiohttp import web
from aiohttp.web_middlewares import normalize_path_middleware
from aiohttp_openmetrics import metrics_middleware, metrics
from http.client import parse_headers  # type: ignore

from breezy import urlutils
from breezy.branch import Branch
from breezy.controldir import ControlDir, format_registry
from breezy.errors import NotBranchError
from breezy.bzr.smart import medium
from breezy.repository import Repository
from breezy.transport import get_transport_from_url
from . import (
    state,
)

from .compat import to_thread
from .config import read_config, get_campaign_config
from .site import is_worker, iter_accept, env as site_env


async def bzr_diff_helper(repo, old_revid, new_revid, path=None):
    if path:
        raise NotImplementedError
    args = [
        sys.executable,
        '-m',
        'breezy',
        "diff",
        '-rrevid:%s..revid:%s' % (
            old_revid.decode(),
            new_revid.decode(),
        ),
        urlutils.join(repo.user_url, path or '')
    ]

    p = await asyncio.create_subprocess_exec(
        *args,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        stdin=asyncio.subprocess.PIPE,
    )

    # TODO(jelmer): Stream this
    try:
        (stdout, stderr) = await asyncio.wait_for(p.communicate(b""), 30.0)
    except asyncio.TimeoutError:
        raise web.HTTPRequestTimeout(text='diff generation timed out')

    if p.returncode != 3:
        return web.Response(body=stdout, content_type="text/x-diff")
    logging.warning('bzr diff failed: %s', stderr.decode())
    raise web.HTTPInternalServerError(text='bzr diff failed: %s' % stderr)


async def bzr_diff_request(request):
    package = request.match_info["package"]
    old_revid = request.query.get('old')
    path = request.query.get('path')
    if old_revid is not None:
        old_revid = old_revid.encode('utf-8')
    new_revid = request.query.get('new')
    if new_revid is not None:
        new_revid = new_revid.encode('utf-8')
    try:
        repo = Repository.open(os.path.join(request.app.local_path, "bzr", package))
    except NotBranchError:
        repo = None
    if repo is None:
        raise web.HTTPServiceUnavailable(
            text="Local VCS repository for %s temporarily inaccessible" %
            package)
    return await bzr_diff_helper(repo, old_revid, new_revid, path)


async def bzr_revision_info_request(request):
    package = request.match_info["package"]
    old_revid = request.query.get('old')
    if old_revid is not None:
        old_revid = old_revid.encode('utf-8')
    new_revid = request.query.get('new')
    if new_revid is not None:
        new_revid = new_revid.encode('utf-8')
    try:
        repo = Repository.open(os.path.join(request.app.local_path, "bzr", package))
    except NotBranchError:
        repo = None
    if repo is None:
        raise web.HTTPServiceUnavailable(
            text="Local VCS repository for %s temporarily inaccessible" %
            package)
    ret = []
    with repo.lock_read():
        graph = repo.get_graph()
        for rev in repo.iter_revisions(graph.iter_lefthand_ancestry(new_revid, [old_revid])):
            ret.append({
                'revision-id': rev.revision_id.decode('utf-8'),
                'link': None,
                'message': rev.description})
    return web.json_response(ret)


async def handle_set_bzr_remote(request):
    package = request.match_info["package"]
    remote = request.match_info["remote"]
    post = await request.post()

    try:
        local_branch = Branch.open(os.path.join(request.app.local_path, "bzr", package, remote))
    except NotBranchError:
        raise web.HTTPNotFound()
    local_branch.set_parent(post["url"])

    # TODO(jelmer): Run 'bzr pull'?

    return web.Response()


async def package_exists(conn, package):
    return bool(await conn.fetchrow("SELECT 1 FROM package WHERE name = $1", package))


async def _bzr_open_repo(local_path, db, package):
    async with db.acquire() as conn:
        if not await package_exists(conn, package):
            raise web.HTTPNotFound(text='no such package: %s' % package)
    repo_path = os.path.join(local_path, "bzr", package)
    try:
        repo = Repository.open(repo_path)
    except NotBranchError:
        controldir = ControlDir.create(repo_path)
        repo = controldir.create_repository(shared=True)
    return repo


async def bzr_backend(request):
    package = request.match_info["package"]
    branch_name = request.match_info.get("branch")
    repo = await _bzr_open_repo(request.app.local_path, request.app.db, package)
    if branch_name:
        try:
            get_campaign_config(request.app.config, branch_name)
        except KeyError:
            raise web.HTTPNotFound(text='no such suite: %s' % branch_name)
        transport = repo.user_transport.clone(branch_name)
    else:
        transport = repo.user_transport
    transport.ensure_base()
    allow_writes = request.app.allow_writes
    if allow_writes is None:
        allow_writes = await is_worker(request.app.db, request)
    if allow_writes:
        backing_transport = transport
    else:
        backing_transport = get_transport_from_url("readonly+" + transport.base)
    out_buffer = BytesIO()
    request_data_bytes = await request.read()

    protocol_factory, unused_bytes = medium._get_protocol_factory_for_bytes(
        request_data_bytes
    )

    smart_protocol_request = protocol_factory(
        transport, out_buffer.write, ".", backing_transport
    )
    smart_protocol_request.accept_bytes(unused_bytes)
    if smart_protocol_request.next_read_size() != 0:
        # The request appears to be incomplete, or perhaps it's just a
        # newer version we don't understand.  Regardless, all we can do
        # is return an error response in the format of our version of the
        # protocol.
        response_data = b"error\x01incomplete request\n"
    else:
        response_data = out_buffer.getvalue()

    response = web.StreamResponse(status=200)
    response.content_type = "application/octet-stream"

    await response.prepare(request)

    await response.write(response_data)

    await response.write_eof()

    return response


async def handle_repo_list(request):
    span = aiozipkin.request_span(request)
    with span.new_child('list-repositories'):
        names = [entry.name
                 for entry in os.scandir(os.path.join(request.app.local_path))]
        names.sort()
    for accept in iter_accept(request):
        if accept in ('application/json', ):
            return web.json_response(names)
        elif accept in ('text/plain', ):
            return web.Response(
                text=''.join([line + '\n' for line in names]),
                content_type='text/plain')
        elif accept in ('text/html', ):
            template = site_env.get_template('repo-list.html')
            text = await template.render_async(vcs="bzr", repositories=names)
            return web.Response(text=text, content_type='text/html')
    return web.json_response(names)


async def handle_health(request):
    return web.Response(text='ok')


async def create_web_app(
    listen_addr: str,
    port: int,
    local_path: str,
    db: asyncpg.pool.Pool,
    config,
    client_max_size: Optional[int] = None,
):
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(
        middlewares=[trailing_slash_redirect], client_max_size=(client_max_size or 0)
    )
    app.local_path = local_path
    app.db = db
    app.allow_writes = True
    app.config = config
    public_app = web.Application(
        middlewares=[trailing_slash_redirect], client_max_size=(client_max_size or 0)
    )
    public_app.local_path = local_path
    public_app.db = db
    public_app.allow_writes = None
    public_app.config = config
    public_app.middlewares.insert(0, metrics_middleware)
    app.middlewares.insert(0, metrics_middleware)
    app.router.add_get("/metrics", metrics, name="metrics")
    public_app.router.add_get("/", handle_repo_list, name='public-repo-list')
    app.router.add_get("/", handle_repo_list, name='repo-list')
    app.router.add_get("/health", handle_health, name='health')
    app.router.add_get("/bzr/{package}/diff", bzr_diff_request, name='bzr-diff')
    app.router.add_get("/bzr/{package}/revision-info", bzr_revision_info_request, name='bzr-revision-info')
    public_app.router.add_post("/bzr/{package}/{branch}/.bzr/smart", bzr_backend, name='bzr-branch-public')
    public_app.router.add_post("/bzr/{package}/.bzr/smart", bzr_backend, name='bzr-repo-public')
    app.router.add_post("/bzr/{package}/.bzr/smart", bzr_backend, name='bzr-repo')
    app.router.add_post("/bzr/{package}/{branch}/.bzr/smart", bzr_backend, name='bzr-branch')
    app.router.add_post("/bzr/{package}/remotes/{remote}", handle_set_bzr_remote, name='bzr-remote')
    endpoint = aiozipkin.create_endpoint("janitor.vcs_store_bzr", ipv4=listen_addr, port=port)
    if config.zipkin_address:
        tracer = await aiozipkin.create(config.zipkin_address, endpoint, sample_rate=1.0)
    else:
        tracer = await aiozipkin.create_custom(endpoint)
    aiozipkin.setup(app, tracer)
    aiozipkin.setup(public_app, tracer)
    return app, public_app


async def main(argv=None):
    import argparse

    parser = argparse.ArgumentParser(prog="janitor.vcs_store_bzr")
    parser.add_argument(
        "--prometheus", type=str, help="Prometheus push gateway to export to."
    )
    parser.add_argument(
        "--listen-address", type=str, help="Listen address", default="localhost"
    )
    parser.add_argument("--port", type=int, help="Listen port", default=9929)
    parser.add_argument(
        "--public-port", type=int, help="Public listen port", default=9930)
    parser.add_argument(
        "--config",
        type=str,
        default="janitor.conf",
        help="Path to load configuration from.",
    )
    parser.add_argument(
        "--client-max-size",
        type=int,
        default=1024 ** 3,
        help="Maximum client body size (0 for no limit)",
    )
    parser.add_argument("--debug", action="store_true", help="Show debug info")
    parser.add_argument("--vcs-path", default=None, type=str, help="Path to local vcs storage")
    parser.add_argument("--gcp-logging", action="store_true")

    args = parser.parse_args()

    if args.gcp_logging:
        import google.cloud.logging
        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    elif args.debug:
        logging.basicConfig(level=logging.DEBUG)
    else:
        logging.basicConfig(level=logging.INFO)

    if args.debug:
        loop = asyncio.get_event_loop()
        loop.set_debug(True)
        loop.slow_callback_duration = 0.001
        warnings.simplefilter('always', ResourceWarning)

    with open(args.config, "r") as f:
        config = read_config(f)

    state.DEFAULT_URL = config.database_location

    db = await state.create_pool(config.database_location)
    app, public_app = await create_web_app(
        args.listen_address,
        args.port,
        args.vcs_path,
        db,
        config,
        client_max_size=args.client_max_size,
    )

    runner = web.AppRunner(app)
    public_runner = web.AppRunner(public_app)
    await runner.setup()
    await public_runner.setup()
    site = web.TCPSite(runner, args.listen_address, port=args.port)
    await site.start()
    logging.info("Listening on %s:%s", args.listen_address, args.port)
    site = web.TCPSite(public_runner, args.listen_address, port=args.public_port)
    await site.start()
    logging.info("Listening on %s:%s", args.listen_address, args.public_port)
    while True:
        await asyncio.sleep(3600)


if __name__ == "__main__":
    import sys

    sys.exit(asyncio.run(main(sys.argv)))
