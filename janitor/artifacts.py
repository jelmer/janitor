#!/usr/bin/python3
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

"""Artifacts."""

from aiohttp import ClientSession, ClientResponseError
import asyncio

import os
import shutil

from yarl import URL

from .trace import note


DEFAULT_GCS_TIMEOUT = 60


class ServiceUnavailable(Exception):
    """The remote server is temporarily unavailable."""


class ArtifactsMissing(Exception):
    """The specified artifacts are missing."""


class ArtifactManager(object):

    async def store_artifacts(self, run_id, local_path, names=None):
        raise NotImplementedError(self.store_artifacts)

    async def retrieve_artifacts(
            self, run_id, local_path, filter_fn=None, timeout=None):
        raise NotImplementedError(self.retrieve_artifacts)

    async def iter_ids(self):
        raise NotImplementedError(self.iter_ids)

    async def __aenter__(self):
        pass

    async def __aexit__(self, exc_type, exc, tb):
        return False


class LocalArtifactManager(ArtifactManager):

    def __init__(self, path):
        self.path = os.path.abspath(path)

    async def store_artifacts(self, run_id, local_path, names=None):
        run_dir = os.path.join(self.path, run_id)
        try:
            os.mkdir(run_dir)
        except FileExistsError:
            pass
        if names is None:
            names = os.listdir(local_path)
        for name in names:
            shutil.copy(
                os.path.join(local_path, name),
                os.path.join(run_dir, name))

    async def iter_ids(self):
        for entry in os.scandir(self.path):
            yield entry.name

    async def retrieve_artifacts(
            self, run_id, local_path, filter_fn=None, timeout=None):
        run_path = os.path.join(self.path, run_id)
        if not os.path.isdir(run_path):
            raise ArtifactsMissing(run_id)
        for entry in os.scandir(run_path):
            if filter_fn is not None and not filter_fn(entry.name):
                continue
            shutil.copy(entry.path, os.path.join(local_path, entry.name))


class GCSArtifactManager(ArtifactManager):

    def __init__(self, location, creds_path=None):
        self.bucket_name = URL(location).host
        self.creds_path = creds_path

    def __repr__(self):
        return "%s(%r)" % (type(self).__name__, 'gs://%s/' % self.bucket_name)

    async def __aenter__(self):
        from gcloud.aio.storage import Storage
        self.session = ClientSession()
        await self.session.__aenter__()
        self.storage = Storage(
            service_file=self.creds_path, session=self.session)
        self.bucket = self.storage.get_bucket(self.bucket_name)

    async def __aexit__(self, exc_type, exc, tb):
        await self.session.__aexit__(exc_type, exc, tb)
        return False

    async def store_artifacts(
            self, run_id, local_path, names=None, timeout=DEFAULT_GCS_TIMEOUT):
        if names is None:
            names = os.listdir(local_path)
        if not names:
            return
        todo = []
        for name in names:
            with open(os.path.join(local_path, name), 'rb') as f:
                uploaded_data = f.read()
                todo.append(self.storage.upload(
                    self.bucket_name, '%s/%s' % (run_id, name),
                    uploaded_data, timeout=timeout))
        try:
            await asyncio.gather(*todo)
        except ClientResponseError as e:
            if e.status == 503:
                raise ServiceUnavailable()
            raise
        note('Uploaded %r to run %s in bucket %s.',
             names, run_id, self.bucket_name)

    async def iter_ids(self):
        ids = set()
        for name in await self.bucket.list_blobs():
            log_id = name.split('/')[0]
            if log_id not in ids:
                yield log_id
            ids.add(log_id)

    async def retrieve_artifacts(
            self, run_id, local_path, filter_fn=None, timeout=DEFAULT_GCS_TIMEOUT):
        names = await self.bucket.list_blobs(prefix=run_id+'/')
        if not names:
            raise ArtifactsMissing(run_id)

        async def download_blob(name):
            with open(
                    os.path.join(local_path, os.path.basename(name)),
                    'wb+') as f:
                f.write(await self.storage.download(
                    bucket=self.bucket_name, object_name=name, timeout=timeout))

        await asyncio.gather(*[
            download_blob(name) for name in names
            if filter_fn is None or filter_fn(os.path.basename(name))])


def get_artifact_manager(location):
    if location.startswith('gs://'):
        return GCSArtifactManager(location)
    # TODO(jelmer): Support uploading to GCS
    return LocalArtifactManager(location)


async def list_ids(manager):
    async with manager:
        async for id in manager.iter_ids():
            print(id)


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest='command')
    list_parser = subparsers.add_parser('list')
    list_parser.add_argument('location', type=str)
    args = parser.parse_args()
    if args.command == 'list':
        manager = get_artifact_manager(args.location)
        asyncio.run(list_ids(manager))
