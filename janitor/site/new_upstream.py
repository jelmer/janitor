#!/usr/bin/python3

import aiozipkin
from aiohttp import web, ClientConnectorError
import asyncpg
from datetime import datetime
import urllib.parse

from . import html_template, render_template_for_request
from ..config import get_suite_config


async def get_proposals(conn: asyncpg.Connection, package, suite):
    return await conn.fetch("""
SELECT
    DISTINCT ON (merge_proposal.url)
    merge_proposal.url, merge_proposal.status
FROM
    merge_proposal
LEFT JOIN run
ON merge_proposal.revision = run.revision AND run.result_code = 'success'
WHERE merge_proposal.package = $1 AND suite = $2
ORDER BY merge_proposal.url, run.finish_time DESC
""", package, suite)


async def generate_candidates(db, suite):
    async with db.acquire() as conn:
        query = """
SELECT
  candidate.package AS package,
  candidate.suite AS suite,
  candidate.context AS version,
  candidate.value AS value,
  candidate.success_chance AS success_chance,
  package.archive_version AS archive_version
FROM candidate
INNER JOIN package on package.name = candidate.package
WHERE NOT package.removed AND suite = $1
"""
        candidates = await conn.fetch(query, suite)
    candidates.sort(key=lambda row: row['package'])
    return {"candidates": candidates, "suite": suite}


@html_template(
    "new-upstream-package.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_new_upstream_pkg(request):
    from .common import generate_pkg_context

    suite = request.match_info["suite"]
    pkg = request.match_info["pkg"]
    run_id = request.match_info.get("run_id")
    return await generate_pkg_context(
        request.app.database,
        request.app.config,
        suite,
        request.app.policy,
        request.app.http_client_session,
        request.app.differ_url,
        request.app.vcs_store_url,
        pkg,
        aiozipkin.request_span(request),
        run_id)


@html_template(
    "new-upstream-candidates.html", headers={"Cache-Control": "max-age=600"})
async def handle_new_upstream_candidates(request):
    from .new_upstream import generate_candidates

    suite = request.match_info["suite"]
    return await generate_candidates(request.app.database, suite)


@html_template("fresh-builds.html", headers={"Cache-Control": "max-age=60"})
async def handle_fresh_builds(request):
    from .apt_repo import get_published_packages
    archive_version = {}
    suite_version = {}
    sources = set()
    SUITES = ["fresh-releases", "fresh-snapshots"]
    url = urllib.parse.urljoin(request.app.archiver_url, "last-publish")
    try:
        async with request.app.http_client_session.get(url) as resp:
            if resp.status == 200:
                last_publish_time = {
                    suite: datetime.fromisoformat(v)
                    for suite, v in (await resp.json()).items()
                }
            else:
                last_publish_time = {}
    except ClientConnectorError:
        last_publish_time = {}

    async with request.app.database.acquire() as conn:
        for suite in SUITES:
            for name, jv, av in await get_published_packages(conn, suite):
                sources.add(name)
                archive_version[name] = av
                suite_version.setdefault(suite, {})[name] = jv
        return {
            "base_distribution": get_suite_config(
                request.app.config, SUITES[0]
            ).debian_build.base_distribution,
            "archive_version": archive_version,
            "suite_version": suite_version,
            "sources": sources,
            "suites": SUITES,
            "last_publish_time": last_publish_time,
        }


async def handle_fresh(request):
    return web.HTTPPermanentRedirect("/fresh-builds")


async def handle_apt_repo(request):
    suite = request.match_info["suite"]
    from .apt_repo import get_published_packages

    async with request.app.database.acquire() as conn:
        vs = {
            "packages": await get_published_packages(conn, suite),
            "suite": suite,
            "suite_config": get_suite_config(request.app.config, suite),
        }
        text = await render_template_for_request(suite + ".html", request, vs)
        return web.Response(
            content_type="text/html",
            text=text,
            headers={"Cache-Control": "max-age=60"},
        )


def register_new_upstream_endpoints(router):
    NEW_UPSTREAM_REGEX = "fresh-(releases|snapshots)"
    router.add_get(
        "/{suite:%s}/" % NEW_UPSTREAM_REGEX, handle_apt_repo, name="new-upstream-start"
    )
    router.add_get("/fresh-builds", handle_fresh_builds, name="fresh-builds")
    router.add_get("/fresh", handle_fresh, name="fresh")
    router.add_get(
        "/{suite:%s}/pkg/{pkg}/" % NEW_UPSTREAM_REGEX,
        handle_new_upstream_pkg,
        name="new-upstream-package",
    )
    router.add_get(
        "/{suite:%s}/pkg/{pkg}/{run_id}" % NEW_UPSTREAM_REGEX,
        handle_new_upstream_pkg,
        name="new-upstream-run",
    )
    router.add_get(
        "/{suite:%s}/candidates" % NEW_UPSTREAM_REGEX,
        handle_new_upstream_candidates,
        name="new-upstream-candidates",
    )
