#!/usr/bin/python3
# Copyright (C) 2019-2020 Jelmer Vernooij <jelmer@jelmer.uk>
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

from aiohttp.web_middlewares import normalize_path_middleware
import aiozipkin
import asyncio
from contextlib import ExitStack
from functools import partial
import json
import logging
import mimeparse
import os
from redis.asyncio import Redis
import sys
from tempfile import TemporaryDirectory
import traceback
import warnings

from aiohttp import web

from . import set_user_agent, state
from .artifacts import ArtifactsMissing, get_artifact_manager
from .config import read_config
from .debian.debdiff import (
    run_debdiff,
    DebdiffError,
    filter_boring as filter_debdiff_boring,
    htmlize_debdiff,
    markdownify_debdiff,
)
from .diffoscope import (
    filter_boring as filter_diffoscope_boring,
    filter_irrelevant as filter_diffoscope_irrelevant,
    run_diffoscope,
    format_diffoscope,
    DiffoscopeError,
)
from aiohttp_openmetrics import setup_metrics


# Common prefix for temporary directories
TMP_PREFIX = 'janitor-differ'
PRECACHE_RETRIEVE_TIMEOUT = 300
routes = web.RouteTableDef()


def find_binaries(path):
    ret = []
    for entry in os.scandir(path):
        ret.append((entry.name, entry.path))
    return ret


def is_binary(n):
    return n.endswith(".deb") or n.endswith(".udeb")


class ArtifactRetrievalTimeout(Exception):
    """Timeout while retrieving artifacts."""


class DiffCommandError(Exception):
    """Generic diff command error."""

    def __init__(self, command, reason):
        self.command = command
        self.reason = reason


class DiffCommandTimeout(Exception):
    """Timeout while running diff command."""

    def __init__(self, command, timeout):
        self.command = command
        self.timeout = timeout


class DiffCommandMemoryError(Exception):
    """Memory error while running diff command."""


@routes.get("/debdiff/{old_id}/{new_id}", name="debdiff")
async def handle_debdiff(request):
    span = aiozipkin.request_span(request)
    old_id = request.match_info["old_id"]
    new_id = request.match_info["new_id"]

    old_run, new_run = await get_run_pair(request.app['pool'], old_id, new_id)

    cache_path = request.app['debdiff_cache_path'](old_run['id'], new_run['id'])
    if cache_path:
        try:
            with open(cache_path, "rb") as f:
                debdiff = f.read()
        except FileNotFoundError:
            debdiff = None
    else:
        debdiff = None

    if debdiff is None:
        logging.info(
            "Generating debdiff between %s (%s/%s/%s) and %s (%s/%s/%s)",
            old_run['id'],
            old_run['package'],
            old_run['build_version'],
            old_run['campaign'],
            new_run['id'],
            new_run['package'],
            new_run['build_version'],
            new_run['campaign'],
        )
        with ExitStack() as es:
            old_dir = es.enter_context(TemporaryDirectory(prefix=TMP_PREFIX))
            new_dir = es.enter_context(TemporaryDirectory(prefix=TMP_PREFIX))

            try:
                with span.new_child('fetch-artifacts'):
                    await asyncio.gather(
                        request.app['artifact_manager'].retrieve_artifacts(
                            old_run['id'], old_dir, filter_fn=is_binary
                        ),
                        request.app['artifact_manager'].retrieve_artifacts(
                            new_run['id'], new_dir, filter_fn=is_binary
                        ),
                    )
            except ArtifactsMissing as e:
                raise web.HTTPNotFound(
                    text="No artifacts for run id: %r" % e,
                    headers={"unavailable_run_id": e.args[0]},
                ) from e
            except asyncio.TimeoutError as e:
                raise web.HTTPGatewayTimeout(
                    text="Timeout retrieving artifacts") from e

            old_binaries = find_binaries(old_dir)
            if not old_binaries:
                raise web.HTTPNotFound(
                    text="No artifacts for run id: %s" % old_run['id'],
                    headers={"unavailable_run_id": old_run['id']},
                )

            new_binaries = find_binaries(new_dir)
            if not new_binaries:
                raise web.HTTPNotFound(
                    text="No artifacts for run id: %s" % new_run['id'],
                    headers={"unavailable_run_id": new_run['id']},
                )

            try:
                with span.new_child('run-debdiff'):
                    debdiff = await run_debdiff(
                        [p for (n, p) in old_binaries], [p for (n, p) in new_binaries]
                    )
            except DebdiffError as e:
                return web.Response(status=400, text=e.args[0])
            except asyncio.TimeoutError as e:
                raise web.HTTPGatewayTimeout(
                    text="Timeout running debdiff") from e
        assert debdiff

        if cache_path:
            with open(cache_path, "wb") as f:
                f.write(debdiff)

    if "filter_boring" in request.query:
        debdiff = filter_debdiff_boring(
            debdiff.decode(), str(old_run['build_version']), str(new_run['build_version'])
        ).encode()

    content_type = mimeparse.best_match(
        ['text/x-diff', 'text/plain', 'text/markdown', 'text/html'],
        request.headers.get("Accept", "*/*"))
    if content_type in ("text/x-diff", "text/plain"):
        return web.Response(body=debdiff, content_type="text/plain")
    if content_type == "text/markdown":
        return web.Response(
            text=markdownify_debdiff(debdiff.decode("utf-8", "replace")),
            content_type="text/markdown",
        )
    if content_type == "text/html":
        return web.Response(
            text=htmlize_debdiff(debdiff.decode("utf-8", "replace")),
            content_type="text/html",
        )
    raise web.HTTPNotAcceptable(
        text="Acceptable content types: text/html, text/plain, text/markdown"
    )


