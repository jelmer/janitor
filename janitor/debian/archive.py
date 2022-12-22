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
from functools import partial
import hashlib
import logging
import gzip
import json
import bz2
import io
import os
import re
import shutil
import sys
import tempfile
import time
from typing import List, Dict, Optional, Any
from email.utils import formatdate, parsedate_to_datetime
from datetime import datetime
from time import mktime

import uvloop

from aiohttp import web
from aiohttp.web_middlewares import normalize_path_middleware

from debian.deb822 import Release, Packages, Sources

import gpg
from gpg.constants.sig import mode as gpg_mode

from aiohttp_openmetrics import (
    Gauge,
    setup_metrics,
)

from .. import state
from ..artifacts import get_artifact_manager, ArtifactsMissing
from ..config import read_config, get_distribution, Campaign, get_campaign_config


TMP_PREFIX = 'janitor-apt'
DEFAULT_GCS_TIMEOUT = 60 * 30

last_publish_time: Dict[str, datetime] = {}


last_publish_success = Gauge(
    "last_suite_publish_success",
    "Last time publishing a suite succeeded",
    labelnames=("suite",),
)


logger = logging.getLogger('janitor.debian.archive')


routes = web.RouteTableDef()

# TODO(jelmer): Generate contents file


class PackageInfoProvider(object):

    async def __aenter__(self):
        return self

    async def __aexit__(self, exc_tp, exc_val, exc_tb):
        return False

    async def packages_for_run(self, run_id, suite_name, package, arch):
        raise NotImplementedError(self.packages_for_run)

    async def sources_for_run(self, run_id, suite_name, package):
        raise NotImplementedError(self.sources_for_run)


async def scan_packages(td, arch: Optional[str] = None):
    args = []
    if arch:
        args.extend(['-a', arch])
    proc = await asyncio.create_subprocess_exec(
        "dpkg-scanpackages", td, *args,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE)
    stdout, stderr = await proc.communicate()
    for para in Packages.iter_paragraphs(stdout, use_apt_pkg=False):
        yield para
    for line in stderr.splitlines(keepends=False):
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


async def scan_sources(td):
    proc = await asyncio.create_subprocess_exec(
        "dpkg-scansources", td,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE)
    stdout, stderr = await proc.communicate()
    for para in Sources.iter_paragraphs(stdout, use_apt_pkg=False):
        yield para
    for line in stderr.splitlines(keepends=False):
        if line.startswith(b'dpkg-scansources: '):
            line = line[len(b'dpkg-scansources: '):]
        if line.startswith(b'info: '):
            logging.debug('%s', line.rstrip(b'\n').decode())
        elif line.startswith(b'warning: '):
            logging.warning('%s', line.rstrip(b'\n').decode())
        elif line.startswith(b'error: '):
            logging.error('%s', line.rstrip(b'\n').decode())
        else:
            logging.info(
                'dpkg-scansources error: %s',
                line.rstrip(b'\n').decode())


class GeneratingPackageInfoProvider(PackageInfoProvider):
    def __init__(self, artifact_manager):
        self.artifact_manager = artifact_manager

    async def __aenter__(self):
        await self.artifact_manager.__aenter__()

    async def __aexit__(self, exc_tp, exc_val, exc_tb):
        await self.artifact_manager.__aexit__(exc_tp, exc_val, exc_tb)
        return False

    async def packages_for_run(self, run_id, suite_name, package, arch):
        with tempfile.TemporaryDirectory(prefix=TMP_PREFIX) as td:
            await self.artifact_manager.retrieve_artifacts(
                run_id, td, timeout=DEFAULT_GCS_TIMEOUT
            )
            async for para in scan_packages(td):
                para["Filename"] = os.path.join(
                    suite_name,
                    "pkg",
                    package,
                    run_id,
                    os.path.basename(para["Filename"]),
                )
                yield bytes(para)
                yield b"\n"

        await asyncio.sleep(0)

    async def sources_for_run(self, run_id, suite_name, package):
        with tempfile.TemporaryDirectory(prefix=TMP_PREFIX) as td:
            await self.artifact_manager.retrieve_artifacts(
                run_id, td, timeout=DEFAULT_GCS_TIMEOUT
            )
            async for para in scan_sources(td):
                para["Directory"] = os.path.join(
                    suite_name,
                    "pkg",
                    package,
                    run_id,
                )
                yield bytes(para)
                yield b"\n"

        await asyncio.sleep(0)


