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
import gzip
from io import BytesIO
import os


class LogFileManager(object):

    async def has_log(self, pkg, run_id, name):
        raise NotImplementedError(self.has_log)

    async def get_log(self, pkg, run_id, name):
        raise NotImplementedError(self.get_log)

    async def import_log(self, pkg, run_id, orig_path):
        raise NotImplementedError(self.import_log)


class FileSystemLogFileManager(LogFileManager):

    def __init__(self, log_directory):
        self.log_directory = log_directory

    def _get_paths(self, pkg, run_id, name):
        if '/' in pkg or '/' in run_id or '/' in name:
            return []
        return [
            os.path.join(self.log_directory, pkg, run_id, name),
            os.path.join(self.log_directory, pkg, run_id, name) + '.gz'
        ]

    async def has_log(self, pkg, run_id, name):
        return any(map(os.path.exists, self._get_paths(pkg, run_id, name)))

    async def get_log(self, pkg, run_id, name):
        for path in self._get_paths(pkg, run_id, name):
            if not os.path.exists(path):
                continue
            if path.endswith('.gz'):
                return gzip.GzipFile(path, mode='rb')
            else:
                return open(path, 'rb')
        raise FileNotFoundError(name)

    async def import_log(self, pkg, run_id, orig_path):
        dest_dir = os.path.join(self.log_directory, pkg, run_id)
        os.makedirs(dest_dir, exist_ok=True)
        with open(orig_path, 'rb') as inf:
            dest_path = os.path.join(
                dest_dir, os.path.basename(orig_path) + '.gz')
            with gzip.GzipFile(dest_path, mode='wb') as outf:
                outf.write(inf.read())


class S3LogFileManager(LogFileManager):

    def __init__(self, base_url):
        self.base_url = base_url
        self.session = ClientSession()

    def _get_url(self, pkg, run_id, name):
        return '%s/%s/%s/%s.gz' % (self.base_url, pkg, run_id, name)

    async def has_log(self, pkg, run_id, name):
        url = self._get_url(pkg, run_id, name)
        async with self.session.head(url) as resp:
            if resp.status == 404:
                return False
            if resp.status == 200:
                return True
            if resp.status == 403:
                return False
            raise AssertionError('Unexpected response code %d' % resp.status)

    async def get_log(self, pkg, run_id, name):
        async with self.session.get(self._get_url(pkg, run_id, name)) as resp:
            if resp.status == 404:
                raise FileNotFoundError(name)
            if resp.status == 200:
                return BytesIO(gzip.decompress(await resp.read()))
            if resp.status == 403:
                raise PermissionError(await resp.text())
            raise AssertionError('Unexpected response code %d' % resp.status)
