#!/usr/bin/python3
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

from aiohttp import ClientSession
import asyncio
import json
from urllib.parse import urljoin


class AptlyError(Exception):

    def __init__(self, status, data):
        self.status = status
        self.data = data


class Aptly(object):

    def __init__(self, session, url):
        self.url = url
        self.session = session

    async def repos_list(self):
        url = urljoin(self.url, 'repos')
        with self.session.get(url) as resp:
            data = json.loads(await resp.text())
            if resp.status != 200:
                raise AptlyError(resp.status, data)
            return data

    async def repos_create(self, name, comment=None, default_distribution=None, default_component=None):
        headers = {'Content-Type': 'application/json'}
        url = urljoin(self.url, 'repos')
        data = {}
        if comment is not None:
            data['Comment'] = comment
        if default_distribution is not None:
            data['DefaultDistribution'] = default_distribution
        if default_component is not None:
            data['DefaultComponent'] = default_component
        with self.session.post(url, data=json.dumps(data), headers=headers) as resp:
            data = json.loads(await resp.text())
            if resp.status != 201:
                raise AptlyError(resp.status, data)
            return data

    async def repos_delete(self, name):
        url = urljoin(self.url, 'repos/%s' % name)
        with self.session.delete(url) as resp:
            data = json.loads(await resp.text())
            if resp.status != 200:
                raise AptlyError(resp.status, data)
            return data

    async def publish_update(self, prefix, suite, not_automatic=False):
        url = urljoin(self.url, 'publish/%s/%s' % (prefix, suite))
        headers = {'Content-Type': 'application/json'}
        data = {
            'Prefix': prefix,
            'SourceKind': 'local',
            'NotAutomatic': 'yes' if not_automatic else 'no',
            }
        with session.post(url, headers=headers, data=data) as resp:
            data = json.loads(await resp.text())
            if resp.status != 200:
                raise AptlyError(resp.status, data)
            return data
