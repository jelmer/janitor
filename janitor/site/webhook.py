#!/usr/bin/python
# Copyright (C) 2019-2021 Jelmer Vernooij <jelmer@jelmer.uk>
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

from typing import List, Optional, Tuple
import json

from aiohttp import web
import asyncpg

from breezy.git.refs import ref_to_branch_name
from breezy.git.urls import git_url_to_bzr_url

from .. import state
from ..schedule import (
    do_schedule,
)


def is_webhook_request(request):
    return ("X-Gitlab-Event" in request.headers or
            "X-GitHub-Event" in request.headers or
            "X-Launchpad-Event-Type" in request.headers)


def get_branch_urls_from_github_webhook(body):
    url_keys = ["clone_url", "html_url", "git_url", "ssh_url"]
    urls = []
    for url_key in url_keys:
        url = body["repository"][url_key]
        urls.append(git_url_to_bzr_url(url, ref=body["ref"].encode()))
        try:
            branch_name = ref_to_branch_name(body["ref"].encode())
        except ValueError:
            pass
        else:
            if branch_name == body["repository"].get("default_branch"):
                urls.append(git_url_to_bzr_url(url))
    return urls


def get_bzr_branch_urls_from_launchpad_webhook(body):
    return [
        base + body['bzr_branch_path']
        for base in [
            'https://code.launchpad.net/',
            'https://bazaar.launchpad.net/',
            'lp:']]


def get_git_branch_urls_from_launchpad_webhook(body):
    path = body['git_repository_path']
    base_urls = [
        'https://git.launchpad.net/' + path,
        'git+ssh://git.launchpad.net/' + path]
    urls = []
    for base_url in base_urls:
        for ref in body['ref_changes']:
            urls.append(git_url_to_bzr_url(base_url, ref=body["ref"].encode()))
        # No idea what the default branch is, so let's trigger on everything
        # for now:
        urls.append(git_url_to_bzr_url(base_url))
    return urls


def get_branch_urls_from_gitlab_webhook(body):
    url_keys = ["git_http_url", "git_ssh_url"]
    urls = []
    for url_key in url_keys:
        urls.append(git_url_to_bzr_url(url_key, ref=body["ref"].encode()))
        try:
            branch_name = ref_to_branch_name(body["ref"].encode())
        except ValueError:
            pass
        else:
            if branch_name == body['project'].get('default_branch'):
                urls.append(git_url_to_bzr_url(url_key))
    return urls


async def get_package_by_branch_url(
    conn: asyncpg.Connection, branch_urls: List[str]
) -> Optional[Tuple[str, str]]:
    query = """
SELECT
  name, branch_url
FROM
  package
WHERE
  branch_url = ANY($1::text[])
"""
    candidates = []
    for url in branch_urls:
        candidates.extend([
            url.rstrip('/'),
            url.rstrip('/')+'/'])
    return await conn.fetchrow(query, candidates)


async def get_package_by_upstream_branch_url(
    conn: asyncpg.Connection, upstream_branch_urls: List[str]
) -> Optional[Tuple[str, str]]:
    query = """
SELECT
  name, branch_url
FROM
  package
WHERE
  name IN (
    SELECT package FROM upstream_branch_urls WHERE url = ANY($1::text[]))
"""
    candidates = []
    for url in upstream_branch_urls:
        candidates.extend([
            url.rstrip('/'),
            url.rstrip('/')+'/',
        ])
    return await conn.fetchrow(query, candidates)


async def process_webhook(request, db):
    if request.content_type == "application/json":
        body = await request.json()
    elif request.content_type == "application/x-www-form-urlencoded":
        post = await request.post()
        body = json.loads(post["payload"])
    else:
        return web.Response(
            status=415, text="Invalid content type %s" % request.content_type
        )
    async with db.acquire() as conn:
        if "X-Gitlab-Event" in request.headers:
            if request.headers["X-Gitlab-Event"] != "Push Hook":
                return web.json_response({}, status=200)
            urls = get_branch_urls_from_gitlab_webhook(body)
            # TODO(jelmer: If nothing found, then maybe fall back to
            # urlutils.basename(body['project']['path_with_namespace'])?
        elif "X-GitHub-Event" in request.headers:
            if request.headers["X-GitHub-Event"] not in ("push", ):
                return web.json_response({}, status=200)
            urls = get_branch_urls_from_github_webhook(body)
        elif "X-Launchpad-Event-Type" in request.headers:
            if request.headers["X-Launchpad-Event-Type"] not in ("bzr:push:0.1", "git:push:0.1"):
                return web.json_response({}, status=200)
            if request.headers["X-Launchpad-Event-Type"] == 'bzr:push:0.1':
                urls = get_bzr_branch_urls_from_launchpad_webhook(body)
            elif request.headers["X-Launchpad-Event-Type"] == 'git:push:0.1':
                urls = get_git_branch_urls_from_launchpad_webhook(body)
            else:
                return web.json_response({}, status=200)
        else:
            return web.Response(status=400, text="Unrecognized webhook")

        rescheduled = {}
        package = await get_package_by_branch_url(conn, urls)
        if package is not None:
            requestor = "Push hook for %s" % package['branch_url']
            for suite in await state.iter_publishable_suites(
                    conn, package['name']
            ):
                if suite not in rescheduled.get(package['name'], []):
                    await do_schedule(
                        conn, package['name'], suite, requestor=requestor, bucket="webhook"
                    )
                    rescheduled.setdefault(package['name'], []).append(suite)

        package = await get_package_by_upstream_branch_url(
            conn, urls)
        if package is not None:
            requestor = "Push hook for %s" % package['branch_url']
            for suite in await state.iter_publishable_suites(
                conn, package['name']
            ):
                if suite not in ("fresh-releases", "fresh-snapshots"):
                    continue
                if suite not in rescheduled.get(package['name'], []):
                    await do_schedule(
                        conn, package['name'], suite, requestor=requestor, bucket="webhook"
                    )
                    rescheduled.setdefault(package['name'], []).append(suite)

        return web.json_response({"rescheduled": rescheduled, "urls": urls})
