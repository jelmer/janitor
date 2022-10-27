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

"""Serve the janitor site."""

import asyncio
import functools
import logging
import os
import re
import shutil
import tempfile
import time

import aioredis
import aiozipkin
from aiohttp.web_urldispatcher import (
    URL,
)
from aiohttp import web, ClientSession, ClientConnectorError
from aiohttp_openmetrics import metrics, metrics_middleware
from aiohttp.web_middlewares import normalize_path_middleware
import gpg

from .. import state
from ..logs import get_log_manager
from ..vcs import get_vcs_managers_from_config

from . import (
    env,
)

from .common import (
    html_template,
    render_template_for_request,
)
from .openid import setup_openid
from .pubsub import pubsub_handler, Topic


def create_background_task(fn, title):
    loop = asyncio.get_event_loop()
    task = loop.create_task(fn)

    def log_result(future):
        try:
            future.result()
        except asyncio.CancelledError:
            logging.debug('%s cancelled', title)
        except BaseException:
            logging.exception('%s failed', title)
        else:
            logging.debug('%s succeeded', title)
    task.add_done_callback(log_result)
    return task


async def get_credentials(session, publisher_url):
    url = URL(publisher_url) / "credentials"
    async with session.get(url=url) as resp:
        if resp.status != 200:
            raise Exception("unexpected response")
        return await resp.json()


async def handle_simple(templatename, request):
    vs = {}
    return web.Response(
        content_type="text/html",
        text=await render_template_for_request(env, templatename, request, vs),
        headers={"Vary": "Cookie"},
    )


@html_template(env, "generic/start.html")
async def handle_generic_start(request):
    return {"suite": request.match_info["suite"]}


@html_template(env, "generic/candidates.html", headers={"Vary": "Cookie"})
async def handle_generic_candidates(request):
    from .common import generate_candidates

    return await generate_candidates(
        request.app.database, suite=request.match_info["suite"]
    )


@html_template(env, "merge-proposals.html", headers={"Vary": "Cookie"})
async def handle_merge_proposals(request):
    from .merge_proposals import write_merge_proposals

    suite = request.match_info.get("suite")
    return await write_merge_proposals(request.app.database, suite)


@html_template(env, "merge-proposal.html", headers={"Vary": "Cookie"})
async def handle_merge_proposal(request):
    from .merge_proposals import write_merge_proposal

    url = request.query["url"]
    return await write_merge_proposal(request.app.database, url)


@html_template(env, "credentials.html", headers={"Vary": "Cookie"})
async def handle_credentials(request):
    try:
        credentials = await get_credentials(
            request.app['http_client_session'], request.app['publisher_url']
        )
    except ClientConnectorError:
        return web.Response(status=500, text='Unable to retrieve credentials')
    pgp_fprs = []
    for keydata in credentials["pgp_keys"]:
        result = request.app['gpg'].key_import(keydata.encode("utf-8"))
        pgp_fprs.extend([i.fpr for i in result.imports])

    pgp_validity = {
        gpg.constants.VALIDITY_FULL: "full",
        gpg.constants.VALIDITY_MARGINAL: "marginal",
        gpg.constants.VALIDITY_NEVER: "never",
        gpg.constants.VALIDITY_ULTIMATE: "ultimate",
        gpg.constants.VALIDITY_UNDEFINED: "undefined",
        gpg.constants.VALIDITY_UNKNOWN: "unknown",
    }

    return {
        "format_pgp_date": lambda ts: time.strftime("%Y-%m-%d", time.localtime(ts)),
        "pgp_validity": pgp_validity.get,
        "pgp_algo": gpg.core.pubkey_algo_name,
        "ssh_keys": credentials["ssh_keys"],
        "pgp_keys": request.app['gpg'].keylist("\0".join(pgp_fprs)),
        "hosting": credentials["hosting"],
    }


async def handle_ssh_keys(request):
    credentials = await get_credentials(
        request.app['http_client_session'], request.app['publisher_url']
    )
    return web.Response(
        text="\n".join(credentials["ssh_keys"]), content_type="text/plain"
    )


