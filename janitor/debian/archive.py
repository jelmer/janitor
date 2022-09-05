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

import aiozipkin
import asyncio
from contextlib import ExitStack
import hashlib
import logging
import gzip
import bz2
import os
import shutil
import subprocess
import tempfile
import sys
import traceback
from typing import List, Dict
from email.utils import formatdate
from datetime import datetime
from time import mktime


from aiohttp import web
from aiohttp.web_middlewares import normalize_path_middleware

from debian.deb822 import Release, Packages

import gpg
from gpg.constants.sig import mode as gpg_mode

from aiohttp_openmetrics import (
    Gauge,
    setup_metrics,
)

from .. import state
from ..artifacts import get_artifact_manager, ArtifactsMissing
from ..config import read_config, get_distribution, Campaign, get_campaign_config


DEFAULT_GCS_TIMEOUT = 60 * 30

last_publish_time: Dict[str, datetime] = {}


last_publish_success = Gauge(
    "last_suite_publish_success",
    "Last time publishing a suite succeeded",
    labelnames=("suite",),
)


logger = logging.getLogger('janitor.debian.archive')

# TODO(jelmer): Generate contents file


class PackageInfoProvider(object):
    def __init__(self, artifact_manager):
        self.artifact_manager = artifact_manager

    async def __aenter__(self):
        await self.artifact_manager.__aenter__()

    async def __aexit__(self, exc_tp, exc_val, exc_tb):
        await self.artifact_manager.__aexit__(exc_tp, exc_val, exc_tb)
        return False

    async def info_for_run(self, run_id, suite_name, package):
        with tempfile.TemporaryDirectory() as td:
            await self.artifact_manager.retrieve_artifacts(
                run_id, td, timeout=DEFAULT_GCS_TIMEOUT
            )
            p = subprocess.Popen(["dpkg-scanpackages", td], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            for para in Packages.iter_paragraphs(p.stdout):
                para["Filename"] = os.path.join(
                    suite_name,
                    "pkg",
                    package,
                    run_id,
                    os.path.basename(para["Filename"]),
                )
                yield bytes(para)
                yield b"\n"
            for line in p.stderr.readlines():
                if line.startswith(b'dpkg-scanpackages: '):
                    line = line[len(b'dpkg-scanpackages: '):]
                if line.startswith(b'info: '):
                    logging.debug('%s', line.rstrip(b'\n').decode())
                elif line.startswith(b'warning: '):
                    logging.warning('%s', line.rstrip(b'\n').decode())
                elif line.startswith(b'error: '):
                    logging.error('%s', line.rstrip(b'\n').decode())
                else:
                    logging.info(
                        'dpkg-scanpackages error: %s',
                        line.rstrip(b'\n').decode())

        await asyncio.sleep(0)


class CachingPackageInfoProvider(object):
    def __init__(self, primary_info_provider, cache_directory):
        self.primary_info_provider = primary_info_provider
        self.cache_directory = cache_directory

    async def __aenter__(self):
        await self.primary_info_provider.__aenter__()

    async def __aexit__(self, exc_tp, exc_val, exc_tb):
        await self.primary_info_provider.__aexit__(exc_tp, exc_val, exc_tb)
        return False

    async def info_for_run(self, run_id, suite_name, package):
        cache_path = os.path.join(self.cache_directory, run_id)
        try:
            with open(cache_path, "rb") as f:
                for chunk in f:
                    yield chunk
        except FileNotFoundError:
            chunks = []
            logger.debug('Retrieving artifacts for %s/%s (%s)',
                         suite_name, package, run_id)
            with open(cache_path, "wb") as f:
                async for chunk in self.primary_info_provider.info_for_run(
                    run_id, suite_name, package
                ):
                    f.write(chunk)
                    chunks.append(chunk)
            for chunk in chunks:
                yield chunk


async def get_packages(db, info_provider, suite_name, component, arch):
    # TODO(jelmer): Actually query component/arch
    async with db.acquire() as conn:
        rows = await conn.fetch(
            "SELECT DISTINCT ON (source) "
            "debian_build.source, debian_build.run_id, debian_build.distribution, "
            "debian_build.version FROM debian_build "
            "INNER JOIN run ON run.id = debian_build.run_id "
            "WHERE debian_build.distribution = $1 AND run.review_status != 'rejected' "
            "ORDER BY debian_build.source, debian_build.version DESC",
            suite_name,
        )

    logger.debug('Need to process %d rows for %s/%s/%s',
                 len(rows), suite_name, component, arch)
    for package, run_id, build_distribution, build_version in rows:
        try:
            async for chunk in info_provider.info_for_run(run_id, suite_name, package):
                yield chunk
                await asyncio.sleep(0)
        except ArtifactsMissing:
            logger.warning("Artifacts missing for %s (%s), skipping", package, run_id)
            continue


def add_file_info(r, base, p):
    hashes = {
        "MD5Sum": hashlib.md5(),
        "SHA1": hashlib.sha1(),
        "SHA256": hashlib.sha256(),
        "SHA512": hashlib.sha512(),
    }
    size = 0
    with open(os.path.join(base, p), "rb") as f:
        for chunk in f:
            for h in hashes.values():
                h.update(chunk)
            size += len(chunk)
    for h, v in hashes.items():
        r.setdefault(h, []).append({h.lower(): v.hexdigest(), "size": size, "name": p})


async def write_suite_files(
    base_path, db, package_info_provider, suite_name, archive_description,
    components, arches, origin, gpg_context
):

    stamp = mktime(datetime.utcnow().timetuple())

    r = Release()
    r["Origin"] = origin
    r["Label"] = archive_description
    r["Codename"] = suite_name
    r["Suite"] = suite_name
    r["Date"] = formatdate(timeval=stamp, localtime=False, usegmt=True)
    r["NotAutomatic"] = "yes"
    r["ButAutomaticUpgrades"] = "yes"
    r["Architectures"] = " ".join(arches)
    r["Components"] = " ".join(components)
    r["Description"] = "Generated by the Debian Janitor"

    for component in components:
        logger.debug('Publishing component %s/%s', suite_name, component)
        component_dir = component
        os.makedirs(os.path.join(base_path, component_dir), exist_ok=True)
        for arch in arches:
            arch_dir = os.path.join(component_dir, "binary-%s" % arch)
            os.makedirs(os.path.join(base_path, arch_dir), exist_ok=True)
            br = Release()
            br["Origin"] = origin
            br["Label"] = archive_description
            br["Archive"] = suite_name
            br["Architecture"] = arch
            br["Component"] = component
            bp = os.path.join(arch_dir, "Release")
            with open(os.path.join(base_path, bp), "wb") as f:
                r.dump(f)
            add_file_info(r, base_path, bp)

            packages_path = os.path.join(component, "binary-%s" % arch, "Packages")
            SUFFIXES = {
                "": open,
                ".gz": gzip.GzipFile,
                ".bz2": bz2.BZ2File,
            }
            with ExitStack() as es:
                fs = []
                for suffix, fn in SUFFIXES.items():
                    fs.append(
                        es.enter_context(
                            fn(os.path.join(base_path, packages_path + suffix), "wb")
                        )
                    )
                async for chunk in get_packages(
                        db, package_info_provider, suite_name, component, arch):
                    for f in fs:
                        f.write(chunk)
            for suffix in SUFFIXES:
                add_file_info(r, base_path, packages_path + suffix)
            await asyncio.sleep(0)
        await asyncio.sleep(0)

    logger.debug('Writing Release file for %s', suite_name)
    with open(os.path.join(base_path, "Release"), "wb") as f:
        r.dump(f)

    logger.debug('Writing Release.gpg file for %s', suite_name)
    data = gpg.Data(r.dump())
    with open(os.path.join(base_path, "Release.gpg"), "wb") as f:
        signature, result = gpg_context.sign(data, mode=gpg_mode.DETACH)
        f.write(signature)

    logger.debug('Writing InRelease file for %s', suite_name)
    data = gpg.Data(r.dump())
    with open(os.path.join(base_path, "InRelease"), "wb") as f:
        signature, result = gpg_context.sign(data, mode=gpg_mode.CLEAR)
        f.write(signature)


# TODO(jelmer): Don't hardcode this
ARCHES: List[str] = ["amd64"]


async def handle_publish(request):
    post = await request.post()
    suite = post.get("suite")
    for campaign_config in request.app.config.campaign:
        if not campaign_config.HasField('debian_build'):
            continue
        build_distribution = (campaign_config.debian_build.build_distribution or campaign_config.name)
        if suite is not None and build_distribution != suite:
            continue
        request.app.generator_manager.trigger(campaign_config)

    return web.json_response({})


async def handle_last_publish(request):
    return web.json_response(
        {suite: dt.isoformat() for (suite, dt) in last_publish_time.items()}
    )


async def handle_health(request):
    return web.Response(text='ok')


async def handle_ready(request):
    return web.Response(text='ok')


async def handle_index(request):
    return web.Response(text='')


async def handle_pgp_keys(request):
    pgp_keys = []
    for entry in list(request.app['gpg'].keylist(secret=True)):
        pgp_keys.append(request.app['gpg'].key_export_minimal(entry.fpr).decode())
    return web.json_response(pgp_keys)


async def run_web_server(listen_addr, port, dists_dir, config, generator_manager, tracer):
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[trailing_slash_redirect])
    app['gpg'] = gpg.Context(armor=True)
    app.config = config
    app.generator_manager = generator_manager
    setup_metrics(app)
    app.router.add_get("/", handle_index, name="index")
    app.router.add_static("/dists", dists_dir, show_index=True)
    app.router.add_post("/publish", handle_publish, name="publish")
    app.router.add_get("/last-publish", handle_last_publish, name="last-publish")
    app.router.add_get("/health", handle_health, name="health")
    app.router.add_get("/ready", handle_ready, name="ready")
    app.router.add_get("/pgp_keys", handle_pgp_keys, name="pgp-keys")
    aiozipkin.setup(app, tracer)
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()