async def get_run(conn, run_id):
    return await conn.fetchrow("""\
SELECT result_code, package, suite AS campaign, id, debian_build.version AS build_version, main_branch_revision
FROM run
LEFT JOIN debian_build ON debian_build.run_id = run.id
WHERE id = $1""", run_id)


async def get_unchanged_run(conn, package, main_branch_revision):
    query = """
SELECT result_code, package, suite AS campaign, id, debian_build.version AS build_version
FROM
    run
LEFT JOIN
    debian_build ON debian_build.run_id = run.id
WHERE
    revision = $1 AND
    package = $2 AND
    result_code = 'success' AND
    change_set IS NULL
ORDER BY finish_time DESC
"""
    return await conn.fetchrow(query, main_branch_revision, package)


async def get_run_pair(pool, old_id, new_id):
    async with pool.acquire() as conn:
        new_run = await get_run(conn, new_id)
        old_run = await get_run(conn, old_id)

    if old_run is None or old_run['result_code'] != 'success':
        raise web.HTTPNotFound(
            text="missing artifacts", headers={"unavailable_run_id": old_id}
        )

    if new_run is None or new_run['result_code'] != 'success':
        raise web.HTTPNotFound(
            text="missing artifacts", headers={"unavailable_run_id": new_id}
        )

    return old_run, new_run


def _set_limits(limit_mb):
    if limit_mb is None:
        return
    import resource

    limit = limit_mb * (1024 ** 2)
    # Limit to 1Gb
    resource.setrlimit(resource.RLIMIT_AS, (int(0.8 * limit), limit))