async def handle_pgp_keys(request):
    credentials = await get_credentials(
        request.app['http_client_session'], request.app['publisher_url']
    )
    armored = request.match_info["extension"] == ".asc"
    if armored:
        return web.Response(
            text="\n".join(credentials["pgp_keys"]),
            content_type="application/pgp-keys",
        )
    else:
        fprs = []
        for keydata in credentials["pgp_keys"]:
            result = request.app['gpg'].key_import(keydata.encode("utf-8"))
            fprs.extend([i.fpr for i in result.imports])
        return web.Response(
            body=request.app['gpg'].key_export_minimal("\0".join(fprs)),
            content_type="application/pgp-keys",
        )


async def handle_archive_keyring(request):
    url = URL(request.app['archiver_url']) / "pgp_keys"
    async with request.app['http_client_session'].get(url=url) as resp:
        if resp.status != 200:
            raise Exception("unexpected response")
        pgp_keys = await resp.json()
    armored = request.match_info["extension"] == ".asc"
    if armored:
        return web.Response(
            text="\n".join(pgp_keys),
            content_type="application/pgp-keys",
        )
    else:
        fprs = []
        for keydata in pgp_keys:
            result = request.app['gpg'].key_import(keydata.encode("utf-8"))
            fprs.extend([i.fpr for i in result.imports])
        return web.Response(
            body=request.app['gpg'].key_export_minimal("\0".join(fprs)),
            content_type="application/pgp-keys",
        )


async def handle_static_file(path, request):
    return web.FileResponse(path)


async def handle_result_file(request):
    pkg = request.match_info["pkg"]
    filename = request.match_info["filename"]
    run_id = request.match_info["run_id"]
    if not re.match("^[a-z0-9+-\\.]+$", pkg) or len(pkg) < 2:
        raise web.HTTPNotFound(text="Invalid package %s for run %s" % (pkg, run_id))
    if not re.match("^[a-z0-9-]+$", run_id) or len(run_id) < 5:
        raise web.HTTPNotFound(text="Invalid run run id %s" % (run_id,))
    if filename.endswith(".log") or re.match(r".*\.log\.[0-9]+", filename):
        if not re.match("^[+a-z0-9\\.]+$", filename) or len(filename) < 3:
            raise web.HTTPNotFound(
                text="No log file %s for run %s" % (filename, run_id)
            )

        try:
            logfile = await request.app.logfile_manager.get_log(pkg, run_id, filename)
        except FileNotFoundError as e:
            raise web.HTTPNotFound(
                text="No log file %s for run %s" % (filename, run_id)
            ) from e
        else:
            with logfile as f:
                text = f.read().decode("utf-8", "replace")
        return web.Response(
            content_type="text/plain",
            text=text,
        )
    else:
        try:
            f = await request.app['artifact_manager'].get_artifact(
                run_id, filename
            )
        except FileNotFoundError as e:
            raise web.HTTPNotFound(
                text="No artifact %s for run %s" % (filename, run_id)) from e
        return web.Response(body=f.read())


@html_template(env, "ready-list.html", headers={"Vary": "Cookie"})
async def handle_ready_proposals(request):
    from .pkg import generate_ready_list

    suite = request.match_info.get("suite")
    review_status = request.query.get("review_status")
    return await generate_ready_list(request.app.database, suite, review_status)


@html_template(env, "generic/package.html", headers={"Vary": "Cookie"})
async def handle_generic_pkg(request):
    from .common import generate_pkg_context

    # TODO(jelmer): Handle Accept: text/diff
    pkg = request.match_info["pkg"]
    run_id = request.match_info.get("run_id")
    return await generate_pkg_context(
        request.app.database,
        request.app['config'],
        request.match_info["suite"],
        request.app['http_client_session'],
        request.app['differ_url'],
        request.app['vcs_managers'],
        pkg,
        aiozipkin.request_span(request),
        run_id,
    )


@html_template(env, "repo-list.html")
async def handle_repo_list(request):
    vcs = request.match_info["vcs"]
    url = request.app['vcs_managers'][vcs].base_url
    async with request.app['http_client_session'].get(url) as resp:
        return {"vcs": vcs, "repositories": await resp.json()}


async def handle_health(request):
    return web.Response(text='ok')


