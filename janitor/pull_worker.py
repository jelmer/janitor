#!/usr/bin/python3
# Copyright (C) 2020 Jelmer Vernooij <jelmer@jelmer.uk>
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

import argparse
import asyncio
from contextlib import contextmanager, ExitStack
from datetime import datetime
import errno
from functools import partial
from http.client import IncompleteRead
from io import BytesIO
import json
import logging
import os
import shlex
import socket
import subprocess
import sys
from tempfile import TemporaryDirectory
from threading import Thread
import traceback
from typing import Any, Optional, List, Dict
from urllib.parse import urljoin

import aiohttp
from aiohttp import ClientSession, MultipartWriter, BasicAuth, ClientTimeout, ClientResponseError, ClientConnectorError, web
import yarl

from jinja2 import Template

from prometheus_client import REGISTRY, push_to_gateway

from breezy import urlutils
from breezy.branch import Branch
from breezy.config import (
    credential_store_registry,
    GlobalStack,
    PlainTextCredentialStore,
)
from breezy.errors import (
    NotBranchError,
    InvalidHttpResponse,
    UnexpectedHttpStatus,
)
from breezy.git.remote import RemoteGitError
from breezy.controldir import ControlDir
from breezy.transport import Transport

from silver_platter.proposal import enable_tag_pushing

from janitor.prometheus import setup_metrics
from janitor.vcs import (
    LocalVcsManager,
    RemoteVcsManager,
    MirrorFailure,
    import_branches,
)
from janitor.worker import (
    WorkerFailure,
    process_package,
)


DEFAULT_UPLOAD_TIMEOUT = ClientTimeout(30 * 60)


class ResultUploadFailure(Exception):
    def __init__(self, reason: str) -> None:
        self.reason = reason


async def abort_run(session: ClientSession, base_url: str, run_id: str) -> None:
    abort_url = urljoin(base_url, "active-runs/%s/abort" % run_id)
    async with session.post(abort_url) as resp:
        if resp.status not in (201, 200):
            raise Exception(
                "Unable to abort run: %r: %d" % (await resp.text(), resp.status)
            )


@contextmanager
def bundle_results(metadata: Any, directory: str):
    with ExitStack() as es:
        with MultipartWriter("form-data") as mpwriter:
            mpwriter.append_json(
                metadata,
                headers=[
                    (
                        "Content-Disposition",
                        'attachment; filename="result.json"; '
                        "filename*=utf-8''result.json",
                    )
                ],
            )  # type: ignore
            for entry in os.scandir(directory):
                if entry.is_file():
                    f = open(entry.path, "rb")
                    es.enter_context(f)
                    mpwriter.append(
                        BytesIO(f.read()),
                        headers=[
                            (
                                "Content-Disposition",
                                'attachment; filename="%s"; '
                                "filename*=utf-8''%s" % (entry.name, entry.name),
                            )
                        ],
                    )  # type: ignore
        yield mpwriter


async def upload_results(
    session: ClientSession,
    base_url: str,
    run_id: str,
    metadata: Any,
    output_directory: str,
) -> Any:
    with bundle_results(metadata, output_directory) as mpwriter:
        finish_url = urljoin(base_url, "active-runs/%s/finish" % run_id)
        async with session.post(
            finish_url, data=mpwriter, timeout=DEFAULT_UPLOAD_TIMEOUT
        ) as resp:
            if resp.status == 404:
                resp_json = await resp.json()
                raise ResultUploadFailure(resp_json["reason"])
            if resp.status not in (201, 200):
                raise ResultUploadFailure(
                    "Unable to submit result: %r: %d" % (await resp.text(), resp.status)
                )
            return await resp.json()


@contextmanager
def copy_output(output_log: str):
    old_stdout = os.dup(sys.stdout.fileno())
    old_stderr = os.dup(sys.stderr.fileno())
    p = subprocess.Popen(["tee", output_log], stdin=subprocess.PIPE)
    os.dup2(p.stdin.fileno(), sys.stdout.fileno())  # type: ignore
    os.dup2(p.stdin.fileno(), sys.stderr.fileno())  # type: ignore
    try:
        yield
    finally:
        sys.stdout.flush()
        sys.stderr.flush()
        os.dup2(old_stdout, sys.stdout.fileno())
        os.dup2(old_stderr, sys.stderr.fileno())
        p.stdin.close()  # type: ignore