class DiskCachingPackageInfoProvider(PackageInfoProvider):
    def __init__(self, primary_info_provider, cache_directory):
        self.primary_info_provider = primary_info_provider
        self.cache_directory = cache_directory

    async def __aenter__(self):
        await self.primary_info_provider.__aenter__()

    async def __aexit__(self, exc_tp, exc_val, exc_tb):
        await self.primary_info_provider.__aexit__(exc_tp, exc_val, exc_tb)
        return False

    async def packages_for_run(self, run_id, suite_name, package, arch):
        cache_path = os.path.join(self.cache_directory, arch, run_id)
        os.makedirs(os.path.dirname(cache_path), exist_ok=True)
        try:
            with open(cache_path, "rb") as f:
                for chunk in f:
                    yield chunk
        except FileNotFoundError:
            logger.debug('Retrieving artifacts for %s/%s (%s)',
                         suite_name, package, run_id)
            with open(cache_path, "wb") as f:
                async for chunk in self.primary_info_provider.packages_for_run(
                    run_id, suite_name, package, arch=arch
                ):
                    f.write(chunk)
                    yield chunk

    async def sources_for_run(self, run_id, suite_name, package):
        cache_path = os.path.join(self.cache_directory, 'source', run_id)
        os.makedirs(os.path.dirname(cache_path), exist_ok=True)
        try:
            with open(cache_path, "rb") as f:
                for chunk in f:
                    yield chunk
        except FileNotFoundError:
            logger.debug('Retrieving artifacts for %s/%s (%s)',
                         suite_name, package, run_id)
            with open(cache_path, "wb") as f:
                async for chunk in self.primary_info_provider.sources_for_run(
                    run_id, suite_name, package
                ):
                    f.write(chunk)
                    yield chunk


async def retrieve_packages(info_provider, rows, suite_name, component, arch):
    logger.debug('Need to process %d rows for %s/%s/%s',
                 len(rows), suite_name, component, arch)
    for package, run_id, build_distribution, _build_version in rows:
        try:
            async for chunk in info_provider.packages_for_run(run_id, build_distribution, package, arch=arch):
                yield chunk
                await asyncio.sleep(0)
        except ArtifactsMissing:
            logger.warning("Artifacts missing for %s (%s), skipping", package, run_id)
            continue


async def retrieve_sources(info_provider, rows, suite_name, component):
    logger.debug('Need to process %d rows for %s/%s',
                 len(rows), suite_name, component)
    for package, run_id, build_distribution, _build_version in rows:
        try:
            async for chunk in info_provider.sources_for_run(run_id, build_distribution, package):
                yield chunk
                await asyncio.sleep(0)
        except ArtifactsMissing:
            logger.warning("Artifacts missing for %s (%s), skipping", package, run_id)
            continue


async def get_builds_for_suite(db, suite_name):
    async with db.acquire() as conn:
        return await conn.fetch(
            "SELECT DISTINCT ON (source) "
            "debian_build.source, debian_build.run_id, debian_build.distribution, "
            "debian_build.version FROM debian_build "
            "INNER JOIN run ON run.id = debian_build.run_id "
            "WHERE debian_build.distribution = $1 AND run.review_status != 'rejected' "
            "ORDER BY debian_build.source, debian_build.version DESC",
            suite_name,
        )


async def get_builds_for_changeset(db, cs_id):
    async with db.acquire() as conn:
        return await conn.fetch(
            "SELECT DISTINCT ON (source) "
            "debian_build.source, debian_build.run_id, debian_build.distribution, "
            "debian_build.version FROM debian_build "
            "INNER JOIN run ON run.id = debian_build.run_id "
            "WHERE run.change_set = $1 AND run.review_status != 'rejected' "
            "ORDER BY debian_build.source, debian_build.version DESC",
            cs_id,
        )


async def get_builds_for_run(db, run_id):
    async with db.acquire() as conn:
        return await conn.fetch(
            "SELECT DISTINCT ON (source) "
            "debian_build.source, debian_build.run_id, debian_build.distribution, "
            "debian_build.version FROM debian_build "
            "INNER JOIN run ON run.id = debian_build.run_id "
            "WHERE run.id = $1 AND run.review_status != 'rejected' "
            "ORDER BY debian_build.source, debian_build.version DESC",
            run_id,
        )