async def create_app(
        config, minified=False,
        external_url=None, debugtoolbar=None,
        runner_url=None, publisher_url=None,
        archiver_url=None, vcs_managers=None,
        differ_url=None,
        listen_address=None, port=None):
    if minified:
        minified_prefix = ""
    else:
        minified_prefix = "min."

    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[
        metrics_middleware, trailing_slash_redirect, state.asyncpg_error_middleware])
    private_app = web.Application(middlewares=[
        metrics_middleware, trailing_slash_redirect, state.asyncpg_error_middleware])

    private_app.router.add_get("/metrics", metrics, name="metrics")
    private_app.router.add_get("/health", handle_health, name="health")

    app.topic_notifications = Topic("notifications")
    app.router.add_get(
        "/ws/notifications",
        functools.partial(pubsub_handler, app.topic_notifications),  # type: ignore
        name="ws-notifications",
    )

    endpoint = aiozipkin.create_endpoint("janitor.site", ipv4=listen_address, port=port)
    if config.zipkin_address:
        tracer = await aiozipkin.create(config.zipkin_address, endpoint, sample_rate=0.1)
    else:
        tracer = await aiozipkin.create_custom(endpoint)
    trace_configs = [aiozipkin.make_trace_config(tracer)]

    aiozipkin.setup(private_app, tracer, skip_routes=[
        private_app.router['metrics'],
    ])
    aiozipkin.setup(app, tracer, skip_routes=[
        app.router['ws-notifications'],
    ])

    async def persistent_session(app):
        app['http_client_session'] = session = ClientSession(trace_configs=trace_configs)
        yield
        await session.close()

    app.cleanup_ctx.append(persistent_session)

    async def start_gpg_context(app):
        gpg_home = tempfile.TemporaryDirectory()
        gpg_context = gpg.Context(home_dir=gpg_home.name)
        app['gpg'] = gpg_context.__enter__()

        async def cleanup_gpg(app):
            gpg_context.__exit__(None, None, None)
            shutil.rmtree(gpg_home)

        app.on_cleanup.append(cleanup_gpg)

    async def connect_redis(app):
        app['redis'] = await aioredis.create_redis_pool(config.redis_location)

    async def disconnect_redis(app):
        app['redis'].close()

    app.on_startup.append(connect_redis)
    app.on_cleanup.append(disconnect_redis)

    async def start_pubsub_forwarder(app):
        async def listen_to_publisher_publish(app):
            ch = (await app['redis'].subscribe('publish'))[0]
            while (await ch.wait_message()):
                app.topic_notifications.publish(["publish", await ch.get_json()])

        async def listen_to_publisher_mp(app):
            ch = (await app['redis'].subscribe('merge-proposal'))[0]
            while (await ch.wait_message()):
                app.topic_notifications.publish(["merge-proposal", await ch.get_json()])

        app['runner_status'] = None

        async def listen_to_queue(app):
            ch = (await app['redis'].subscribe('queue'))[0]
            while (await ch.wait_message()):
                msg = await ch.get_json()
                app['runner_status'] = msg
                app.topic_notifications.publish(["queue", msg])

        async def listen_to_result(app):
            ch = (await app['redis'].subscribe('result'))[0]
            while (await ch.wait_message()):
                app.topic_notifications.publish(["result", await ch.get_json()])

        for cb, title in [
            (listen_to_publisher_publish, 'publisher publish listening'),
            (listen_to_publisher_mp, 'merge proposal listening'),
            (listen_to_queue, 'queue listening'),
            (listen_to_result, 'result listening'),
        ]:
            listener = create_background_task(cb(app), title)

            async def stop_listener(app):
                listener.cancel()
                await listener

            app.on_cleanup.append(stop_listener)

    for path, templatename in [
        ("/", "index"),
        ("/contact", "contact"),
        ("/about", "about"),
    ]:
        app.router.add_get(
            path,
            functools.partial(handle_simple, templatename + ".html"),
            name=templatename,
        )
    app.router.add_get("/credentials", handle_credentials, name="credentials")
    app.router.add_get("/ssh_keys", handle_ssh_keys, name="ssh-keys")
    app.router.add_get(
        r"/pgp_keys{extension:(\.asc)?}", handle_pgp_keys, name="pgp-keys"
    )
    app.router.add_get(
        r"/archive-keyring{extension:(\.asc|\.gpg)}", handle_archive_keyring,
        name="archive-keyring"
    )
    CAMPAIGN_REGEX = "|".join([re.escape(campaign.name) for campaign in config.campaign])
    app.router.add_get(
        "/{suite:%s}/merge-proposals" % CAMPAIGN_REGEX,
        handle_merge_proposals,
        name="suite-merge-proposals",
    )
    app.router.add_get(
        "/{suite:%s}/merge-proposal" % CAMPAIGN_REGEX,
        handle_merge_proposal,
        name="suite-merge-proposal",
    )
    app.router.add_get(
        "/{suite:%s}/ready" % CAMPAIGN_REGEX, handle_ready_proposals, name="suite-ready"
    )
    app.router.add_get(
        "/{vcs:git|bzr}/", handle_repo_list, name="repo-list")
    from .cupboard import register_cupboard_endpoints
    register_cupboard_endpoints(app.router)
    app.router.add_get(
        "/{suite:" + CAMPAIGN_REGEX + "}/pkg/{pkg}/{run_id}/{filename:.+}",
        handle_result_file,
        name="result-file",
    )
    app.router.add_get(
        "/{suite:" + CAMPAIGN_REGEX + "}/", handle_generic_start, name="generic-start"
    )
    app.router.add_get(
        "/{suite:" + CAMPAIGN_REGEX + "}/candidates",
        handle_generic_candidates,
        name="generic-candidates",
    )
    app.router.add_get(
        "/{suite:" + CAMPAIGN_REGEX + "}/pkg/{pkg}/",
        handle_generic_pkg,
        name="generic-package",
    )
    app.router.add_get(
        "/{suite:" + CAMPAIGN_REGEX + "}/pkg/{pkg}/{run_id}",
        handle_generic_pkg,
        name="generic-run",
    )
    for entry in os.scandir(os.path.join(os.path.dirname(__file__), "_static")):
        app.router.add_get(
            "/_static/%s" % entry.name,
            functools.partial(handle_static_file, entry.path),
        )
    app.router.add_static(
        "/_static/images/datatables", "/usr/share/javascript/jquery-datatables/images"
    )
    for (name, kind, basepath) in [
        ("chart", "js", "/usr/share/javascript/chart.js/Chart"),
        ("chart", "css", "/usr/share/javascript/chart.js/Chart"),
        ("jquery", "js", "/usr/share/javascript/jquery/jquery"),
        (
            "jquery.typeahead",
            "js",
            "/usr/share/javascript/jquery-typeahead/jquery.typeahead",
        ),
        (
            "jquery.datatables",
            "js",
            "/usr/share/javascript/jquery-datatables/jquery.dataTables",
        ),
        ("moment", "js", "/usr/share/javascript/moment/moment"),
    ]:
        if not os.path.exists(basepath + "." + kind):
            continue
        app.router.add_get(
            "/_static/%s.%s" % (name, kind),
            functools.partial(
                handle_static_file, "%s.%s%s" % (basepath, minified_prefix, kind)
            ),
        )
    from .api import create_app as create_api_app
    from .webhook import process_webhook, is_webhook_request

    async def handle_post_root(request):
        if is_webhook_request(request):
            return await process_webhook(request, request.app.database)
        raise web.HTTPMethodNotAllowed(method='POST', allowed_methods=['GET', 'HEAD'])

    app['runner_url'] = runner_url
    app['archiver_url'] = archiver_url
    app['differ_url'] = differ_url
    app['publisher_url'] = publisher_url
    app['vcs_managers'] = vcs_managers
    app.on_startup.append(start_pubsub_forwarder)
    app.on_startup.append(start_gpg_context)
    if external_url:
        app['external_url'] = URL(external_url)
    else:
        app['external_url'] = None
    database = await state.create_pool(config.database_location)
    app['pool'] = database
    app.database = database
    app['config'] = config

    from janitor.artifacts import get_artifact_manager

    async def startup_artifact_manager(app):
        app['artifact_manager'] = get_artifact_manager(
            config.artifact_location, trace_configs=trace_configs)
        await app['artifact_manager'].__aenter__()

    async def turndown_artifact_manager(app):
        await app['artifact_manager'].__aexit__(None, None, None)

    app.on_startup.append(startup_artifact_manager)
    app.on_cleanup.append(turndown_artifact_manager)
    setup_openid(
        app, config.oauth2_provider.base_url if config.oauth2_provider else None)
    app.router.add_post("/", handle_post_root, name="root-post")
    from .stats import stats_app
    app.add_subapp(
        "/cupboard/stats", stats_app(app['pool'], config, app['external_url']))

    app.add_subapp(
        "/api",
        create_api_app(
            app['pool'],
            publisher_url,
            runner_url,  # type: ignore
            vcs_managers,
            differ_url,
            config,
            external_url=(
                app['external_url'].join(URL("api")) if app['external_url'] else None
            ),
            trace_configs=trace_configs,
        ),
    )
    import aiohttp_apispec
    app.router.add_static('/static/swagger', os.path.join(os.path.dirname(aiohttp_apispec.__file__), "static"))

    if debugtoolbar:
        import aiohttp_debugtoolbar
        # install aiohttp_debugtoolbar
        aiohttp_debugtoolbar.setup(app, hosts=debugtoolbar)

    async def setup_logfile_manager(app):
        app.logfile_manager = get_log_manager(config.logs_location, trace_configs=trace_configs)

    app.on_startup.append(setup_logfile_manager)
    return private_app, app