def push_branch(
    source_branch: Branch,
    url: str,
    vcs_type: str,
    overwrite=False,
    stop_revision=None,
    tag_selector=None,
    possible_transports: Optional[List[Transport]] = None,
) -> None:
    url, params = urlutils.split_segment_parameters(url)
    branch_name = params.get("branch")
    if branch_name is not None:
        branch_name = urlutils.unquote(branch_name)
    if vcs_type is None:
        vcs_type = source_branch.controldir.cloning_metadir()
    try:
        target = ControlDir.open(url, possible_transports=possible_transports)
    except NotBranchError:
        target = ControlDir.create(
            url, format=vcs_type, possible_transports=possible_transports
        )

    target.push_branch(
        source_branch, revision_id=stop_revision, overwrite=overwrite, name=branch_name,
        tag_selector=tag_selector
    )


def run_worker(
    branch_url,
    run_id,
    subpath,
    vcs_type,
    env,
    command,
    output_directory,
    metadata,
    vcs_manager,
    vendor,
    suite,
    target,
    resume_branch_url=None,
    cached_branch_url=None,
    resume_subworker_result=None,
    resume_branches=None,
    possible_transports=None,
):
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    with copy_output(os.path.join(output_directory, "worker.log")):
        try:
            with process_package(
                branch_url,
                subpath,
                env,
                command,
                output_directory,
                metadata=metadata,
                target=target,
                resume_branch_url=resume_branch_url,
                cached_branch_url=cached_branch_url,
                resume_subworker_result=resume_subworker_result,
                extra_resume_branches=[
                    (role, name) for (role, name, base, revision) in resume_branches
                ]
                if resume_branches
                else None,
                possible_transports=possible_transports,
            ) as (ws, result):
                enable_tag_pushing(ws.local_tree.branch)
                logging.info("Pushing result branch to %r", vcs_manager)

                try:
                    import_branches(
                        vcs_manager,
                        ws.local_tree.branch,
                        env["PACKAGE"],
                        suite,
                        run_id,
                        result.branches,
                        result.tags,
                    )
                except UnexpectedHttpStatus as e:
                    if e.code == 502:
                        raise WorkerFailure(
                            "result-push-bad-gateway",
                            "Failed to push result branch: %s" % e,
                        )
                    raise WorkerFailure(
                        "result-push-failed", "Failed to push result branch: %s" % e
                    )
                except (InvalidHttpResponse, IncompleteRead, MirrorFailure) as e:
                    raise WorkerFailure(
                        "result-push-failed", "Failed to push result branch: %s" % e
                    )
                except RemoteGitError as e:
                    if str(e) == 'missing necessary objects':
                        raise WorkerFailure(
                            'result-push-git-missing-necessary-objects', str(e))
                    else:
                        raise

                logging.info("Pushing packaging branch cache to %s", cached_branch_url)

                def tag_selector(tag_name):
                    return tag_name.startswith(vendor + '/') or tag_name.startswith('upstream/')

                push_branch(
                    ws.local_tree.branch,
                    cached_branch_url,
                    vcs_type=vcs_type.lower() if vcs_type is not None else None,
                    possible_transports=possible_transports,
                    stop_revision=ws.main_branch.last_revision(),
                    tag_selector=tag_selector,
                    overwrite=True,
                )
                logging.info("All done.")
                return result
        except WorkerFailure:
            raise
        except BaseException:
            traceback.print_exc()
            raise


async def get_assignment(
    session: ClientSession,
    base_url: str,
    node_name: str,
    jenkins_metadata: Optional[Dict[str, str]],
) -> Any:
    assign_url = urljoin(base_url, "active-runs")
    build_arch = subprocess.check_output(
        ["dpkg-architecture", "-qDEB_BUILD_ARCH"]
    ).decode().strip()
    json: Any = {"node": node_name, "archs": [build_arch]}
    if jenkins_metadata:
        json["jenkins"] = jenkins_metadata
    logging.debug("Sending assignment request: %r", json)
    async with session.post(assign_url, json=json) as resp:
        if resp.status != 201:
            raise ValueError("Unable to get assignment: %r" % await resp.text())
        return await resp.json()