async def listen_to_runner(redis_location, generator_manager):
    import aioredis
    redis = await aioredis.create_redis(redis_location)

    ch = (await redis.subscribe('result'))[0]
    try:
        while (await ch.wait_message()):
            result = await ch.get_json()
            if result["code"] != "success":
                continue
            campaign = get_campaign_config(generator_manager.config, result["campaign"])
            if campaign:
                generator_manager.trigger(campaign)
    finally:
        redis.close()


async def publish_suite(
    dists_directory, db, package_info_provider, config, suite, gpg_context
):
    if not suite.HasField('debian_build'):
        logger.info("%s is not a Debian suite", suite.name)
        return
    try:
        start_time = datetime.utcnow()
        logger.info("Publishing %s", suite.name)
        distribution = get_distribution(config, suite.debian_build.base_distribution)
        suite_path = os.path.join(dists_directory, suite.name)
        with tempfile.TemporaryDirectory(dir=dists_directory) as td:
            await write_suite_files(
                td,
                db,
                package_info_provider,
                suite.name,
                suite.debian_build.archive_description,
                components=distribution.component,
                arches=ARCHES,
                origin=config.origin,
                gpg_context=gpg_context,
            )
            old_suite_path = suite_path + '.old'
            if os.path.exists(old_suite_path):
                shutil.rmtree(suite_path + '.old')
            if os.path.exists(suite_path):
                os.rename(suite_path, suite_path + '.old')
            os.rename(td, suite_path)
        if os.path.exists(old_suite_path):
            shutil.rmtree(suite_path + '.old')

        logger.info(
            "Done publishing %s (took %s)", suite.name, datetime.utcnow() - start_time
        )
        last_publish_success.labels(suite=suite.name).set_to_current_time()
        last_publish_time[suite.name] = datetime.utcnow()
    except BaseException:
        traceback.print_exc()


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
    return task


