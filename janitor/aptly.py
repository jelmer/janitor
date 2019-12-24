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

from aiohttp import MultipartWriter
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
        async with self.session.get(url) as resp:
            data = json.loads(await resp.text())
            if resp.status != 200:
                raise AptlyError(resp.status, data)
            return data

    async def repos_create(self, name, comment=None, default_distribution=None, default_component=None):
        headers = {'Content-Type': 'application/json'}
        url = urljoin(self.url, 'repos')
        data = {'Name': name}
        if comment is not None:
            data['Comment'] = comment
        if default_distribution is not None:
            data['DefaultDistribution'] = default_distribution
        if default_component is not None:
            data['DefaultComponent'] = default_component
        async with self.session.post(url, json=data, headers=headers) as resp:
            data = json.loads(await resp.text())
            if resp.status != 201:
                raise AptlyError(resp.status, data)
            return data

    async def repos_delete(self, name):
        url = urljoin(self.url, 'repos/%s' % name)
        async with self.session.delete(url) as resp:
            data = json.loads(await resp.text())
            if resp.status != 200:
                raise AptlyError(resp.status, data)
            return data

    async def repos_include(self, name, dirname, no_remove_files=None,
                            force_replace=None, ignore_signature=None,
                            accept_unsigned=None):
        url = urljoin(self.url, 'repos/%s/include/%s' % (name, dirname))
        data = {}
        if no_remove_files is not None:
            data['noRemoveFiles'] = '1' if no_remove_files else '0'
        if force_replace is not None:
            data['forceReplace'] = '1' if force_replace else '0'
        if ignore_signature is not None:
            data['ignoreSignature'] = '1' if ignore_signature else '0'
        if accept_unsigned is not None:
            data['acceptUnsigned'] = '1' if accept_unsigned else '0'
        async with self.session.post(url, json=data) as resp:
            if resp.status != 200:
                raise AptlyError(resp.status, data)
            return data

    async def upload_files(self, name, dirname, files):
        url = urljoin(self.url, 'files/%s' % (dirname, ))
        async with MultipartWriter() as mpwriter:
            for f in files:
                mpwriter.append(f)
            async with self.session.post(url, data=mpwriter) as resp:
                data = json.loads(await resp.text())
                if resp.status != 200:
                    raise AptlyError(resp.status, data)
                return data

    async def publish(self, prefix, suite, distribution=None, architectures=None, not_automatic=None):
        url = urljoin(self.url, 'publish/%s' % (prefix, ))
        headers = {'Content-Type': 'application/json'}
        data = {
            'SourceKind': 'local',
            'Sources': [{'Component': 'main', 'Name': suite}],
            }
        if not_automatic is not None:
            data['NotAutomatic'] = 'yes' if not_automatic else 'no'
        if distribution is not None:
            data['Distribution'] = distribution
        if architectures is not None:
            data['Architectures'] = architectures
        async with self.session.post(url, headers=headers, json=data) as resp:
            if resp.status != 201:
                raise AptlyError(resp.status, await resp.text())
            data = json.loads(await resp.text())
            return data

    async def publish_update(self, prefix, suite, force_overwrite=False):
        url = urljoin(self.url, 'publish/%s/%s' % (prefix, suite))
        headers = {'Content-Type': 'application/json'}
        data = {
            'ForceOverwrite': force_overwrite,
            }
        async with self.session.put(url, headers=headers, json=data) as resp:
            if resp.status != 200:
                raise AptlyError(resp.status, await resp.text())
            data = json.loads(await resp.text())
            return data