class WatchdogPetter(object):

    def __init__(self, base_url, auth, run_id, queue_id=None):
        self.base_url = base_url
        self.auth = auth
        self.run_id = run_id
        self._task = None
        self._log_cached = []
        self.ws = None
        self.loop = asyncio.new_event_loop()
        self._thread = Thread(target=self._run, daemon=True)
        self._thread.start()
        self._tasks = []
        self._log_dir_tasks = {}
        self._last_communication = datetime.utcnow()
        self.kill = None
        self.queue_id = queue_id

    def _run(self):
        asyncio.set_event_loop(self.loop)
        self.loop.run_forever()

    def start(self):
        for task in [self._connection(), self._send_keepalives()]:
            self._tasks.append(task)
            asyncio.run_coroutine_threadsafe(task, self.loop)

    async def _send_keepalives(self):
        try:
            while True:
                await asyncio.sleep(10)
                if (datetime.utcnow() - self._last_communication).total_seconds() > 60:
                    if not await self.send_keepalive():
                        logging.warning('failed to send keepalive')
        except BaseException:
            logging.exception('sending keepalives')
            raise

    async def _connection(self):
        ws_url = urljoin(
            self.base_url, "ws/active-runs/%s/progress" % self.run_id)
        params = {}
        if self.queue_id is not None:
            params['queue_id'] = self.queue_id
        async with ClientSession(auth=self.auth) as session:
            while True:
                try:
                    self.ws = await session.ws_connect(ws_url, params=params)
                except (ClientResponseError, ClientConnectorError) as e:
                    self.ws = None
                    logging.warning("progress ws: Unable to connect: %s" % e)
                    await asyncio.sleep(5)
                    continue

                for (fn, data) in self._log_cached:
                    await self.send_log_fragment(fn, data)
                self._log_cached = []

                while True:
                    msg = await self.ws.receive()

                    if msg.type == aiohttp.WSMsgType.text:
                        logging.warning("Unknown websocket message: %r", msg.data)
                    elif msg.type == aiohttp.WSMsgType.BINARY:
                        if msg.data == b'kill':
                            logging.info('Received kill over websocket, exiting..')
                            if self.kill:
                                self.kill()
                        else:
                            logging.warning("Unknown websocket message: %r", msg.data)
                    elif msg.type == aiohttp.WSMsgType.closed:
                        break
                    elif msg.type == aiohttp.WSMsgType.error:
                        logging.warning("Error on websocket: %s", self.ws.exception())
                        break
                    elif msg.type == aiohttp.WSMsgType.close:
                        logging.info('Request to close websocket.')
                        await self.ws.close()
                        break
                    else:
                        logging.warning("Ignoring ws message type %r", msg.type)
                self.ws = None
                await asyncio.sleep(5)

    async def send_keepalive(self):
        if self.ws is not None:
            logging.debug('Sending keepalive')
            await self.ws.send_bytes(b"keepalive")
            return True
        else:
            logging.debug('Not sending keepalive; websocket is dead')
            return False
        self._last_communication = datetime.utcnow()

    async def send_log_fragment(self, filename, data):
        if self.ws is None:
            self._log_cached.append((filename, data))
        else:
            await self.ws.send_bytes(
                b"\0".join([b"log", filename.encode("utf-8"), data])
            )
        self._last_communication = datetime.utcnow()

    def track_log_directory(self, directory):
        task = self._forward_logs(directory)
        self._log_dir_tasks[directory] = task
        asyncio.run_coroutine_threadsafe(task, self.loop)

    async def _forward_logs(self, directory):
        fs = {}
        try:
            while True:
                try:
                    for entry in os.scandir(directory):
                        if (entry.name not in fs and
                                entry.name.endswith('.log')):
                            fs[entry.name] = open(entry.path, 'rb')
                except FileNotFoundError:
                    pass  # Uhm, okay
                for name, f in fs.items():
                    data = f.read()
                    await self.send_log_fragment(name, data)
                await asyncio.sleep(60)
        except BaseException:
            logging.exception('log directory forwarding')
            raise


INDEX_TEMPLATE = Template("""\
<html>
<head><title>Job</title></head>
<body>

<h1>Build Details</h1>

<ul>
<li><b>Command: </b>{{ assignment['command'] }}</li>
<li><b>Start Time: </b>: {{ metadata['start_time'] }}
<li><b>Current duration: </b>: {{ datetime.utcnow() - metadata['start_time'] }}
</ul>

<h1>Logs</h1>
<ul>
{% for name in names %}
  <li><a href="/logs/{{ name }}">{{ name }}</a></li>
{% endfor %}
</ul>

</body>
</html>
""")


async def handle_index(request):
    return web.Response(text=INDEX_TEMPLATE.render(
        assignment=request.app['assignment'],
        metadata=request.app['metadata'],
        datetime=datetime),
        content_type='text/html', status=200)


async def handle_assignment(request):
    return web.json_response(request.app['assignment'])