@routes.get("/diffoscope/{old_id}/{new_id}", name="diffoscope")
async def handle_diffoscope(request):
    span = aiozipkin.request_span(request)

    content_type = mimeparse.best_match(
        ['text/plain', 'text/html', 'application/json', 'text/markdown'],
        request.headers.get('Accept', '*/*'))

    if content_type is None:
        raise web.HTTPNotAcceptable(
            text="Acceptable content types: "
            "text/html, text/plain, application/json, "
            "application/markdown"
        )

    old_id = request.match_info["old_id"]
    new_id = request.match_info["new_id"]

    old_run, new_run = await get_run_pair(request.app['pool'], old_id, new_id)

    cache_path = request.app['diffoscope_cache_path'](old_run['id'], new_run['id'])
    if cache_path:
        try:
            with open(cache_path, "rb") as f:
                diffoscope_diff = json.load(f)
        except FileNotFoundError:
            diffoscope_diff = None
    else:
        diffoscope_diff = None

    if diffoscope_diff is None:
        logging.info(
            "Generating diffoscope between %s (%s/%s/%s) and %s (%s/%s/%s)",
            old_run['id'],
            old_run['package'],
            old_run['build_version'],
            old_run['campaign'],
            new_run['id'],
            new_run['package'],
            new_run['build_version'],
            new_run['campaign'],
        )
        with ExitStack() as es:
            old_dir = es.enter_context(TemporaryDirectory(prefix=TMP_PREFIX))
            new_dir = es.enter_context(TemporaryDirectory(prefix=TMP_PREFIX))

            try:
                with span.new_child('fetch-artifacts'):
                    await asyncio.gather(
                        request.app['artifact_manager'].retrieve_artifacts(
                            old_run['id'], old_dir, filter_fn=is_binary
                        ),
                        request.app['artifact_manager'].retrieve_artifacts(
                            new_run['id'], new_dir, filter_fn=is_binary
                        ),
                    )
            except ArtifactsMissing as e:
                raise web.HTTPNotFound(
                    text="No artifacts for run id: %r" % e,
                    headers={"unavailable_run_id": e.args[0]},
                ) from e
            except asyncio.TimeoutError as e:
                raise web.HTTPGatewayTimeout(
                    text="Timeout retrieving artifacts") from e

            old_binaries = find_binaries(old_dir)
            if not old_binaries:
                raise web.HTTPNotFound(
                    text="No artifacts for run id: %s" % old_run['id'],
                    headers={"unavailable_run_id": old_run['id']},
                )

            new_binaries = find_binaries(new_dir)
            if not new_binaries:
                raise web.HTTPNotFound(
                    text="No artifacts for run id: %s" % new_run['id'],
                    headers={"unavailable_run_id": new_run['id']},
                )

            try:
                with span.new_child('run-diffoscope'):
                    diffoscope_diff = await run_diffoscope(
                        old_binaries, new_binaries,
                        timeout=request.app['task_timeout'],
                        preexec_fn=lambda: _set_limits(request.app['task_memory_limit']),
                        diffoscope_command=request.app['diffoscope_command'])
            except MemoryError as e:
                raise web.HTTPServiceUnavailable(
                    text="diffoscope used too much memory") from e
            except asyncio.TimeoutError as e:
                raise web.HTTPGatewayTimeout(
                    text="diffoscope timed out") from e
            except DiffoscopeError as e:
                raise web.HTTPInternalServerError(
                    reason='diffoscope error', text=e.args[0]) from e

        if cache_path is not None:
            with open(cache_path, "w") as f:
                json.dump(diffoscope_diff, f)

    diffoscope_diff["source1"] = "%s version %s (%s)" % (
        old_run['package'],
        old_run['build_version'],
        old_run['campaign'],
    )
    diffoscope_diff["source2"] = "%s version %s (%s)" % (
        new_run['package'],
        new_run['build_version'],
        new_run['campaign'],
    )

    filter_diffoscope_irrelevant(diffoscope_diff)

    title = "diffoscope for %s applied to %s" % (new_run['campaign'], new_run['package'])

    if "filter_boring" in request.query:
        filter_diffoscope_boring(
            diffoscope_diff,
            str(old_run['build_version']),
            str(new_run['build_version']),
            old_run['campaign'],
            new_run['campaign'],
        )
        title += " (filtered)"

    with span.new_child('format-diffoscope'):
        debdiff = await format_diffoscope(
            diffoscope_diff,
            content_type,
            title=title,
            css_url=request.query.get("css_url"),
        )

    return web.Response(text=debdiff, content_type=content_type)


