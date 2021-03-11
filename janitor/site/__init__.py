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

import aiohttp
from aiohttp import ClientConnectorError, web, BasicAuth
from jinja2 import Environment, PackageLoader, select_autoescape
from typing import Optional
import urllib.parse
from yarl import URL

from janitor import state
from janitor.config import Config
from janitor.schedule import TRANSIENT_ERROR_RESULT_CODES
from janitor.vcs import RemoteVcsManager


def json_chart_data(max_age=None):
    if max_age is not None:
        headers = {"Cache-Control": "max-age=%d" % max_age}
    else:
        headers = {}

    def decorator(fn):
        async def handle(request):
            async with request.app.database.acquire() as conn:
                return web.json_response(await fn(request, conn), headers=headers)

        return handle

    return decorator


def update_vars_from_request(vs, request):
    vs["is_admin"] = is_admin(request)
    vs["is_qa_reviewer"] = is_qa_reviewer(request)
    vs["user"] = request.user
    vs["rel_url"] = request.rel_url
    vs["suites"] = request.app.config.suite
    vs["site_name"] = request.app.config.instance_name or "Debian Janitor"
    vs["openid_configured"] = bool(getattr(request.app, "openid_config", None))
    if request.app.external_url is not None:
        vs["url"] = request.app.external_url.join(request.rel_url)
        vs["vcs_manager"] = RemoteVcsManager(str(request.app.external_url))
    else:
        vs["url"] = request.url
        vs["vcs_manager"] = RemoteVcsManager(str(request.url.with_path("/")))


async def render_template_for_request(templatename, request, vs):
    update_vars_from_request(vs, request)
    template = env.get_template(templatename)
    return await template.render_async(**vs)


def html_template(template_name, headers={}):
    def decorator(fn):
        async def handle(request):
            template = request.app.jinja_env.get_template(template_name)
            vs = await fn(request)
            if isinstance(vs, web.Response):
                return vs
            update_vars_from_request(vs, request)
            text = await template.render_async(**vs)
            return web.Response(content_type="text/html", text=text, headers=headers)

        return handle

    return decorator


