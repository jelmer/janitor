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

import asyncio
from io import BytesIO
from aiohttp import web
from aiohttp.web_middlewares import normalize_path_middleware

from breezy.controldir import ControlDir, format_registry
from breezy.bzr.smart import medium
from breezy.transport import get_transport_from_url
from dulwich.protocol import ReceivableProtocol
from dulwich.server import (
    DEFAULT_HANDLERS as DULWICH_SERVICE_HANDLERS,
    DictBackend,
)
from . import (
    state,
)

from .config import read_config
from .prometheus import setup_metrics
from .site import is_worker
from .trace import note
from .vcs import (
    VcsManager,
    LocalVcsManager,
    get_run_diff,
)


async def diff_request(request):
    run_id = request.match_info["run_id"]
    role = request.match_info["role"]
    async with request.app.db.acquire() as conn:
        run = await state.get_run(conn, run_id)
        if not run:
            raise web.HTTPNotFound(text="No such run: %r" % run_id)
    diff = get_run_diff(request.app.vcs_manager, run, role)
    return web.Response(body=diff, content_type="text/x-diff")


async def _git_open_repo(vcs_manager, db, package):
    repo = vcs_manager.get_repository(package, "git")

    if repo is None:
        async with db.acquire() as conn:
            if not await state.package_exists(conn, package):
                raise web.HTTPNotFound()
        controldir = ControlDir.create(
            vcs_manager.get_repository_url(package, "git"),
            format=format_registry.get("git-bare")(),
        )
        note(
            "Created missing git repository for %s at %s", package, controldir.user_url
        )
        return controldir.open_repository()
    else:
        return repo


def _git_check_service(service: str, allow_writes: bool = False):
    if service == "git-upload-pack":
        return

    if service == "git-receive-pack":
        if not allow_writes:
            raise web.HTTPUnauthorized(
                text="git-receive-pack requires login",
                headers={"WWW-Authenticate": 'Basic Realm="Debian Janitor"'},
            )
        return

    raise web.HTTPForbidden(text="Unsupported service %s" % service)


async def handle_klaus(request):
    package = request.match_info["package"]

    repo = await _git_open_repo(request.app.vcs_manager, request.app.db, package)

    from klaus import views, utils, KLAUS_VERSION
    from flask import Flask
    from klaus.repo import FancyRepo

    class Klaus(Flask):
        def __init__(self, package, repo):
            super(Klaus, self).__init__("klaus")
            self.package = package
            self.valid_repos = {package: FancyRepo(repo._transport.local_abspath("."))}

        def should_use_ctags(self, git_repo, git_commit):
            return False

        def create_jinja_environment(self):
            """Called by Flask.__init__"""
            env = super(Klaus, self).create_jinja_environment()
            for func in [
                "force_unicode",
                "timesince",
                "shorten_sha1",
                "shorten_message",
                "extract_author_name",
                "formattimestamp",
            ]:
                env.filters[func] = getattr(utils, func)

            env.globals["KLAUS_VERSION"] = KLAUS_VERSION
            env.globals["USE_SMARTHTTP"] = False
            env.globals["SITE_NAME"] = "Package list"
            return env

    app = Klaus(package, repo)

    for endpoint, rule in [
        ("blob", "/blob/"),
        ("blob", "/blob/<rev>/<path:path>"),
        ("blame", "/blame/"),
        ("blame", "/blame/<rev>/<path:path>"),
        ("raw", "/raw/<path:path>/"),
        ("raw", "/raw/<rev>/<path:path>"),
        ("submodule", "/submodule/<rev>/"),
        ("submodule", "/submodule/<rev>/<path:path>"),
        ("commit", "/commit/<path:rev>/"),
        ("patch", "/commit/<path:rev>.diff"),
        ("patch", "/commit/<path:rev>.patch"),
        ("index", "/"),
        ("index", "/<path:rev>"),
        ("history", "/tree/<rev>/"),
        ("history", "/tree/<rev>/<path:path>"),
        ("download", "/tarball/<path:rev>/"),
        ("repo_list", "/.."),
    ]:
        app.add_url_rule(
            rule, view_func=getattr(views, endpoint), defaults={"repo": package}
        )

    from aiohttp_wsgi import WSGIHandler

    wsgi_handler = WSGIHandler(app)

    return await wsgi_handler(request)


async def handle_set_git_remote(request):
    package = request.match_info["package"]
    remote = request.match_info["remote"]

    repo = await _git_open_repo(request.app.vcs_manager, request.app.db, package)

    post = await request.post()
    r = repo._git
    c = r.get_config()
    section = ("remote", remote)
    c.set(section, "url", post["url"])
    c.set(section, "fetch", "+refs/heads/*:refs/remotes/%s/*" % remote)
    b = BytesIO()
    c.write_to_file(b)
    r._controltransport.put_bytes("config", b.getvalue())

    # TODO(jelmer): Run 'git fetch $remote'?

    return web.Response()


async def handle_set_bzr_remote(request):
    package = request.match_info["package"]
    remote = request.match_info["remote"]
    post = await request.post()

    local_branch = request.app.vcs_manager.get_branch(package, remote)

    local_branch.set_parent(post["url"])

    # TODO(jelmer): Run 'bzr pull'?

    return web.Response()