async def precache(
        artifact_manager, old_id, new_id, *,
        task_memory_limit=None, task_timeout=None,
        diffoscope_cache_path=None,
        debdiff_cache_path=None,
        diffoscope_command=None):
    """Precache the diff between two runs.

    Args:
      old_id: Run id for old run
      new_id: Run id for new run
    Raises:
      ArtifactsMissing: if either the old or new run artifacts are missing
      ArtifactRetrievalTimeout: if retrieving artifacts resulted in a timeout
      DiffCommandTimeout: if running the diff command triggered a timeout
      DiffCommandMemoryError: if the diff command used too much memory
      DiffCommandError: if a diff command failed
    """
    with ExitStack() as es:
        old_dir = es.enter_context(TemporaryDirectory(prefix=TMP_PREFIX))
        new_dir = es.enter_context(TemporaryDirectory(prefix=TMP_PREFIX))

        await asyncio.gather(
            artifact_manager.retrieve_artifacts(
                old_id, old_dir, filter_fn=is_binary, timeout=PRECACHE_RETRIEVE_TIMEOUT
            ),
            artifact_manager.retrieve_artifacts(
                new_id, new_dir, filter_fn=is_binary, timeout=PRECACHE_RETRIEVE_TIMEOUT
            ),
        )

        old_binaries = find_binaries(old_dir)
        if not old_binaries:
            raise ArtifactsMissing(old_id)

        new_binaries = find_binaries(new_dir)
        if not new_binaries:
            raise ArtifactsMissing(new_id)

        if debdiff_cache_path:
            p = debdiff_cache_path(old_id, new_id)
        else:
            p = None

        if p and not os.path.exists(p):
            with open(p, "wb") as f:
                f.write(
                    await run_debdiff(
                        [p for (n, p) in old_binaries], [p for (n, p) in new_binaries]
                    )
                )
            logging.info("Precached debdiff result for %s/%s", old_id, new_id)

        if diffoscope_cache_path:
            p = diffoscope_cache_path(old_id, new_id)
        else:
            p = None

        if p and not os.path.exists(p):
            try:
                diffoscope_diff = await run_diffoscope(
                    old_binaries, new_binaries,
                    preexec_fn=lambda: _set_limits(task_memory_limit),
                    timeout=task_timeout,
                    diffoscope_command=diffoscope_command)
            except MemoryError as e:
                raise DiffCommandMemoryError(
                    "diffoscope", task_memory_limit) from e
            except asyncio.TimeoutError as e:
                raise DiffCommandTimeout("diffoscope", task_timeout) from e
            except DiffoscopeError as e:
                raise DiffCommandError("diffoscope", e.args[0]) from e

            try:
                with open(p, "w") as f:
                    json.dump(diffoscope_diff, f)
            except json.JSONDecodeError as e:
                raise web.HTTPServerError(text=str(e)) from e
            logging.info("Precached diffoscope result for %s/%s", old_id, new_id)


def create_background_task(fn, title):
    loop = asyncio.get_event_loop()
    task = loop.create_task(fn)

    def log_result(future):
        try:
            future.result()
        except BaseException:
            logging.exception('%s failed', title)
        else:
            logging.debug('%s succeeded', title)
    task.add_done_callback(log_result)


@routes.post("/precache/{old_id}/{new_id}", name="precache")
async def handle_precache(request):
    old_id = request.match_info["old_id"]
    new_id = request.match_info["new_id"]

    old_run, new_run = await get_run_pair(request.app['pool'], old_id, new_id)

    async def _precache():
        return precache(
            request.app['artifact_manager'],
            old_run['id'], new_run['id'],
            task_memory_limit=request.app['task_memory_limit'],
            task_timeout=request.app['task_timeout'],
            diffoscope_cache_path=request.app['diffoscope_cache_path'],
            debdiff_cache_path=request.app['debdiff_cache_path'],
            diffoscope_command=request.app['diffoscope_command'])

    create_background_task(_precache(), 'precaching')

    return web.Response(status=202, text="Precaching started")


