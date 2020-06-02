#!/usr/bin/python
# Copyright (C) 2020 Jelmer Vernooij <jelmer@jelmer.uk>
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

from aiohttp.multipart import MultipartReader

from io import BytesIO

import os
import shutil
import tempfile

import asynctest

from janitor.pull_worker import bundle_results


class AsyncBytesIO:

    def __init__(self):
        self._io = BytesIO()

    def seek(self, pos):
        self._io.seek(pos)

    async def write(self, chunk):
        self._io.write(chunk)

    async def readline(self):
        return self._io.readline()

    async def read(self, size=None):
        return self._io.read(size)

    def getvalue(self):
        return b''.join(self.chunks)


class BundleResultsTests(asynctest.TestCase):

    def setUp(self):
        super(BundleResultsTests, self).setUp()
        self.test_dir = tempfile.mkdtemp()
        old_dir = os.getcwd()
        os.chdir(self.test_dir)
        self.addCleanup(os.chdir, old_dir)
        self.addCleanup(shutil.rmtree, self.test_dir)

    async def test_simple(self):
        with open('a', 'w') as f:
            f.write('some data\n')
        with bundle_results(
                {'result_code': 'success'}, self.test_dir) as writer:
            self.assertEqual(['Content-Type'], list(writer.headers.keys()))
            b = AsyncBytesIO()
            await writer.write(b)
            b.seek(0)
            reader = MultipartReader(writer.headers, b)
            part = await reader.next()
            self.assertEqual(part.headers, {
                'Content-Disposition':
                    'attachment; filename="result.json"; '
                    'filename*=utf-8\'\'result.json',
                'Content-Length': '26',
                'Content-Type': 'application/json'})
            self.assertEqual('result.json', part.filename)
            self.assertEqual(
                b'{"result_code": "success"}',
                bytes(await part.read()))
            part = await reader.next()
            self.assertEqual(part.headers, {
                'Content-Disposition':
                    'attachment; filename="a"; '
                    'filename*=utf-8\'\'a',
                'Content-Length': '10',
                'Content-Type': 'application/octet-stream'})
            self.assertEqual('a', part.filename)
            self.assertEqual(
                b'some data\n',
                bytes(await part.read()))
            self.assertTrue(part.at_eof())
