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
import logging
import os
from typing import Optional

from aiohttp import web
from aiohttp.web_middlewares import normalize_path_middleware
from http.client import parse_headers  # type: ignore

from breezy.controldir import ControlDir, format_registry
from breezy.errors import NotBranchError
from breezy.bzr.smart import medium
from breezy.transport import get_transport_from_url
from dulwich.web import HTTPGitApplication
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
from .site import is_worker, iter_accept, env as site_env
from .vcs import (
    VcsManager,
    LocalVcsManager,
)


GIT_BACKEND_CHUNK_SIZE = 4096
GIT_BACKEND_TIMEOUT = 60.0 * 60.0


async def diff_request(request):
    run_id = request.match_info["run_id"]
    role = request.match_info["role"]
    async with request.app.db.acquire() as conn:
        run = await state.get_run(conn, run_id)
        if not run:
            raise web.HTTPNotFound(text="No such run: %r" % run_id)
    try:
        repo = request.app.vcs_manager.get_repository(run.package)
    except NotBranchError:
        repo = None
    if repo is None:
        raise web.HTTPServiceUnavailable(
            text="Local VCS repository for %s temporarily inaccessible" %
            run.package)
    for actual_role, _, base_revision, revision in run.result_branches:
        if role == actual_role:
            old_revid = base_revision
            new_revid = revision
            break
    else:
        raise web.HTTPNotFound(text="No branch with role %s" % role)

    args = [
        sys.executable,
        "-m",
        "breezy",
        "diff",
        "-rrevid:%s..%s" % (old_revid.decode(), new_revid.decode()),
        repo.user_url,
    ]

    p = await asyncio.create_subprocess_exec(
        *args,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        stdin=asyncio.subprocess.PIPE
    )

    # TODO(jelmer): Stream this
    try:
        (stdout, stderr) = await asyncio.wait_for(p.communicate(b""), 30.0)
    except asyncio.TimeoutError:
        raise web.HTTPRequestTimeout(text='diff generation timed out')

    return web.Response(body=stdout, content_type="text/x-diff")


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
        logging.info(
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


async def git_backend(request):
    package = request.match_info["package"]
    subpath = request.match_info["subpath"]

    allow_writes = await is_worker(request.app.db, request)
    service = request.query.get("service")
    if service is not None:
        _git_check_service(service, allow_writes)

    repo = await _git_open_repo(request.app.vcs_manager, request.app.db, package)

    args = ["/usr/bin/git"]
    if allow_writes:
        args.extend(["-c", "http.receivepack=1"])
    args.append("http-backend")
    local_path = repo.user_transport.local_abspath(".")
    full_path = os.path.join(local_path, subpath)
    env = {
        "GIT_HTTP_EXPORT_ALL": "true",
        "REQUEST_METHOD": request.method,
        "REMOTE_ADDR": request.remote,
        "CONTENT_TYPE": request.content_type,
        "PATH_TRANSLATED": full_path,
        "QUERY_STRING": request.query_string,
        # REMOTE_USER is not set
    }

    if request.content_length is not None:
        env['CONTENT_LENGTH'] = str(request.content_length)

    if request.content_type is not None:
        env['CONTENT_TYPE'] = request.content_type

    for key, value in request.headers.items():
        env["HTTP_" + key.replace("-", "_").upper()] = value

    for name in ["HTTP_CONTENT_ENCODING", "HTTP_CONTENT_LENGTH"]:
        try:
            del env[name]
        except KeyError:
            pass

    p = await asyncio.create_subprocess_exec(
        *args,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        env=env,
        stdin=asyncio.subprocess.PIPE,
    )

    async def feed_stdin(stream):
        async for chunk in request.content.iter_any():
            stream.write(chunk)
            await stream.drain()
        stream.close()
        await stream.wait_closed()

    async def read_stderr(stream):
        line = await stream.readline()
        while line:
            logging.warning("git: %s", line.decode().rstrip('\n'))
            line = await stream.readline()

    async def read_stdout(stream):
        b = BytesIO()
        line = await stream.readline()
        while line != b'\r\n':
            b.write(line)
            line = await stream.readline()
        b.seek(0)
        headers = parse_headers(b)
        status = headers.get("Status")
        if status:
            del headers["Status"]
            (status_code, status_reason) = status.split(" ", 1)
            status_code = int(status_code)
            status_reason = status_reason
        else:
            status_code = 200
            status_reason = "OK"

        response = web.StreamResponse(
            headers=headers,
            status=status_code, reason=status_reason
        )

        await response.prepare(request)
        response.enable_chunked_encoding()

        chunk = await p.stdout.read(GIT_BACKEND_CHUNK_SIZE)
        while chunk:
            await response.write(chunk)
            chunk = await p.stdout.read(GIT_BACKEND_CHUNK_SIZE)

        await response.write_eof()

        return response

    unused_stderr, response, unused_stdin = await asyncio.wait_for(asyncio.gather(*[
        read_stderr(p.stderr), read_stdout(p.stdout),
        feed_stdin(p.stdin)
        ], return_exceptions=True), GIT_BACKEND_TIMEOUT)

    return response


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

    await asyncio.to_thread(handler.handle)

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

    await asyncio.to_thread(handle)

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


async def handle_repo_list(request):
    vcs = request.match_info["vcs"]
    names = list(request.app.vcs_manager.list_repositories(vcs))
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
            text = await template.render_async(vcs=vcs, repositories=names)
            return web.Response(text=text, content_type='text/html')
    return web.json_response(names)


def run_web_server(
    listen_addr: str,
    port: int,
    vcs_manager: VcsManager,
    db: state.Database,
    dulwich_server: bool = False,
    client_max_size: Optional[int] = None,
):
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(
        middlewares=[trailing_slash_redirect], client_max_size=client_max_size
    )
    app.vcs_manager = vcs_manager
    app.db = db
    setup_metrics(app)
    app.router.add_get("/diff/{run_id}/{role}", diff_request)
    if dulwich_server:
        app.router.add_post(
            "/git/{package}/{service:git-receive-pack|git-upload-pack}", dulwich_service
        )
        app.router.add_get("/git/{package}/info/refs", dulwich_refs)
    else:
        for (method, regex), fn in HTTPGitApplication.services.items():
            app.router.add_route(
                method, "/git/{package}{subpath:" + regex.pattern + "}", git_backend
            )

    app.router.add_get("/{vcs:git|bzr}/", handle_repo_list)
    app.router.add_get("/git/{package}/{path_info:.*}", handle_klaus)
    app.router.add_post("/bzr/{package}/.bzr/smart", bzr_backend)
    app.router.add_post("/bzr/{package}/{branch}/.bzr/smart", bzr_backend)
    app.router.add_get("/vcs-type/{package}", get_vcs_type)
    app.router.add_post("/remotes/git/{package}/{remote}", handle_set_git_remote)
    app.router.add_post("/remotes/bzr/{package}/{remote}", handle_set_bzr_remote)
    logging.info("Listening on %s:%s", listen_addr, port)
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
        "--dulwich-server",
        action="store_true",
        help="Use dulwich server implementation.",
    )
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

    args = parser.parse_args()

    if args.debug:
        logging.basicConfig(level=logging.DEBUG)
    else:
        logging.basicConfig(level=logging.INFO)

    with open(args.config, "r") as f:
        config = read_config(f)

    state.DEFAULT_URL = config.database_location

    vcs_manager = LocalVcsManager(config.vcs_location)
    db = state.Database(config.database_location)
    run_web_server(
        args.listen_address,
        args.port,
        vcs_manager,
        db,
        dulwich_server=args.dulwich_server,
        client_max_size=args.client_max_size,
    )


if __name__ == "__main__":
    import sys

    sys.exit(main(sys.argv))