HASHES = {
    "MD5Sum": hashlib.md5,
    "SHA1": hashlib.sha1,
    "SHA256": hashlib.sha256,
    "SHA512": hashlib.sha512,
}


def cleanup_by_hash_files(base, number_to_keep):
    for h in HASHES:
        ages = []
        for entry in os.scandir(os.path.join(base, "by-hash", h)):
            ages.append((entry, time.time() - entry.stat().st_mtime))

        ages.sort(key=lambda k: k[1])

        for entry, _age in ages[number_to_keep:]:
            os.unlink(entry.path)


class HashedFileWriter(object):
    """File write wrapper that writes by-hash files."""

    def __init__(self, release, base, path, open=open):
        self.open = open
        self.release = release
        self.base = base
        self.path = path

    def __enter__(self):
        dir = os.path.join(self.base, os.path.dirname(self.path))
        os.makedirs(dir, exist_ok=True)
        fd, self._tmpf_path = tempfile.mkstemp(
            dir=dir, prefix=os.path.basename(self.path))
        self._tmpf = self.open(self._tmpf_path, 'wb')
        os.close(fd)
        return self

    def done(self):
        """Mark the file as done, close it and calculate size/hash."""
        self._tmpf.flush()
        self._tmpf.close()

        hashes = {n: kls() for (n, kls) in HASHES.items()}

        self.size = 0
        with open(self._tmpf_path, 'rb') as f:
            while True:
                chunk = f.read(io.DEFAULT_BUFFER_SIZE)
                if not chunk:
                    break
                for h in hashes.values():
                    h.update(chunk)
                self.size += len(chunk)

        d, n = os.path.split(self.path)
        for hn, v in hashes.items():
            os.makedirs(os.path.join(self.base, d, "by-hash", hn), exist_ok=True)
            hash_path = os.path.join(self.base, d, "by-hash", hn, v.hexdigest())
            shutil.copy(self._tmpf_path, hash_path)
            self.release.setdefault(hn, []).append({
                hn.lower(): v.hexdigest(),
                "size": self.size,
                "name": self.path
            })
            assert self.size == os.path.getsize(hash_path)

    def __exit__(self, exc_type, exc_val, exc_tb):
        if exc_type:
            return False
        os.rename(
            self._tmpf_path,
            os.path.join(self.base, self.path))
        assert self.size == os.path.getsize(os.path.join(self.base, self.path))
        return False

    def write(self, chunk):
        self._tmpf.write(chunk)


async def write_suite_files(
    base_path, *, get_packages, get_sources, suite_name, archive_description,
    components, arches, origin, gpg_context,
    timestamp: Optional[datetime] = None
):
    SUFFIXES: Dict[str, Any] = {
        "": open,
        ".gz": gzip.GzipFile,
        ".bz2": bz2.BZ2File,
    }

    if timestamp is None:
        timestamp = datetime.utcnow()
    stamp = mktime(timestamp.timetuple())

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
    r["Description"] = "Generated by the Janitor"
    r["Acquire-By-Hash"] = "yes"

    with ExitStack() as es:
        for component in components:
            logger.debug('Publishing component %s/%s', suite_name, component)
            component_dir = component
            os.makedirs(os.path.join(base_path, component_dir), exist_ok=True)
            for arch in arches:
                arch_dir = os.path.join(component_dir, f"binary-{arch}")
                os.makedirs(os.path.join(base_path, arch_dir), exist_ok=True)

                packages_path = os.path.join(arch_dir, "Packages")
                fs = []
                for suffix, fn in SUFFIXES.items():
                    fs.append(
                        es.enter_context(
                            HashedFileWriter(r, base_path, packages_path + suffix, fn)))
                async for chunk in get_packages(suite_name, component, arch):
                    for f in fs:
                        f.write(chunk)
                for f in fs:
                    f.done()
                cleanup_by_hash_files(
                    os.path.join(base_path, arch_dir),
                    4 * len(SUFFIXES))
                await asyncio.sleep(0)
            source_dir = os.path.join(component_dir, 'source')
            os.makedirs(os.path.join(base_path, source_dir), exist_ok=True)

            sources_path = os.path.join(source_dir, "Sources")
            fs = []
            for suffix, fn in SUFFIXES.items():
                fs.append(
                    es.enter_context(
                        HashedFileWriter(r, base_path, sources_path + suffix, fn)))
            async for chunk in get_sources(suite_name, component):
                for f in fs:
                    f.write(chunk)
            for f in fs:
                f.done()
            cleanup_by_hash_files(
                os.path.join(base_path, source_dir),
                4 * len(SUFFIXES))

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


