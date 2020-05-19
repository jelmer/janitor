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
import urllib.parse

from janitor import state
from janitor.schedule import TRANSIENT_ERROR_RESULT_CODES
from janitor.vcs import (
    CACHE_URL_BZR,
    CACHE_URL_GIT,
)


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
    return ts.isoformat(timespec='minutes')


async def get_vcs_type(client, publisher_url, package):
    url = urllib.parse.urljoin(publisher_url, 'vcs-type/%s' % package)
    try:
        async with client.get(url) as resp:
            if resp.status == 200:
                ret = (await resp.read()).decode('utf-8', 'replace')
                if ret == "":
                    ret = None
            else:
                ret = None
        return ret
    except ClientConnectorError as e:
        return 'Unable to retrieve diff; error %s' % e


env = Environment(
    loader=PackageLoader('janitor.site', 'templates'),
    autoescape=select_autoescape(['html', 'xml']),
    enable_async=True,
)


def highlight_diff(diff):
    from pygments import highlight
    from pygments.lexers.diff import DiffLexer
    from pygments.formatters import HtmlFormatter
    return highlight(diff, DiffLexer(stripnl=False), HtmlFormatter())


def classify_result_code(result_code):
    if result_code in ('success', 'nothing-to-do', 'nothing-new-to-do'):
        return result_code
    if result_code in TRANSIENT_ERROR_RESULT_CODES:
        return 'transient-failure'
    return 'failure'


env.globals.update(format_duration=format_duration)
env.globals.update(format_timestamp=format_timestamp)
env.globals.update(cache_url_git=CACHE_URL_GIT)
env.globals.update(cache_url_bzr=CACHE_URL_BZR)
env.globals.update(enumerate=enumerate)
env.globals.update(highlight_diff=highlight_diff)
env.globals.update(classify_result_code=classify_result_code)


class DebdiffRetrievalError(Exception):
    """Error occurred while retrieving debdiff."""


async def get_archive_diff(client, archiver_url, run, unchanged_run,
                           kind, accept=None, filter_boring=False):
    if unchanged_run.build_version is None:
        raise DebdiffRetrievalError('unchanged run not built')
    if run.build_version is None:
        raise DebdiffRetrievalError('run not built')
    if kind not in ('debdiff', 'diffoscope'):
        raise DebdiffRetrievalError('invalid diff kind %r' % kind)
    url = urllib.parse.urljoin(archiver_url, kind)
    payload = {
        'old_suite': 'unchanged',
        'new_suite': run.suite,
        'source': unchanged_run.package,
        'old_version': unchanged_run.build_version,
        'new_version': run.build_version,
        'jquery_url': 'https://janitor.debian.org/_static/jquery.js',
        'css_url': None,
    }
    if filter_boring:
        payload["filter_boring"] = "yes"
    headers = {}
    if accept:
        headers['Accept'] = (
            ', '.join(accept)
            if isinstance(accept, list)
            else accept)
    try:
        async with client.post(url, data=payload, headers=headers) as resp:
            if resp.status == 200:
                return await resp.read(), resp.content_type
            elif resp.status == 404:
                raise FileNotFoundError
            else:
                raise DebdiffRetrievalError(
                    'Unable to get debdiff: %s' % await resp.text())
    except ClientConnectorError as e:
        raise DebdiffRetrievalError(str(e))


def is_admin(request):
    return request.debsso_email == 'jelmer@debian.org'


def check_admin(request):
    if not is_admin(request):
        return web.Response(text='Unauthorized', status=401)


async def is_worker(db, request):
    auth_header = request.headers.get(aiohttp.hdrs.AUTHORIZATION)
    if not auth_header:
        return None
    auth = BasicAuth.decode(auth_header=auth_header)
    async with db.acquire() as conn:
        if await state.check_worker_credentials(
                conn, auth.login, auth.password):
            return auth.login
    return None


async def check_worker_creds(db, request):
    auth_header = request.headers.get(aiohttp.hdrs.AUTHORIZATION)
    if not auth_header:
        raise web.HTTPUnauthorized(body='worker login required')
    login = await is_worker(db, request)
    if not login:
        raise web.HTTPUnauthorized(body='worker login required')
    return login