async def dulwich_refs(request):
    package = request.match_info["package"]

    allow_writes = await is_worker(request.app.db, request)

    repo = await _git_open_repo(request.app.vcs_manager, request.app.db, package)
    r = repo._git

    service = request.query.get("service")
    _git_check_service(service, allow_writes)

    headers = {
        "Expires": "Fri, 01 Jan 1980 00:00:00 GMT",
        "Pragma": "no-cache",
        "Cache-Control": "no-cache, max-age=0, must-revalidate",
    }

    handler_cls = DULWICH_SERVICE_HANDLERS[service.encode("ascii")]

    response = web.StreamResponse(status=200, headers=headers)
    response.content_type = "application/x-%s-advertisement" % service

    await response.prepare(request)

    out = BytesIO()
    proto = ReceivableProtocol(BytesIO().read, out.write)
    handler = handler_cls(
        DictBackend({".": r}), ["."], proto, stateless_rpc=True, advertise_refs=True
    )
    handler.proto.write_pkt_line(b"# service=" + service.encode("ascii") + b"\n")
    handler.proto.write_pkt_line(None)
    loop = asyncio.get_event_loop()
    await loop.run_in_executor(None, handler.handle)

    await response.write(out.getvalue())

    await response.write_eof()

    return response


async def dulwich_service(request):
    package = request.match_info["package"]
    service = request.match_info["service"]

    allow_writes = await is_worker(request.app.db, request)

    repo = await _git_open_repo(request.app.vcs_manager, request.app.db, package)

    _git_check_service(service, allow_writes)

    headers = {
        "Expires": "Fri, 01 Jan 1980 00:00:00 GMT",
        "Pragma": "no-cache",
        "Cache-Control": "no-cache, max-age=0, must-revalidate",
    }
    handler_cls = DULWICH_SERVICE_HANDLERS[service.encode("ascii")]

    response = web.StreamResponse(status=200, headers=headers)
    response.content_type = "application/x-%s-result" % service

    await response.prepare(request)

    inf = BytesIO(await request.read())
    outf = BytesIO()

    def handle():
        r = repo._git
        proto = ReceivableProtocol(inf.read, outf.write)
        handler = handler_cls(DictBackend({".": r}), ["."], proto, stateless_rpc=True)
        handler.handle()

    loop = asyncio.get_event_loop()
    await loop.run_in_executor(None, handle)

    await response.write(outf.getvalue())

    await response.write_eof()
    return response


async def bzr_backend(request):
    vcs_manager = request.app.vcs_manager
    package = request.match_info["package"]
    branch = request.match_info.get("branch")
    repo = vcs_manager.get_repository(package, "bzr")
    if await is_worker(request.app.db, request):
        if repo is None:
            controldir = ControlDir.create(
                vcs_manager.get_repository_url(package, "bzr")
            )
            repo = controldir.create_repository(shared=True)
        backing_transport = repo.user_transport
    else:
        if repo is None:
            raise web.HTTPNotFound()
        backing_transport = get_transport_from_url("readonly+" + repo.user_url)
    transport = backing_transport.clone(branch)
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
    # TODO(jelmer): Use StreamResponse
    return web.Response(
        status=200, body=response_data, content_type="application/octet-stream"
    )


async def get_vcs_type(request):
    package = request.match_info["package"]
    vcs_type = request.app.vcs_manager.get_vcs_type(package)
    if vcs_type is None:
        raise web.HTTPNotFound()
    return web.Response(body=vcs_type.encode("utf-8"))


def run_web_server(
    listen_addr: str, port: int, vcs_manager: VcsManager, db: state.Database
):
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[trailing_slash_redirect])
    app.vcs_manager = vcs_manager
    app.db = db
    setup_metrics(app)
    app.router.add_get("/diff/{run_id}/{role}", diff_request)
    app.router.add_post(
        "/git/{package}/{service:git-receive-pack|git-upload-pack}", dulwich_service
    )
    app.router.add_get("/git/{package}/info/refs", dulwich_refs)
    app.router.add_get("/git/{package}/{path_info:.*}", handle_klaus)
    app.router.add_post("/bzr/{package}/.bzr/smart", bzr_backend)
    app.router.add_post("/bzr/{package}/{branch}/.bzr/smart", bzr_backend)
    app.router.add_get("/vcs-type/{package}", get_vcs_type)
    app.router.add_post("/remotes/git/{package}/{remote}", handle_set_git_remote)
    app.router.add_post("/remotes/bzr/{package}/{remote}", handle_set_bzr_remote)
    note("Listening on %s:%s", listen_addr, port)
    web.run_app(app, host=listen_addr, port=port)


def main(argv=None):
    import argparse

    parser = argparse.ArgumentParser(prog="janitor.vcs_store")
    parser.add_argument(
        "--prometheus", type=str, help="Prometheus push gateway to export to."
    )
    parser.add_argument(
        "--listen-address", type=str, help="Listen address", default="localhost"
    )
    parser.add_argument("--port", type=int, help="Listen port", default=9923)
    parser.add_argument(
        "--config",
        type=str,
        default="janitor.conf",
        help="Path to load configuration from.",
    )

    args = parser.parse_args()

    with open(args.config, "r") as f:
        config = read_config(f)

    state.DEFAULT_URL = config.database_location

    vcs_manager = LocalVcsManager(config.vcs_location)
    db = state.Database(config.database_location)
    run_web_server(args.listen_address, args.port, vcs_manager, db)


if __name__ == "__main__":
    import sys

    sys.exit(main(sys.argv))