@routes.post("/publish", name="publish")
async def handle_publish(request):
    post = await request.post()
    campaign = post.get("campaign")
    for campaign_config in request.app['config'].campaign:
        if not campaign_config.HasField('debian_build'):
            continue
        if campaign is not None and campaign != campaign_config.name:
            continue
        request.app['generator_manager'].trigger(campaign_config)

    return web.json_response({})


@routes.get("/last-publish", name="last-publish")
async def handle_last_publish(request):
    return web.json_response(
        {suite: dt.isoformat() for (suite, dt) in last_publish_time.items()}
    )


@routes.get("/health", name="health")
async def handle_health(request):
    return web.Response(text='ok')


@routes.get("/ready", name="ready")
async def handle_ready(request):
    missing = []
    for campaign_config in request.app['config'].campaign:
        if not campaign_config.HasField('debian_build'):
            continue
        if campaign_config.name not in last_publish_time:
            missing.append(campaign_config.name)
    status = ''.join(
        [f'{name}: {dt.isoformat()}\n'
         for (name, dt) in last_publish_time.items()])
    if missing:
        return web.Response(text=(
            'missing: %s' % ', '.join(missing) + '\n\n'
            'present:\n' + status), status=500)
    return web.Response(text=status)


@routes.get("/", name="index")
async def handle_index(request):
    return web.Response(text='')


@routes.get("/pgp_keys", name="pgp-keys")
async def handle_pgp_keys(request):
    pgp_keys = []
    for entry in list(request.app['gpg'].keylist(secret=True)):
        pgp_keys.append(request.app['gpg'].key_export_minimal(entry.fpr).decode())
    return web.json_response(pgp_keys)


async def serve_dists_release_file(request):
    path = os.path.join(
        request.app['dists_dir'],
        request.match_info['release'],
        request.match_info['file'])
    if not os.path.exists(path):
        raise web.HTTPNotFound(text='suite release files not generated yet')
    return web.FileResponse(path)


async def serve_dists_component_file(request):
    path = os.path.join(
        request.app['dists_dir'],
        request.match_info['release'],
        request.match_info['component'],
        request.match_info['arch'],
        request.match_info['file'])
    if not os.path.exists(path):
        raise web.HTTPNotFound(text='suite component files not generated yet')
    return web.FileResponse(path)


async def serve_dists_component_hash_file(request):
    path = os.path.join(
        request.app['dists_dir'],
        request.match_info['release'],
        request.match_info['component'],
        request.match_info['arch'],
        "by-hash",
        request.match_info['hash_type'],
        request.match_info['hash'])
    if not os.path.exists(path):
        raise web.HTTPNotFound(text='suite by-hash file not present')
    return web.FileResponse(path)