def format_duration(duration):
    weeks = duration.days // 7
    days = duration.days % 7
    if weeks:
        return "%dw%dd" % (weeks, days)
    if duration.days:
        return "%dd%dh" % (duration.days, duration.seconds // (60 * 60))
    hours = duration.seconds // (60 * 60)
    seconds = duration.seconds % (60 * 60)
    minutes = seconds // 60
    seconds %= 60
    if hours:
        return "%dh%dm" % (hours, minutes)
    if minutes:
        return "%dm%ds" % (minutes, seconds)
    return "%ds" % seconds


def format_timestamp(ts):
    return ts.isoformat(timespec="minutes")


async def get_vcs_type(client, vcs_store_url, package):
    url = urllib.parse.urljoin(vcs_store_url, "vcs-type/%s" % package)
    try:
        async with client.get(url) as resp:
            if resp.status == 200:
                ret = (await resp.read()).decode("utf-8", "replace")
                if ret == "":
                    ret = None
            else:
                ret = None
        return ret
    except ClientConnectorError as e:
        return "Unable to retrieve diff; error %s" % e


env = Environment(
    loader=PackageLoader("janitor.site", "templates"),
    autoescape=select_autoescape(["html", "xml"]),
    enable_async=True,
)


def highlight_diff(diff):
    from pygments import highlight
    from pygments.lexers.diff import DiffLexer
    from pygments.formatters import HtmlFormatter

    return highlight(diff, DiffLexer(stripnl=False), HtmlFormatter())


def classify_result_code(result_code):
    if result_code in ("success", "nothing-to-do", "nothing-new-to-do"):
        return result_code
    if result_code in TRANSIENT_ERROR_RESULT_CODES:
        return "transient-failure"
    return "failure"


env.globals.update(format_duration=format_duration)
env.globals.update(format_timestamp=format_timestamp)
env.globals.update(enumerate=enumerate)
env.globals.update(highlight_diff=highlight_diff)
env.globals.update(classify_result_code=classify_result_code)
env.globals.update(URL=URL)


class DebdiffRetrievalError(Exception):
    """Error occurred while retrieving debdiff."""


class BuildDiffUnavailable(Exception):
    """The build diff is not available."""

    def __init__(self, unavailable_run):
        self.unavailable_run = unavailable_run


async def get_archive_diff(
    client, differ_url, run, unchanged_run, kind, accept=None, filter_boring=False
):
    if not unchanged_run.has_artifacts():
        raise DebdiffRetrievalError("unchanged run not successful")
    if not run.has_artifacts():
        raise DebdiffRetrievalError("run not successful")
    if kind not in ("debdiff", "diffoscope"):
        raise DebdiffRetrievalError("invalid diff kind %r" % kind)
    url = urllib.parse.urljoin(
        differ_url, "%s/%s/%s" % (kind, unchanged_run.id, run.id)
    )
    params = {
        "jquery_url": "https://janitor.debian.org/_static/jquery.js",
    }
    # TODO(jelmer): Set css_url
    if filter_boring:
        params["filter_boring"] = "yes"
    headers = {}
    if accept:
        headers["Accept"] = ", ".join(accept) if isinstance(accept, list) else accept
    try:
        async with client.get(url, params=params, headers=headers) as resp:
            if resp.status == 200:
                return await resp.read(), resp.content_type
            elif resp.status == 404:
                if resp.headers.get("unavailable_run_id") == unchanged_run.id:
                    raise BuildDiffUnavailable(unchanged_run)
                else:
                    raise BuildDiffUnavailable(run)
            else:
                raise DebdiffRetrievalError(
                    "Unable to get debdiff: %s" % await resp.text()
                )
    except ClientConnectorError as e:
        raise DebdiffRetrievalError(str(e))


def is_admin(request: web.Request) -> bool:
    if not request.user:
        return False
    admin_group = request.app.config.oauth2_provider.admin_group
    if admin_group is None:
        return True
    return admin_group in request.user["groups"]


def check_qa_reviewer(request: web.Request) -> None:
    if not is_qa_reviewer(request):
        raise web.HTTPUnauthorized()


def is_qa_reviewer(request: web.Request) -> bool:
    if not request.user:
        return False
    qa_reviewer_group = request.app.config.oauth2_provider.qa_reviewer_group
    if qa_reviewer_group is None:
        return True
    return qa_reviewer_group in request.user["groups"]


def check_admin(request: web.Request) -> None:
    if not is_admin(request):
        raise web.HTTPUnauthorized()


async def is_worker(db, request: web.Request) -> Optional[str]:
    auth_header = request.headers.get(aiohttp.hdrs.AUTHORIZATION)
    if not auth_header:
        return None
    auth = BasicAuth.decode(auth_header=auth_header)
    async with db.acquire() as conn:
        if await state.check_worker_credentials(conn, auth.login, auth.password):
            return auth.login
    return None


async def check_worker_creds(db, request: web.Request) -> Optional[str]:
    auth_header = request.headers.get(aiohttp.hdrs.AUTHORIZATION)
    if not auth_header:
        raise web.HTTPUnauthorized(
            text="worker login required",
            headers={"WWW-Authenticate": 'Basic Realm="Debian Janitor"'},
        )
    login = await is_worker(db, request)
    if not login:
        raise web.HTTPUnauthorized(
            text="worker login required",
            headers={"WWW-Authenticate": 'Basic Realm="Debian Janitor"'},
        )

    return login


def tracker_url(config: Config, pkg: str) -> Optional[str]:
    if config.distribution.tracker_url:
        return "%s/%s" % (config.distribution.tracker_url.rstrip("/"), pkg)
    return None


def iter_accept(request):
    return [h.strip() for h in request.headers.get("Accept", "*/*").split(",")]