LOG_INDEX_TEMPLATE = Template("""\
<html>
<head><title>Log Index</title><head>
<body>
<h1>Logs</h1>
<ul>
{% for name in names %}
  <li><a href="/logs/{{ name }}">{{ name }}</a></li>
{% endfor %}
</ul>
</body>
</html>
""")


async def handle_log_index(request):
    if 'directory' not in request.app:
        return web.Response("Log directory not created yet", status=404)
    names = [entry.name for entry in os.scandir(request.app['directory'])
             if entry.name.endswith('.log')]
    return web.Response(
        text=LOG_INDEX_TEMPLATE.render(names=names), content_type='text/html',
        status=200)


async def handle_log(request):
    return web.FileResponse(os.path.join(request.app['directory'], request.match_info['filename']))


async def handle_health(request):
    return web.Response(text='ok', status=200)


async def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="janitor-pull-worker",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument(
        "--base-url",
        type=str,
        help="Base URL",
        default="https://janitor.debian.net/api/",
    )
    parser.add_argument(
        "--output-directory", type=str, help="Output directory", default="."
    )
    parser.add_argument(
        "--credentials", help="Path to credentials file (JSON).", type=str, default=None
    )
    parser.add_argument(
        "--vcs-location", help="Override VCS location.", type=str)
    parser.add_argument(
        "--debug",
        help="Print out API communication",
        action="store_true",
        default=False,
    )
    parser.add_argument(
        "--prometheus", type=str, help="Prometheus push gateway to export to."
    )
    parser.add_argument(
        '--port', type=int, default=0, help="Port to use for diagnostics web server")

    # Unused, here for backwards compatibility.
    parser.add_argument('--build-command', help=argparse.SUPPRESS, type=str)
    parser.add_argument("--gcp-logging", action="store_true")
    parser.add_argument("--listen-address", type=str, default="127.0.0.1")

    args = parser.parse_args(argv)

    if args.gcp_logging:
        import google.cloud.logging
        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    elif args.debug:
        log_level = logging.DEBUG
    else:
        log_level = logging.INFO

    logging.basicConfig(
        level=log_level,
        format="[%(asctime)s] %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S")

    global_config = GlobalStack()
    global_config.set("branch.fetch_tags", True)

    base_url = yarl.URL(args.base_url)

    if args.credentials:
        with open(args.credentials) as f:
            creds = json.load(f)
        auth = BasicAuth(login=creds["login"], password=creds["password"])
    elif 'WORKER_NAME' in os.environ and 'WORKER_PASSWORD' in os.environ:
        auth = BasicAuth(
            login=os.environ["WORKER_NAME"],
            password=os.environ["WORKER_PASSWORD"])
    else:
        auth = BasicAuth.from_url(base_url)

    if auth is not None:
        class WorkerCredentialStore(PlainTextCredentialStore):
            def get_credentials(
                self, protocol, host, port=None, user=None, path=None, realm=None
            ):
                if host == base_url.host:
                    return {
                        "user": auth.login,
                        "password": auth.password,
                        "protocol": protocol,
                        "port": port,
                        "host": host,
                        "realm": realm,
                        "verify_certificates": True,
                    }
                return None

        credential_store_registry.register(
            "janitor-worker", WorkerCredentialStore, fallback=True
        )

    if any(
        filter(
            os.environ.__contains__,
            ["BUILD_URL", "EXECUTOR_NUMBER", "BUILD_ID", "BUILD_NUMBER"],
        )
    ):
        jenkins_metadata = {
            "build_url": os.environ.get("BUILD_URL"),
            "executor_number": os.environ.get("EXECUTOR_NUMBER"),
            "build_id": os.environ.get("BUILD_ID"),
            "build_number": os.environ.get("BUILD_NUMBER"),
        }
    else:
        jenkins_metadata = None

    node_name = os.environ.get("NODE_NAME")
    if not node_name:
        node_name = socket.gethostname()

    async with ClientSession(auth=auth) as session:
        try:
            assignment = await get_assignment(
                session, args.base_url, node_name, jenkins_metadata=jenkins_metadata
            )
        except asyncio.TimeoutError as e:
            logging.fatal("timeout while retrieving assignment: %s", e)
            return 1

        logging.debug("Got back assignment: %r", assignment)

        watchdog_petter = WatchdogPetter(
            args.base_url, auth, assignment['id'],
            queue_id=assignment['queue_id'])
        watchdog_petter.start()

        suite = assignment["suite"]
        branch_url = assignment["branch"]["url"]
        vcs_type = assignment["branch"]["vcs_type"]
        subpath = assignment["branch"].get("subpath", "") or ""
        if assignment["resume"]:
            resume_result = assignment["resume"].get("result")
            resume_branch_url = assignment["resume"]["branch_url"].rstrip("/")
            resume_branches = [
                (role, name, base.encode("utf-8"), revision.encode("utf-8"))
                for (role, name, base, revision) in assignment["resume"]["branches"]
            ]
        else:
            resume_result = None
            resume_branch_url = None
            resume_branches = None
        cached_branch_url = assignment["branch"].get("cached_url")
        command = assignment["command"]
        if isinstance(command, str):
            command = shlex.split(command)
        target = assignment["build"]["target"]
        build_environment = assignment["build"].get("environment", {})

        if args.vcs_location:
            vcs_manager = LocalVcsManager(args.vcs_location)
        else:
            vcs_manager = RemoteVcsManager(assignment["vcs_manager"])
        run_id = assignment["id"]

        possible_transports = []

        env = assignment["env"]

        vendor = build_environment.get('DEB_VENDOR', 'debian')

        os.environ.update(env)
        os.environ.update(build_environment)

        metadata = {"queue_id": assignment["queue_id"]}
        if jenkins_metadata:
            metadata["jenkins"] = jenkins_metadata

        with TemporaryDirectory(prefix='janitor') as output_directory:
            loop = asyncio.get_running_loop()
            app = web.Application()
            app['directory'] = output_directory
            app['assignment'] = assignment
            app['metadata'] = metadata
            app.router.add_get('/', handle_index, name='index')
            app.router.add_get('/assignment', handle_assignment, name='assignment')
            app.router.add_get('/logs/', handle_log_index, name='log-index')
            app.router.add_get('/logs/{filename}', handle_log, name='log')
            app.router.add_get('/health', handle_health, name='health')
            setup_metrics(app)
            runner = web.AppRunner(app)
            await runner.setup()
            site = web.TCPSite(runner, args.listen_address, args.port)
            await site.start()
            (site_addr, site_port) = site._server.sockets[0].getsockname()
            logging.info('Diagnostics available at http://%s:%d/', site_addr, site_port)
            watchdog_petter.track_log_directory(output_directory)

            start_time = datetime.utcnow()
            main_task = loop.run_in_executor(
                None,
                partial(
                    run_worker,
                    branch_url,
                    run_id,
                    subpath,
                    vcs_type,
                    os.environ,
                    command,
                    output_directory,
                    metadata,
                    vcs_manager,
                    vendor,
                    suite,
                    target=target,
                    resume_branch_url=resume_branch_url,
                    resume_branches=resume_branches,
                    cached_branch_url=cached_branch_url,
                    resume_subworker_result=resume_result,
                    possible_transports=possible_transports,
                ),
            )
            watchdog_petter.kill = main_task.cancel
            metadata["start_time"] = start_time.isoformat()
            try:
                result = await main_task
            except WorkerFailure as e:
                metadata.update(e.json())
                logging.info("Worker failed (%s): %s", e.code, e.description)
                # This is a failure for the worker, but returning 0 will cause
                # jenkins to mark the job having failed, which is not really
                # true.  We're happy if we get to successfully POST to /finish
                return 0
            except OSError as e:
                if e.errno == errno.ENOSPC:
                    metadata["code"] = "no-space-on-device"
                    metadata["description"] = str(e)
                else:
                    metadata["code"] = "worker-exception"
                    metadata["description"] = str(e)
                    raise
            except BaseException as e:
                metadata["code"] = "worker-failure"
                metadata["description"] = ''.join(traceback.format_exception_only(type(e), e)).rstrip('\n')
                raise
            else:
                metadata["code"] = None
                metadata.update(result.json())
                logging.info("%s", result.description)

                return 0
            finally:
                finish_time = datetime.utcnow()
                metadata["finish_time"] = finish_time.isoformat()
                logging.info("Elapsed time: %s", finish_time - start_time)

                try:
                    result = await upload_results(
                        session,
                        args.base_url,
                        assignment["id"],
                        metadata,
                        output_directory,
                    )
                except ResultUploadFailure as e:
                    sys.stderr.write(str(e))
                    sys.exit(1)

                logging.info('Results uploaded')

                if args.debug:
                    logging.debug("Result: %r", result)

                if args.prometheus:
                    push_to_gateway(
                        args.prometheus, job="janitor.pull_worker",
                        registry=REGISTRY)


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
