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

from abc import abstractmethod, ABC
from aiohttp import (
    ClientSession,
    ClientResponseError,
    ClientTimeout,
    ServerDisconnectedError,
)
from datetime import datetime
import gzip
from io import BytesIO
import os
from yarl import URL


class ServiceUnavailable(Exception):
    """The remote server is temporarily unavailable."""


class LogFileManager(ABC):
    @abstractmethod
    async def has_log(self, codebase: str, run_id: str, name: str, timeout=None):
        raise NotImplementedError(self.has_log)

    @abstractmethod
    async def get_log(self, codebase: str, run_id: str, name: str, timeout=None):
        raise NotImplementedError(self.get_log)

    @abstractmethod
    async def import_log(self, codebase: str, run_id: str, orig_path: str, timeout=None, mtime=None):
        raise NotImplementedError(self.import_log)

    @abstractmethod
    async def iter_logs(self):
        raise NotImplementedError(self.iter_logs)

    @abstractmethod
    async def get_ctime(self, codebase: str, run_id: str, name: str) -> datetime:
        raise NotImplementedError(self.get_ctime)

    async def __aexit__(self, exc_typ, exc_val, exc_tb):
        return False

    async def __aenter__(self):
        return self


class FileSystemLogFileManager(LogFileManager):
    def __init__(self, log_directory):
        self.log_directory = log_directory

    async def __aexit__(self, exc_typ, exc_val, exc_tb):
        return False

    async def __aenter__(self):
        return self

    def _get_paths(self, codebase: str, run_id: str, name: str):
        if "/" in codebase or "/" in run_id or "/" in name:
            return []
        return [
            os.path.join(self.log_directory, codebase, run_id, name),
            os.path.join(self.log_directory, codebase, run_id, name) + ".gz",
        ]

    async def iter_logs(self):
        for codebase in os.scandir(self.log_directory):
            for entry in os.scandir(codebase.path):
                yield (
                    codebase.name,
                    entry.name,
                    [n[:-3] if n.endswith('.gz') else n
                     for n in os.listdir(entry.path)])

    async def has_log(self, codebase, run_id, name):
        return any(map(os.path.exists, self._get_paths(codebase, run_id, name)))

    async def get_ctime(self, codebase: str, run_id: str, name: str) -> datetime:
        for p in self._get_paths(codebase, run_id, name):
            try:
                return datetime.fromtimestamp(os.stat(p).st_ctime)
            except FileNotFoundError:
                continue
        raise FileNotFoundError(name)

    async def get_log(self, codebase, run_id, name, timeout=None):
        for path in self._get_paths(codebase, run_id, name):
            if not os.path.exists(path):
                continue
            if path.endswith(".gz"):
                return gzip.GzipFile(path, mode="rb")
            else:
                return open(path, "rb")
        raise FileNotFoundError(name)

    async def import_log(self, codebase, run_id, orig_path, timeout=None, mtime=None):
        dest_dir = os.path.join(self.log_directory, codebase, run_id)
        os.makedirs(dest_dir, exist_ok=True)
        with open(orig_path, "rb") as inf:
            dest_path = os.path.join(dest_dir, os.path.basename(orig_path) + ".gz")
            with gzip.GzipFile(dest_path, mode="wb", mtime=mtime) as outf:
                outf.write(inf.read())

    async def delete_log(self, codebase, run_id, name):
        for path in self._get_paths(codebase, run_id, name):
            try:
                os.unlink(path)
            except FileNotFoundError:
                pass
            else:
                break
        else:
            raise FileNotFoundError(name)


class LogRetrievalError(Exception):
    """Unable to retrieve log file."""


