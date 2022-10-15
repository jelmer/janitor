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

from breezy.controldir import ControlDir, format_registry
from breezy.errors import NotBranchError
from breezy.repository import Repository
from dulwich.errors import HangupException, MissingCommitError
from dulwich.objects import valid_hexsha, ZERO_SHA
from dulwich.web import HTTPGitApplication

try:
    from dulwich.web import NO_CACHE_HEADERS
except ImportError:  # dulwich < 0.20.47
    NO_CACHE_HEADERS = [
        ("Expires", "Fri, 01 Jan 1980 00:00:00 GMT"),
        ("Pragma", "no-cache"),
        ("Cache-Control", "no-cache, max-age=0, must-revalidate"),
    ]

from dulwich.protocol import ReceivableProtocol
from dulwich.server import (
    DEFAULT_HANDLERS as DULWICH_SERVICE_HANDLERS,
    DictBackend,
)
from . import (
    state,
)

from .compat import to_thread
from .config import read_config
from .site import is_worker, iter_accept, env as site_env


GIT_BACKEND_CHUNK_SIZE = 4096


async def git_diff_request(request):
    package = request.match_info["package"]
    try:
        old_sha = request.query['old'].encode('utf-8')
        new_sha = request.query['new'].encode('utf-8')
    except KeyError:
        raise web.HTTPBadRequest(text='need both old and new')
    path = request.query.get('path')
    try:
        repo = Repository.open(os.path.join(request.app['local_path'], package))
    except NotBranchError:
        raise web.HTTPServiceUnavailable(
            text="Local VCS repository for %s temporarily inaccessible" %
            package)
    if not valid_hexsha(old_sha) or not valid_hexsha(new_sha):
        raise web.HTTPBadRequest(text='invalid shas specified')

    args = [
        "git",
        "diff",
        old_sha, new_sha
    ]
    if path:
        args.extend(['--', path])

    p = await asyncio.create_subprocess_exec(
        *args,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        stdin=asyncio.subprocess.PIPE,
        cwd=repo.user_transport.local_abspath('.'),
    )

    # TODO(jelmer): Stream this
    try:
        (stdout, stderr) = await asyncio.wait_for(p.communicate(b""), 30.0)
    except asyncio.TimeoutError:
        raise web.HTTPRequestTimeout(text='diff generation timed out')

    if p.returncode == 0:
        return web.Response(body=stdout, content_type="text/x-diff")
    logging.warning('git diff failed: %s', stderr.decode())
    raise web.HTTPInternalServerError(text='git diff failed: %s' % stderr)


async def git_revision_info_request(request):
    package = request.match_info["package"]
    try:
        old_sha = request.query['old'].encode('utf-8')
        new_sha = request.query['new'].encode('utf-8')
    except KeyError:
        raise web.HTTPBadRequest(text='need both old and new')
    try:
        repo = Repository.open(os.path.join(request.app['local_path'], package))
    except NotBranchError:
        raise web.HTTPServiceUnavailable(
            text="Local VCS repository for %s temporarily inaccessible" %
            package)
    if not valid_hexsha(old_sha) or not valid_hexsha(new_sha):
        raise web.HTTPBadRequest(text='invalid shas specified')
    ret = []
    try:
        walker = repo._git.get_walker(
            include=[new_sha],
            exclude=([old_sha] if old_sha != ZERO_SHA else []))
    except MissingCommitError:
        return web.json_response({}, status=404)
    for entry in walker:
        ret.append({
            'commit-id': entry.commit.id.decode('ascii'),
            'revision-id': 'git-v1:' + entry.commit.id.decode('ascii'),
            'link': '/git/%s/commit/%s/' % (package, entry.commit.id.decode('ascii')),
            'message': entry.commit.message.decode('utf-8', 'replace')})
    return web.json_response(ret)


async def _git_open_repo(local_path: str, db, package: str) -> Repository:
    repo_path = os.path.join(local_path, package)
    try:
        repo = Repository.open(repo_path)
    except NotBranchError:
        async with db.acquire() as conn:
            if not await package_exists(conn, package):
                raise web.HTTPNotFound(text='no such package: %s' % package)
        controldir = ControlDir.create(repo_path, format=format_registry.get("git-bare")())
        logging.info(
            "Created missing git repository for %s at %s", package, controldir.user_url
        )
        return controldir.open_repository()
    else:
        return repo


def _git_check_service(service: str, allow_writes: bool = False) -> None:
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

    span = aiozipkin.request_span(request)
    with span.new_child('open-repo'):
        repo = await _git_open_repo(request.app['local_path'], request.app['db'], package)

    from klaus import views, utils, KLAUS_VERSION
    from flask import Flask
    from klaus.repo import FancyRepo

    class Klaus(Flask):
        def __init__(self, package, repo):
            super(Klaus, self).__init__("klaus")
            self.package = package
            self.valid_repos = {package: FancyRepo(repo._transport.local_abspath("."), namespace=None)}

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

    span = aiozipkin.request_span(request)
    with span.new_child('open-repo'):
        repo = await _git_open_repo(request.app['local_path'], request.app['db'], package)

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


