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

from aiohttp import ClientSession, ClientConnectorError
from debian.deb822 import Changes
from jinja2 import Environment, PackageLoader, select_autoescape
import json
import os
import urllib.parse

from janitor import SUITES


def format_duration(duration):
    total_seconds = duration.seconds
    seconds = duration.seconds % 60
    total_minutes = total_seconds // 60
    minutes = total_minutes % 60
    total_hours = total_minutes // 24
    hours = total_hours % 24
    days = total_hours // 24
    if days:
        return '%dd%02dh' % (days, hours)
    if hours:
        return '%dh%02dm' % (hours, minutes)
    return '%dm%02ds' % (minutes, seconds)


def format_timestamp(ts):
    return ts.isoformat(timespec='minutes')


async def get_vcs_type(publisher_url, package):
    url = urllib.parse.urljoin(publisher_url, 'vcs-type/%s' % package)
    async with ClientSession() as client:
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

env.globals.update(format_duration=format_duration)
env.globals.update(format_timestamp=format_timestamp)
env.globals.update(suites=SUITES)
env.globals.update(json_dumps=json.dumps)


def get_build_architecture():
    # TODO(jelmer): don't hardcode this
    return "amd64"


def highlight_diff(diff):
    from pygments import highlight
    from pygments.lexers.diff import DiffLexer
    from pygments.formatters import HtmlFormatter
    return highlight(diff, DiffLexer(stripnl=False), HtmlFormatter())


def open_changes_file(run, changes_name):
    path = os.path.join(
            os.path.dirname(__file__), '..', '..',
            "public_html", run.build_distribution, changes_name)
    return open(path, 'rb')


def changes_get_binaries(cf):
    changes = Changes(cf)
    return changes['Binary'].split(' ')