async def refresh_on_demand_dists(
        dists_dir, db, config, package_info_provider, gpg_context, kind, id):
    os.makedirs(os.path.join(dists_dir, kind, id), exist_ok=True)
    release_path = os.path.join(dists_dir, kind, id, 'Release')
    try:
        with open(release_path, 'r') as f:
            release = Release(f)
    except FileNotFoundError:
        stamp = None
    else:
        stamp = parsedate_to_datetime(release["Date"])
    async with db.acquire() as conn:
        if kind == 'run':
            row = await conn.fetchrow(
                'SELECT suite, max(finish_time) FROM run WHERE id = $1', id)
            if row is None:
                raise web.HTTPNotFound(text=f"no such run: {id}")
            campaign, max_finish_time = row
            builds = await get_builds_for_run(db, id)
            campaign_config = get_campaign_config(config, campaign)
        elif kind == 'cs':
            campaign = await conn.fetchval(
                'SELECT campaign FROM change_set WHERE id = $1', id)
            if campaign is None:
                raise web.HTTPNotFound(text=f"no such changeset: {id}")
            max_finish_time = await conn.fetchval(
                'SELECT max(finish_time) FROM run WHERE change_set = $1', id)
            builds = await get_builds_for_changeset(db, id)
            description = f"Change set {id}"
            campaign_config = get_campaign_config(config, campaign)
        else:
            try:
                campaign_config = get_campaign_config(config, kind)
            except KeyError as e:
                raise web.HTTPNotFound(text=f"No such campaign: {kind}") from e
            cs_id = await conn.fetchval(
                "SELECT run.change_set FROM run "
                "INNER JOIN change_set ON change_set.id = run.change_set "
                "WHERE run.suite = $1 AND run.package = $2 "
                "AND change_set.state in ('working', 'ready', 'publishing', 'done') AND "
                "run.result_code = 'success' "
                "ORDER BY run.finish_time DESC", kind, id)
            if cs_id is None:
                if not (await conn.fetchrow("SELECT 1 FROM debian_build WHERE source = $1", id)):
                    raise web.HTTPNotFound(text=f"No such source package: {id}")
            max_finish_time = await conn.fetchval(
                'SELECT max(finish_time) FROM run WHERE change_set = $1', cs_id)
            builds = await get_builds_for_changeset(db, cs_id)
            description = f"Campaign {kind} for {id}"
    if stamp is not None and max_finish_time and max_finish_time.astimezone() < stamp:
        return
    logging.info("Generating metadata for %s/%s", kind, id)
    distribution = get_distribution(
        config, campaign_config.debian_build.base_distribution)
    await write_suite_files(
        os.path.join(dists_dir, kind, id),
        get_packages=partial(retrieve_packages, package_info_provider, builds),
        get_sources=partial(retrieve_sources, package_info_provider, builds),
        suite_name=f"{kind}/{id}",
        archive_description=description,
        components=distribution.component,
        arches=ARCHES,
        origin=config.origin,
        gpg_context=gpg_context,
    )


async def serve_on_demand_dists_release_file(request):
    await refresh_on_demand_dists(
        request.app['dists_dir'],
        request.app['db'],
        request.app['config'],
        request.app['generator_manager'].package_info_provider,
        request.app['gpg'],
        request.match_info['kind'],
        request.match_info['id'])

    path = os.path.join(
        request.app['dists_dir'],
        request.match_info['kind'],
        request.match_info['id'],
        request.match_info['file'])
    if not os.path.exists(path):
        raise web.HTTPNotFound()
    return web.FileResponse(path)


async def serve_on_demand_dists_component_file(request):
    await refresh_on_demand_dists(
        request.app['dists_dir'],
        request.app['db'],
        request.app['config'],
        request.app['generator_manager'].package_info_provider,
        request.app['gpg'],
        request.match_info['kind'],
        request.match_info['id'])

    path = os.path.join(
        request.app['dists_dir'],
        request.match_info['kind'],
        request.match_info['id'],
        request.match_info['component'],
        request.match_info['arch'],
        request.match_info['file'])
    if not os.path.exists(path):
        raise web.HTTPNotFound()
    return web.FileResponse(path)


async def serve_on_demand_dists_component_hash_file(request):
    await refresh_on_demand_dists(
        request.app['dists_dir'],
        request.app['db'],
        request.app['config'],
        request.app['generator_manager'].package_info_provider,
        request.app['gpg'],
        request.match_info['kind'],
        request.match_info['id'])

    path = os.path.join(
        request.app['dists_dir'],
        request.match_info['kind'],
        request.match_info['id'],
        request.match_info['component'],
        request.match_info['arch'],
        'by-hash',
        request.match_info['hash_type'],
        request.match_info['hash'])
    if not os.path.exists(path):
        raise web.HTTPNotFound()
    return web.FileResponse(path)


async def create_app(generator_manager, config, dists_dir, db):
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[
        trailing_slash_redirect, state.asyncpg_error_middleware])
    app['gpg'] = gpg.Context(armor=True)
    app['dists_dir'] = dists_dir
    app['config'] = config
    app['generator_manager'] = generator_manager
    app['db'] = db
    setup_metrics(app)
    app.router.add_routes(routes)
    app.router.add_get(
        "/dists/{release}/{file:InRelease|Release.gpg|Release}",
        serve_dists_release_file)
    app.router.add_get(
        "/dists/{release}/{component}/{arch}/"
        r"{file:(Packages|Sources)(|\..*)}",
        serve_dists_component_file)
    app.router.add_get(
        "/dists/{release}/{component}/{arch}/"
        r"by-hash/{hash_type}/{hash}",
        serve_dists_component_hash_file)

    CAMPAIGNS_REGEX = "|".join(re.escape(c.name) for c in config.campaign)
    app.router.add_get(
        "/dists/{kind:cs|run|" + CAMPAIGNS_REGEX + "}/{id}/{file:InRelease|Release.gpg|Release}",
        serve_on_demand_dists_release_file)
    app.router.add_get(
        "/dists/{kind:cs|run|" + CAMPAIGNS_REGEX + "}/{id}/{component}/{arch}/"
        r"{file:(Packages|Sources)(|\..*)}",
        serve_on_demand_dists_component_file)
    app.router.add_get(
        "/dists/{kind:cs|run|" + CAMPAIGNS_REGEX + "}/{id}/{component}/{arch}/"
        r"by-hash/{hash_type}/{hash}",
        serve_on_demand_dists_component_hash_file)
    return app