async def cgit_backend(request):
    package = request.match_info["package"]
    subpath = request.match_info["subpath"]
    span = aiozipkin.request_span(request)

    allow_writes = request.app['allow_writes']
    if allow_writes is None:
        allow_writes = await is_worker(request.app['db'], request)
    service = request.query.get("service")
    if service is not None:
        _git_check_service(service, allow_writes)

    with span.new_child('open-repo'):
        repo = await _git_open_repo(request.app['local_path'], request.app['db'], package)

    args = ["/usr/bin/git"]
    if allow_writes:
        args.extend(["-c", "http.receivepack=1"])
    args.append("http-backend")
    local_path = repo.user_transport.local_abspath(".")
    full_path = os.path.join(local_path, subpath.lstrip('/'))
    env = {
        "GIT_HTTP_EXPORT_ALL": "true",
        "REQUEST_METHOD": request.method,
        "REMOTE_ADDR": request.remote,
        "CONTENT_TYPE": request.content_type,
        "PATH_TRANSLATED": full_path,
        "QUERY_STRING": request.query_string,
        # REMOTE_USER is not set
    }

    if request.content_type is not None:
        env['CONTENT_TYPE'] = request.content_type

    for key, value in request.headers.items():
        env["HTTP_" + key.replace("-", "_").upper()] = value

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

        if 'Content-Length' in headers:
            content_length = int(headers['Content-Length'])
            return web.Response(
                headers=headers, status=status_code, reason=status_reason,
                body=await p.stdout.read(content_length))
        else:
            response = web.StreamResponse(
                headers=headers,
                status=status_code, reason=status_reason,
            )

            if tuple(request.version) == (1, 1):
                response.enable_chunked_encoding()

            await response.prepare(request)

            chunk = await p.stdout.read(GIT_BACKEND_CHUNK_SIZE)
            while chunk:
                await response.write(chunk)
                chunk = await p.stdout.read(GIT_BACKEND_CHUNK_SIZE)

            await response.write_eof()

            return response

    with span.new_child('git-backend'):
        try:
            unused_stderr, response, unused_stdin = await asyncio.gather(*[
                read_stderr(p.stderr), read_stdout(p.stdout),
                feed_stdin(p.stdin),
            ], return_exceptions=False)
        except asyncio.CancelledError:
            p.terminate()
            await p.wait()
            raise

    return response


async def dulwich_refs(request):
    package = request.match_info["package"]

    allow_writes = request.app['allow_writes']
    if allow_writes is None:
        allow_writes = await is_worker(request.app['db'], request)

    span = aiozipkin.request_span(request)
    with span.new_child('open-repo'):
        repo = await _git_open_repo(request.app['local_path'], request.app['db'], package)
    r = repo._git

    service = request.query.get("service")
    _git_check_service(service, allow_writes)

    headers = {
        "Content-Type": "application/x-%s-advertisement" % service,
    }
    headers.update(NO_CACHE_HEADERS)

    handler_cls = DULWICH_SERVICE_HANDLERS[service.encode("ascii")]

    response = web.StreamResponse(status=200, headers=headers)

    await response.prepare(request)

    out = BytesIO()
    proto = ReceivableProtocol(BytesIO().read, out.write)
    handler = handler_cls(
        DictBackend({".": r}), ["."], proto, stateless_rpc=True, advertise_refs=True
    )
    handler.proto.write_pkt_line(b"# service=" + service.encode("ascii") + b"\n")
    handler.proto.write_pkt_line(None)

    await to_thread(handler.handle)

    await response.write(out.getvalue())

    await response.write_eof()

    return response


async def dulwich_service(request):
    package = request.match_info["package"]
    service = request.match_info["service"]

    allow_writes = request.app['allow_writes']
    if allow_writes is None:
        allow_writes = await is_worker(request.app['db'], request)

    span = aiozipkin.request_span(request)
    with span.new_child('open-repo'):
        repo = await _git_open_repo(request.app['local_path'], request.app['db'], package)

    _git_check_service(service, allow_writes)

    headers = {
        'Content-Type': "application/x-%s-result" % service
    }
    headers.update(NO_CACHE_HEADERS)
    handler_cls = DULWICH_SERVICE_HANDLERS[service.encode("ascii")]

    response = web.StreamResponse(status=200, headers=headers)

    await response.prepare(request)

    inf = BytesIO(await request.read())
    outf = BytesIO()

    def handle():
        r = repo._git
        proto = ReceivableProtocol(inf.read, outf.write)
        handler = handler_cls(DictBackend({".": r}), ["."], proto, stateless_rpc=True)
        try:
            handler.handle()
        except HangupException:
            response.force_close()

    await to_thread(handle)

    await response.write(outf.getvalue())

    await response.write_eof()
    return response