class S3LogFileManager(LogFileManager):
    def __init__(self, endpoint_url, bucket_name="debian-janitor", trace_configs=None):
        self.base_url = endpoint_url + ("/%s/" % bucket_name)
        self.trace_configs = trace_configs
        self.bucket_name = bucket_name
        self.endpoint_url = endpoint_url

    async def __aenter__(self):
        import boto3
        self.session = ClientSession(trace_configs=self.trace_configs)
        self.s3 = boto3.resource("s3", endpoint_url=self.endpoint_url)
        self.s3_bucket = self.s3.Bucket(self.bucket_name)
        return self

    async def __aexit__(self, exc_typ, exc_val, exc_tb):
        return False

    def _get_key(self, codebase, run_id, name):
        return f"logs/{codebase}/{run_id}/{name}.gz"

    def _get_url(self, codebase, run_id, name):
        return f"{self.base_url}{self._get_key(codebase, run_id, name)}"

    async def has_log(self, codebase, run_id, name):
        url = self._get_url(codebase, run_id, name)
        async with self.session.head(url) as resp:
            if resp.status == 404:
                return False
            if resp.status == 200:
                return True
            if resp.status == 403:
                return False
            raise LogRetrievalError(
                "Unexpected response code %d: %s" % (resp.status, await resp.text())
            )

    async def get_log(self, codebase, run_id, name, timeout=10):
        url = self._get_url(codebase, run_id, name)
        client_timeout = ClientTimeout(timeout)
        async with self.session.get(url, timeout=client_timeout) as resp:
            if resp.status == 404:
                raise FileNotFoundError(name)
            if resp.status == 200:
                return BytesIO(gzip.decompress(await resp.read()))
            if resp.status == 403:
                raise PermissionError(await resp.text())
            raise LogRetrievalError(
                "Unexpected response code %d: %s" % (resp.status, await resp.text())
            )

    async def import_log(self, codebase, run_id, orig_path, timeout=360, mtime=None):
        with open(orig_path, "rb") as f:
            data = gzip.compress(f.read(), mtime=mtime)  # type: ignore

        key = self._get_key(codebase, run_id, os.path.basename(orig_path))
        self.s3_bucket.put_object(Key=key, Body=data, ACL="public-read")

    async def delete_log(self, codebase, run_id, name):
        key = self._get_key(codebase, run_id, name)
        self.s3_bucket.delete_objects(Delete={"Objects": [{"Key": key}]})

    async def iter_logs(self):
        # TODO(jelmer)
        raise NotImplementedError(self.iter_logs)

    async def get_ctime(self, codebase, run_id, name):
        # TODO(jelmer)
        raise NotImplementedError(self.get_ctime)


class GCSLogFileManager(LogFileManager):

    session: ClientSession

    def __init__(self, location, creds_path=None, trace_configs=None):
        hostname = URL(location).host
        if hostname is None:
            raise ValueError('invalid location missing bucket name: %s' % location)
        self.bucket_name = hostname
        self.trace_configs = trace_configs
        self.creds_path = creds_path

    async def __aenter__(self):
        from gcloud.aio.storage import Storage
        self.session = ClientSession(trace_configs=self.trace_configs)
        self.storage = Storage(service_file=self.creds_path, session=self.session)  # type: ignore
        self.bucket = self.storage.get_bucket(self.bucket_name)
        return self

    async def __aexit__(self, exc_typ, exc_val, exc_tb):
        return False

    async def iter_logs(self):
        seen: dict[tuple[str, str], list[str]] = {}
        for name in await self.bucket.list_blobs():
            codebase, log_id, lfn = name.split("/")
            seen.setdefault((codebase, log_id), []).append(lfn)
        for (codebase, log_id), lfns in seen.items():
            yield codebase, log_id, lfns

    def _get_object_name(self, codebase, run_id, name):
        return f"{codebase}/{run_id}/{name}.gz"

    async def has_log(self, codebase, run_id, name):
        object_name = self._get_object_name(codebase, run_id, name)
        return await self.bucket.blob_exists(object_name, session=self.session)  # type: ignore

    async def get_ctime(self, codebase, run_id, name):
        from iso8601 import parse_date
        object_name = self._get_object_name(codebase, run_id, name)
        try:
            blob = await self.bucket.get_blob(object_name, session=self.session)  # type: ignore
        except ClientResponseError as e:
            if e.status == 404:
                raise FileNotFoundError(name) from e
            raise ServiceUnavailable() from e
        except ServerDisconnectedError as e:
            raise ServiceUnavailable() from e
        return parse_date(blob.timeCreated)  # type: ignore

    async def get_log(self, codebase, run_id, name, timeout=30):
        object_name = self._get_object_name(codebase, run_id, name)
        try:
            data = await self.storage.download(
                self.bucket_name, object_name,
                session=self.session, timeout=timeout  # type: ignore
            )
            return BytesIO(gzip.decompress(data))
        except ClientResponseError as e:
            if e.status == 404:
                raise FileNotFoundError(name) from e
            raise ServiceUnavailable() from e
        except ServerDisconnectedError as e:
            raise ServiceUnavailable() from e

    async def import_log(self, codebase, run_id, orig_path, timeout=360, mtime=None):
        object_name = self._get_object_name(codebase, run_id, os.path.basename(orig_path))
        with open(orig_path, "rb") as f:
            plain_data = f.read()
        compressed_data = gzip.compress(plain_data, mtime=mtime)  # type: ignore
        try:
            await self.storage.upload(
                self.bucket_name, object_name, compressed_data, timeout=timeout
            )
        except ClientResponseError as e:
            if e.status == 503:
                raise ServiceUnavailable() from e
            if e.status == 403:
                data = await self.storage.download(
                    self.bucket_name, object_name,
                    session=self.session, timeout=timeout)  # type: ignore
                if data == plain_data:
                    return
                raise PermissionError(e.message) from e
            raise


def get_log_manager(location, trace_configs=None):
    if location.startswith("gs://"):
        return GCSLogFileManager(location, trace_configs=trace_configs)
    if location.startswith("http:") or location.startswith("https:"):
        return S3LogFileManager(location, trace_configs=trace_configs)
    return FileSystemLogFileManager(location)