@routes.post("/precache-all", name="precache-all")
async def handle_precache_all(request):
    todo = []
    async with request.app['pool'].acquire() as conn:
        rows = await conn.fetch(
            """
select run.id, unchanged_run.id from run
inner join run as unchanged_run
on run.main_branch_revision = unchanged_run.revision
where
  run.result_code = 'success' and
  unchanged_run.result_code = 'success' and
  run.main_branch_revision != run.revision and
  suite not in ('control', 'unchanged')
 order by run.finish_time desc, unchanged_run.finish_time desc
"""
        )
        for row in rows:
            todo.append(precache(
                request.app['artifact_manager'],
                row[1], row[0],
                task_memory_limit=request.app['task_memory_limit'],
                task_timeout=request.app['task_timeout'],
                diffoscope_cache_path=request.app['diffoscope_cache_path'],
                debdiff_cache_path=request.app['debdiff_cache_path'],
                diffoscope_command=request.app['diffoscope_command']))


    async def _precache_all():
        for i in range(0, len(todo), 100):
            done, pending = await asyncio.wait(
                set(todo[i : i + 100]), return_when=asyncio.ALL_COMPLETED
            )
            for x in done:
                try:
                    x.result()
                except ArtifactRetrievalTimeout as e:
                    logging.info("Timeout retrieving artifacts: %s", e)
                except DiffCommandTimeout as e:
                    logging.info("Timeout diffing artifacts: %s", e)
                except DiffCommandMemoryError as e:
                    logging.info("Memory error diffing artifacts: %s", e)
                except DiffCommandError as e:
                    logging.info("Error diff artifacts: %s", e)
                except Exception as e:
                    logging.info("Error precaching: %r", e)
                    traceback.print_exc()

    create_background_task(_precache_all(), 'precache all')
    return web.Response(status=202, text="Precache started (todo: %d)" % len(todo))


@routes.get("/health", name="health")
async def handle_health(request):
    return web.Response(text="ok")


@routes.get("/ready", name="ready")
async def handle_ready(request):
    return web.Response(text="ok")


def diffoscope_cache_path(cache_path, old_id, new_id):
    base_path = os.path.join(cache_path, "diffoscope")
    if not os.path.isdir(base_path):
        os.mkdir(base_path)
    return os.path.join(base_path, "%s_%s.json" % (old_id, new_id))


def debdiff_cache_path(cache_path, old_id, new_id):
    base_path = os.path.join(cache_path, "debdiff")
    # This can happen when the default branch changes
    if not os.path.isdir(base_path):
        os.mkdir(base_path)
    return os.path.join(base_path, "%s_%s" % (old_id, new_id))


async def run_web_server(app, listen_addr, port):
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    try:
        await site.start()
        while True:
            await asyncio.sleep(3600)
    finally:
        await runner.cleanup()


async def listen_to_runner(redis, db, app):
    async def handle_result_message(msg):
        result = json.loads(msg['data'])
        if result["code"] != "success":
            return
        async with db.acquire() as conn:
            to_precache = []
            if result["revision"] == result["main_branch_revision"]:
                for row in await conn.fetch(
                    "select id from run where result_code = 'success' "
                    "and main_branch_revision = $1",
                    result["revision"],
                ):
                    to_precache.append((result["log_id"], row[0]))
            else:
                unchanged_run = await get_unchanged_run(
                    conn,
                    result["package"],
                    result["main_branch_revision"])
                if unchanged_run:
                    to_precache.append((unchanged_run['id'], result["log_id"]))
        # This could be concurrent, but risks hitting resource constraints
        # for large packages.
        for old_id, new_id in to_precache:
            try:
                await precache(
                    app['artifact_manager'], old_id, new_id,
                    task_memory_limit=app['task_memory_limit'],
                    task_timeout=app['task_timeout'],
                    diffoscope_cache_path=app['diffoscope_cache_path'],
                    debdiff_cache_path=app['debdiff_cache_path'],
                    diffoscope_command=app['diffoscope_command'])
            except ArtifactsMissing as e:
                logging.info(
                    "Artifacts missing while precaching diff for "
                    "new result %s: %r",
                    result["log_id"],
                    e,
                )
            except ArtifactRetrievalTimeout as e:
                logging.info("Timeout retrieving artifacts: %s", e)
            except DiffCommandTimeout as e:
                logging.info("Timeout diffing artifacts: %s", e)
            except DiffCommandMemoryError as e:
                logging.info("Memory error diffing artifacts: %s", e)
            except DiffCommandError as e:
                logging.info("Error diff artifacts: %s", e)
            except Exception as e:
                logging.info(
                    "Error precaching diff for %s: %r", result["log_id"], e
                )
                traceback.print_exc()

    try:
        async with redis.pubsub(ignore_subscribe_messages=True) as ch:
            await ch.subscribe('result', result=handle_result_message)
            await ch.run()
    finally:
        await redis.close()