async def package_exists(conn, package):
    return bool(await conn.fetchrow("SELECT 1 FROM package WHERE name = $1", package))


async def handle_repo_list(request):
    span = aiozipkin.request_span(request)
    with span.new_child('list-repositories'):
        names = [entry.name
                 for entry in os.scandir(os.path.join(request.app['local_path']))]
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
            text = await template.render_async(vcs="git", repositories=names)
            return web.Response(text=text, content_type='text/html')
    return web.json_response(names)


async def handle_health(request):
    return web.Response(text='ok')


async def handle_ready(request):
    return web.Response(text='ok')


async def handle_home(request):
    return web.Response(text='')


async def create_web_app(
    listen_addr: str,
    port: int,
    local_path: str,
    db: asyncpg.pool.Pool,
    config,
    dulwich_server: bool = False,
    client_max_size: Optional[int] = None,
):
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(
        middlewares=[trailing_slash_redirect, state.asyncpg_error_middlewares],
        client_max_size=(client_max_size or 0)
    )
    app['local_path'] = local_path
    app['db'] = db
    app['allow_writes'] = True
    public_app = web.Application(
        middlewares=[trailing_slash_redirect, state.asyncpg_error_middlewares],
        client_max_size=(client_max_size or 0)
    )
    public_app['local_path'] = local_path
    public_app['db'] = db
    public_app['allow_writes'] = None
    public_app.middlewares.insert(0, metrics_middleware)
    app.middlewares.insert(0, metrics_middleware)
    app.router.add_get("/metrics", metrics, name="metrics")
    if dulwich_server:
        app.router.add_post(
            "/{package}/{service:git-receive-pack|git-upload-pack}",
            dulwich_service,
            name='dulwich-service'
        )
        public_app.router.add_post(
            "/git/{package}/{service:git-receive-pack|git-upload-pack}",
            dulwich_service,
            name='dulwich-service-public'
        )
        app.router.add_get(
            "/{package}/info/refs",
            dulwich_refs, name='dulwich-refs')
        public_app.router.add_get(
            "/git/{package}/info/refs",
            dulwich_refs, name='dulwich-refs-public')
    else:
        for (method, regex), fn in HTTPGitApplication.services.items():
            app.router.add_route(
                method, "/{package}{subpath:" + regex.pattern + "}",
                cgit_backend,
            )
            public_app.router.add_route(
                method, "/git/{package}{subpath:" + regex.pattern + "}",
                cgit_backend,
            )


    public_app.router.add_get("/", handle_home, name='home')
    public_app.router.add_get("/git/", handle_repo_list, name='public-repo-list')
    app.router.add_get("/", handle_repo_list, name='repo-list')
    app.router.add_get("/health", handle_health, name='health')
    app.router.add_get("/ready", handle_ready, name='ready')
    app.router.add_get("/{package}/diff", git_diff_request, name='git-diff')
    app.router.add_get("/{package}/revision-info", git_revision_info_request, name='git-revision-info')
    public_app.router.add_get("/git/{package}/{path_info:.*}", handle_klaus, name='klaus')
    app.router.add_post("/{package}/remotes/{remote}", handle_set_git_remote, name='git-remote')
    endpoint = aiozipkin.create_endpoint("janitor.git_store", ipv4=listen_addr, port=port)
    if config.zipkin_address:
        tracer = await aiozipkin.create(config.zipkin_address, endpoint, sample_rate=1.0)
    else:
        tracer = await aiozipkin.create_custom(endpoint)
    aiozipkin.setup(app, tracer)
    aiozipkin.setup(public_app, tracer)
    return app, public_app


async def main(argv=None):
    import argparse

    parser = argparse.ArgumentParser(prog="janitor.git_store")
    parser.add_argument(
        "--prometheus", type=str, help="Prometheus push gateway to export to."
    )
    parser.add_argument(
        "--listen-address", type=str, help="Listen address", default="localhost"
    )
    parser.add_argument("--port", type=int, help="Listen port", default=9923)
    parser.add_argument(
        "--public-port", type=int, help="Public listen port", default=9924)
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

    if not os.path.exists(args.vcs_path):
        raise RuntimeError('vcs path %s does not exist' % args.vcs_path)

    db = await state.create_pool(config.database_location)
    app, public_app = await create_web_app(
        args.listen_address,
        args.port,
        args.vcs_path,
        db,
        config,
        dulwich_server=args.dulwich_server,
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
