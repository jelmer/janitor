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

import asyncio
from contextlib import ExitStack
from datetime import datetime
import hashlib
import logging
import gzip
import bz2
import os
import subprocess
import tempfile
import sys
from typing import List, Dict

from aiohttp import web
from aiohttp.web_middlewares import normalize_path_middleware

from debian.changelog import format_date
from debian.deb822 import Release, Packages

import gpg
from gpg.constants.sig import mode as gpg_mode

from prometheus_client import (
    Gauge,
)

from .. import state
from ..artifacts import get_artifact_manager, ArtifactsMissing
from ..config import read_config, get_suite_config
from ..prometheus import setup_metrics
from ..pubsub import pubsub_reader


DEFAULT_GCS_TIMEOUT = 60 * 30

last_publish_time: Dict[str, datetime] = {}


last_publish_success = Gauge(
    "last_suite_publish_success",
    "Last time publishing a suite succeeded",
    labelnames=("suite",),
)


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
            p = subprocess.Popen(["dpkg-scanpackages", td], stdout=subprocess.PIPE)
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
            "SELECT DISTINCT ON (source) source, id, debian_build.version FROM run "
            "WHERE distribution = $1 "
            "ORDER BY source, version DESC",
            suite_name,
        )

    for package, run_id, build_verison in rows:
        try:
            async for chunk in info_provider.info_for_run(run_id, suite_name, package):
                yield chunk
        except ArtifactsMissing:
            logging.warning("Artifacts missing for %s (%s), skipping", package, run_id)
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
    base_path, db, package_info_provider, suite, components, arches, origin, gpg_context
):
    r = Release()
    r["Origin"] = origin
    r["Label"] = suite.debian_build.archive_description
    r["Codename"] = suite.name
    r["Suite"] = suite.name
    r["Date"] = format_date()
    r["NotAutomatic"] = "yes"
    r["ButAutomaticUpgrades"] = "yes"
    r["Architectures"] = " ".join(arches)
    r["Components"] = " ".join(components)
    r["Description"] = "Generated by the Debian Janitor"

    for component in components:
        component_dir = component
        os.makedirs(os.path.join(base_path, component_dir), exist_ok=True)
        for arch in arches:
            arch_dir = os.path.join(component_dir, "binary-%s" % arch)
            os.makedirs(os.path.join(base_path, arch_dir), exist_ok=True)
            packages_chunks = get_packages(
                db, package_info_provider, suite.name, component, arch
            )
            br = Release()
            br["Origin"] = origin
            br["Label"] = suite.debian_build.archive_description
            br["Archive"] = suite.name
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
                async for chunk in packages_chunks:
                    for f in fs:
                        f.write(chunk)
            for suffix in SUFFIXES:
                add_file_info(r, base_path, packages_path + suffix)

    with open(os.path.join(base_path, "Release"), "wb") as f:
        r.dump(f)

    data = gpg.Data(r.dump())
    with open(os.path.join(base_path, "Release.gpg"), "wb") as f:
        signature, result = gpg_context.sign(data, mode=gpg_mode.DETACH)
        f.write(signature)

    data = gpg.Data(r.dump())
    with open(os.path.join(base_path, "InRelease"), "wb") as f:
        signature, result = gpg_context.sign(data, mode=gpg_mode.CLEAR)
        f.write(signature)


# TODO(jelmer): Don't hardcode this
ARCHES: List[str] = ["amd64"]


async def handle_publish(request):
    post = await request.post()
    suite = post.get("suite")
    for suite_config in request.app.config.suite:
        if suite is not None and suite_config.name != suite:
            continue
        if not suite_config.HasField('debian_build'):
            continue
        request.app.generator_manager.trigger(suite_config)

    return web.json_response({})


async def handle_last_publish(request):
    return web.json_response(
        {suite: dt.isoformat() for (suite, dt) in last_publish_time.items()}
    )


async def run_web_server(listen_addr, port, dists_dir, config, generator_manager):
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[trailing_slash_redirect])
    app.config = config
    app.generator_manager = generator_manager
    setup_metrics(app)
    app.router.add_static("/dists", dists_dir)
    app.router.add_post("/publish", handle_publish, name="publish")
    app.router.add_get("/last-publish", handle_last_publish, name="last-publish")
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()


async def listen_to_runner(runner_url, generator_manager):
    from aiohttp.client import ClientSession
    import urllib.parse

    url = urllib.parse.urljoin(runner_url, "ws/result")
    async with ClientSession() as session:
        async for result in pubsub_reader(session, url):
            if result["code"] != "success":
                continue
            generator_manager.trigger(result["suite"])


async def publish_suite(
    dists_directory, db, package_info_provider, config, suite, gpg_context
):
    if not suite.debian_build:
        logging.info("%s is not a Debian suite", suite.name)
        return
    start_time = datetime.now()
    logging.info("Publishing %s", suite.name)
    suite_path = os.path.join(dists_directory, suite.name)
    os.makedirs(suite_path, exist_ok=True)
    await write_suite_files(
        suite_path,
        db,
        package_info_provider,
        suite,
        components=config.distribution.component,
        arches=ARCHES,
        origin=config.origin,
        gpg_context=gpg_context,
    )
    logging.info(
        "Done publishing %s (took %s)", suite.name, datetime.now() - start_time
    )
    last_publish_success.labels(suite=suite.name).set_to_current_time()
    last_publish_time[suite.name] = datetime.now()


class GeneratorManager(object):
    def __init__(self, dists_dir, db, config, package_info_provider, gpg_context):
        self.dists_dir = dists_dir
        self.db = db
        self.config = config
        self.package_info_provider = package_info_provider
        self.gpg_context = gpg_context
        self.generators = {}

    def trigger(self, suite):
        if isinstance(suite, str):
            suite_config = get_suite_config(self.config, suite)
        else:
            suite_config = suite
        if not suite_config.HasField('debian_build'):
            return
        try:
            task = self.generators[suite_config.name]
        except KeyError:
            pass
        else:
            if not task.done():
                return
        loop = asyncio.get_event_loop()
        self.generators[suite_config.name] = loop.create_task(
            publish_suite(
                self.dists_dir,
                self.db,
                self.package_info_provider,
                self.config,
                suite_config,
                self.gpg_context,
            )
        )


async def loop_publish(config, generator_manager):
    while True:
        for suite in config.suite:
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

    args = parser.parse_args()
    if not args.dists_directory:
        parser.print_usage()
        sys.exit(1)

    logging.basicConfig(level=logging.INFO)

    with open(args.config, "r") as f:
        config = read_config(f)

    os.makedirs(args.dists_directory, exist_ok=True)

    db = state.Database(config.database_location)

    artifact_manager = get_artifact_manager(config.artifact_location)

    gpg_context = gpg.Context()

    package_info_provider = PackageInfoProvider(artifact_manager)
    if args.cache_directory:
        os.makedirs(args.cache_directory, exist_ok=True)
        package_info_provider = CachingPackageInfoProvider(
            package_info_provider, args.cache_directory
        )

    generator_manager = GeneratorManager(
        args.dists_directory, db, config, package_info_provider, gpg_context
    )

    async with package_info_provider:
        loop = asyncio.get_event_loop()

        await asyncio.gather(
            loop.create_task(
                run_web_server(
                    args.listen_address,
                    args.port,
                    args.dists_directory,
                    config,
                    generator_manager,
                )
            ),
            loop.create_task(loop_publish(config, generator_manager)),
        )


if __name__ == "__main__":
    sys.exit(asyncio.run(main(sys.argv)))