def create_app(cache_path, artifact_manager, database_location, *,
               task_memory_limit=None, task_timeout=None,
               diffoscope_command=None):
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[
        trailing_slash_redirect, state.asyncpg_error_middleware])
    app.router.add_routes(routes)
    app['artifact_manager'] = artifact_manager
    app['task_memory_limit'] = task_memory_limit
    app['task_timeout'] = task_timeout
    app['diffoscope_cache_path'] = partial(diffoscope_cache_path, cache_path)
    app['debdiff_cache_path'] = partial(debdiff_cache_path, cache_path)
    app['diffoscope_command'] = diffoscope_command

    async def connect_artifact_manager(app):
        await app['artifact_manager'].__aenter__()

    app.on_startup.append(connect_artifact_manager)

    async def connect_postgres(app):
        app['pool'] = await state.create_pool(database_location)

    app.on_startup.append(connect_postgres)

    return app


async def main(argv=None):
    import argparse

    parser = argparse.ArgumentParser(prog="janitor.differ")
    parser.add_argument(
        "--listen-address", type=str, help="Listen address", default="localhost"
    )
    parser.add_argument("--port", type=int, help="Listen port", default=9920)
    parser.add_argument(
        "--config", type=str, default="janitor.conf", help="Path to configuration."
    )
    parser.add_argument("--cache-path", type=str, default=None, help="Path to cache.")
    parser.add_argument(
        '--task-memory-limit', help='Task memory limit (in MB)',
        type=int, default=1500)
    parser.add_argument(
        '--task-timeout', help='Task timeout (in seconds)',
        type=int, default=60)
    parser.add_argument('--gcp-logging', action='store_true')
    parser.add_argument('--diffoscope-command', type=str, default='diffoscope')
    parser.add_argument('--debug', action='store_true')

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

    with open(args.config, "r") as f:
        config = read_config(f)

    set_user_agent(config.user_agent)

    endpoint = aiozipkin.create_endpoint("janitor.differ", ipv4=args.listen_address, port=args.port)
    if config.zipkin_address:
        tracer = await aiozipkin.create(config.zipkin_address, endpoint, sample_rate=0.1)
    else:
        tracer = await aiozipkin.create_custom(endpoint)
    trace_configs = [aiozipkin.make_trace_config(tracer)]

    artifact_manager = get_artifact_manager(
        config.artifact_location, trace_configs=trace_configs)

    loop = asyncio.get_event_loop()

    if args.debug:
        loop.set_debug(True)
        loop.slow_callback_duration = 0.001
        warnings.simplefilter('always', ResourceWarning)

    if args.cache_path and not os.path.isdir(args.cache_path):
        os.makedirs(args.cache_path)

    redis = Redis.from_url(config.redis_location)

    app = create_app(
        args.cache_path, artifact_manager, config.database_location,
        task_memory_limit=args.task_memory_limit,
        task_timeout=args.task_timeout,
        diffoscope_command=args.diffoscope_command)
    setup_metrics(app)
    aiozipkin.setup(app, tracer)

    tasks = [loop.create_task(run_web_server(app, args.listen_address, args.port))]

    db = await state.create_pool(config.database_location)

    tasks.append(loop.create_task(listen_to_runner(redis, db, app)))

    await asyncio.gather(*tasks)


if __name__ == "__main__":
    sys.exit(asyncio.run(main(sys.argv)))