async def run_web_server(listen_addr, port, dists_dir, config, db, generator_manager, tracer):
    app = await create_app(generator_manager, config, dists_dir, db)
    aiozipkin.setup(app, tracer)
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()


async def listen_to_runner(redis, generator_manager):
    async def handle_result_message(msg):
        result = json.loads(msg['data'])
        if result["code"] != "success":
            return
        if result['target']['name'] != 'debian':
            return
        campaign = get_campaign_config(generator_manager.config, result["campaign"])
        if campaign:
            generator_manager.trigger(campaign)

    try:
        async with redis.pubsub(ignore_subscribe_messages=True) as ch:
            await ch.subscribe('result', result=handle_result_message)
            await ch.run()
    finally:
        await redis.close()


async def publish_suite(
    dists_directory, db, package_info_provider, config, suite, gpg_context
):
    if not suite.HasField('debian_build'):
        logger.info("%s is not a Debian suite", suite.name)
        return
    start_time = datetime.utcnow()
    logger.info("Publishing %s", suite.name)
    distribution = get_distribution(config, suite.debian_build.base_distribution)
    suite_path = os.path.join(dists_directory, suite.name)
    builds = await get_builds_for_suite(db, suite.name)
    await write_suite_files(
        suite_path,
        get_packages=partial(retrieve_packages, package_info_provider, builds),
        get_sources=partial(retrieve_sources, package_info_provider, builds),
        suite_name=suite.name,
        archive_description=suite.debian_build.archive_description,
        components=distribution.component,
        arches=ARCHES,
        origin=config.origin,
        gpg_context=gpg_context)

    logger.info(
        "Done publishing %s (took %s)", suite.name,
        datetime.utcnow() - start_time)
    last_publish_success.labels(suite=suite.name).set_to_current_time()
    last_publish_time[suite.name] = datetime.utcnow()


def create_background_task(fn, title):
    loop = asyncio.get_event_loop()
    task = loop.create_task(fn)

    def log_result(future):
        try:
            future.result()
        except BaseException:
            logging.exception('%s failed', title)
            raise
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
            ), f'publish {campaign_config.name}'
        )


async def loop_publish(config, generator_manager):
    while True:
        for suite in config.campaign:
            generator_manager.trigger(suite)
        # every 12 hours
        await asyncio.sleep(60 * 60 * 12)


async def main(argv=None):
    import argparse
    from redis.asyncio import Redis

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

    endpoint = aiozipkin.create_endpoint(
        "janitor.debian.archive", ipv4=args.listen_address, port=args.port)
    if config.zipkin_address:
        tracer = await aiozipkin.create(
            config.zipkin_address, endpoint, sample_rate=0.1)
    else:
        tracer = await aiozipkin.create_custom(endpoint)
    trace_configs = [aiozipkin.make_trace_config(tracer)]

    artifact_manager = get_artifact_manager(
        config.artifact_location, trace_configs=trace_configs)

    gpg_context = gpg.Context()

    package_info_provider: PackageInfoProvider
    package_info_provider = GeneratingPackageInfoProvider(artifact_manager)
    if args.cache_directory:
        os.makedirs(args.cache_directory, exist_ok=True)
        package_info_provider = DiskCachingPackageInfoProvider(
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
                db,
                generator_manager,
                tracer,
            )
        ),
        create_background_task(
            loop_publish(config, generator_manager), 'regenerate suites'),
    ]

    redis = Redis.from_url(config.redis_location)

    tasks.append(loop.create_task(
        listen_to_runner(redis, generator_manager)))

    async with package_info_provider:
        await asyncio.gather(*tasks)


if __name__ == "__main__":
    asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())
    sys.exit(asyncio.run(main(sys.argv)))