async def main(argv=None):
    import argparse
    from janitor.config import read_config

    parser = argparse.ArgumentParser()
    parser.add_argument("--debugtoolbar", type=str, action="append", help="IP to allow debugtoolbar queries from.")
    parser.add_argument("--host", type=str, help="Host to listen on")
    parser.add_argument("--port", type=int, help="Port to listen on", default=8080)
    parser.add_argument(
        "--public-port", type=int, help="Public port to listen on", default=8090)
    parser.add_argument(
        "--publisher-url",
        type=str,
        default="http://localhost:9912/",
        help="URL for publisher.",
    )
    parser.add_argument(
        "--vcs-store-url",
        type=str,
        default="http://localhost:9921/",
        help="URL for VCS store.",
    )
    parser.add_argument(
        "--runner-url",
        type=str,
        default="http://localhost:9911/",
        help="URL for runner.",
    )
    parser.add_argument(
        "--archiver-url",
        type=str,
        default="http://localhost:9914/",
        help="URL for runner.",
    )
    parser.add_argument(
        "--differ-url",
        type=str,
        default="http://localhost:9920/",
        help="URL for differ.",
    )
    parser.add_argument(
        "--config", type=str, default="janitor.conf", help="Path to configuration."
    )
    parser.add_argument(
        "--debug",
        action="store_true",
        help="Enable debugging mode. For example, avoid minified JS.",
    )
    parser.add_argument("--gcp-logging", action='store_true', help='Use Google cloud logging.')
    parser.add_argument("--external-url", type=str, default=None, help="External URL")

    args = parser.parse_args()

    if args.gcp_logging:
        import google.cloud.logging
        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    else:
        logging.basicConfig(level=logging.INFO)

    with open(args.config, "r") as f:
        config = read_config(f)

    private_app, public_app = await create_app(
        config, minified=args.debug,
        external_url=args.external_url,
        debugtoolbar=args.debugtoolbar,
        runner_url=args.runner_url,
        archiver_url=args.archiver_url,
        publisher_url=args.publisher_url,
        vcs_managers=get_vcs_managers_from_config(config),
        differ_url=args.differ_url,
        listen_address=args.host,
        port=args.port)

    private_runner = web.AppRunner(private_app)
    public_runner = web.AppRunner(public_app)
    await private_runner.setup()
    await public_runner.setup()
    site = web.TCPSite(private_runner, args.host, port=args.port)
    await site.start()
    logging.info("Listening on %s:%s", args.host, args.port)
    site = web.TCPSite(public_runner, args.host, port=args.public_port)
    await site.start()
    logging.info("Listening on %s:%s", args.host, args.public_port)
    while True:
        await asyncio.sleep(3600)


if __name__ == "__main__":
    import sys

    sys.exit(asyncio.run(main(sys.argv)))