class GeneratorManager(object):
    def __init__(self, dists_dir, db, config, package_info_provider, gpg_context):
        self.dists_dir = dists_dir
        self.db = db
        self.config = config
        self.package_info_provider = package_info_provider
        self.gpg_context = gpg_context
        self.generators = {}

    def trigger(self, campaign_config: Campaign):
        if not campaign_config.HasField('debian_build'):
            return
        try:
            task = self.generators[campaign_config.name]
        except KeyError:
            pass
        else:
            if not task.done():
                return
        self.generators[campaign_config.name] = create_background_task(
            publish_suite(
                self.dists_dir,
                self.db,
                self.package_info_provider,
                self.config,
                campaign_config,
                self.gpg_context,
            ), 'publish %s' % campaign_config.name
        )


async def loop_publish(config, generator_manager):
    while True:
        for suite in config.campaign:
            generator_manager.trigger(suite)
        # every 12 hours
        await asyncio.sleep(60 * 60 * 12)


async def main(argv=None):
    import argparse

    parser = argparse.ArgumentParser(prog="janitor.debian.archive")
    parser.add_argument(
        "--listen-address", type=str, help="Listen address", default="localhost"
    )
    parser.add_argument("--port", type=int, help="Listen port", default=9914)
    parser.add_argument(
        "--config", type=str, default="janitor.conf", help="Path to configuration."
    )
    parser.add_argument("--dists-directory", type=str, help="Dists directory")
    parser.add_argument("--cache-directory", type=str, help="Cache directory")
    parser.add_argument("--verbose", action='store_true')
    parser.add_argument("--gcp-logging", action='store_true', help='Use Google cloud logging.')

    args = parser.parse_args()
    if not args.dists_directory:
        parser.print_usage()
        sys.exit(1)

    if args.gcp_logging:
        import google.cloud.logging
        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    elif args.verbose:
        logging.basicConfig(level=logging.DEBUG)
    else:
        logging.basicConfig(level=logging.INFO)

    with open(args.config, "r") as f:
        config = read_config(f)

    os.makedirs(args.dists_directory, exist_ok=True)

    db = await state.create_pool(config.database_location)

    endpoint = aiozipkin.create_endpoint("janitor.debian.archive", ipv4=args.listen_address, port=args.port)
    if config.zipkin_address:
        tracer = await aiozipkin.create(config.zipkin_address, endpoint, sample_rate=1.0)
    else:
        tracer = await aiozipkin.create_custom(endpoint)
    trace_configs = [aiozipkin.make_trace_config(tracer)]

    artifact_manager = get_artifact_manager(config.artifact_location, trace_configs=trace_configs)

    gpg_context = gpg.Context()

    package_info_provider = PackageInfoProvider(artifact_manager)
    if args.cache_directory:
        os.makedirs(args.cache_directory, exist_ok=True)
        package_info_provider = CachingPackageInfoProvider(
            package_info_provider, args.cache_directory
        )

    generator_manager = GeneratorManager(
        args.dists_directory, db, config, package_info_provider, gpg_context,
    )

    loop = asyncio.get_event_loop()
    tasks = [
        loop.create_task(
            run_web_server(
                args.listen_address,
                args.port,
                args.dists_directory,
                config,
                generator_manager,
                tracer,
            )
        ),
        loop.create_task(loop_publish(config, generator_manager)),
    ]

    tasks.append(loop.create_task(
        listen_to_runner(
            config.redis_location,
            generator_manager)))

    async with package_info_provider:
        await asyncio.gather(*tasks)


if __name__ == "__main__":
    sys.exit(asyncio.run(main(sys.argv)))
