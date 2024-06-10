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

import asyncio
import logging
import os
import shutil
import tempfile
from io import BytesIO
from typing import TYPE_CHECKING, Optional

from aiohttp import ClientResponseError, ClientSession
from yarl import URL

DEFAULT_GCS_TIMEOUT = 60


class ServiceUnavailable(Exception):
    """The remote server is temporarily unavailable."""


class ArtifactsMissing(Exception):
    """The specified artifacts are missing."""


class ArtifactManager:
    """Manage sets of per-run artifacts.

    Artifacts are named files; no other metadata is stored.
    """

    async def store_artifacts(
        self, run_id: str, local_path: str, names: Optional[list[str]] = None
    ):
        """Store a set of artifacts.

        Args:
          run_id: The run id
          local_path: Local path to retrieve files from
          names: Optional list of filenames in local_path to upload.
            Defaults to all files in local_path.
        """
        raise NotImplementedError(self.store_artifacts)

    async def get_artifact(self, run_id, filename, timeout=None):
        raise NotImplementedError(self.get_artifact)

    def public_artifact_url(self, run_id, filename):
        raise NotImplementedError(self.public_artifact_url)

    async def retrieve_artifacts(
        self, run_id, local_path, filter_fn=None, timeout=None
    ):
        raise NotImplementedError(self.retrieve_artifacts)

    async def iter_ids(self):
        raise NotImplementedError(self.iter_ids)

    async def __aenter__(self):
        return self

    async def __aexit__(self, exc_type, exc, tb):
        return False


class LocalArtifactManager(ArtifactManager):
    def __init__(self, path) -> None:
        self.path = os.path.abspath(path)
        if not os.path.isdir(self.path):
            os.makedirs(self.path)

    def __repr__(self) -> str:
        return f"{type(self).__name__}({self.path!r})"

    async def store_artifacts(self, run_id, local_path, names=None, timeout=None):
        run_dir = os.path.join(self.path, run_id)
        try:
            os.mkdir(run_dir)
        except FileExistsError:
            pass
        if names is None:
            names = os.listdir(local_path)
        for name in names:
            shutil.copy(os.path.join(local_path, name), os.path.join(run_dir, name))

    async def iter_ids(self):
        for entry in os.scandir(self.path):
            yield entry.name

    async def delete_artifacts(self, run_id):
        shutil.rmtree(os.path.join(self.path, run_id))

    async def get_artifact(self, run_id, filename, timeout=None):
        return open(os.path.join(self.path, run_id, filename), "rb")

    def public_artifact_url(self, run_id, filename):
        raise NotImplementedError(self.public_artifact_url)

    async def retrieve_artifacts(
        self, run_id, local_path, filter_fn=None, timeout=None
    ):
        run_path = os.path.join(self.path, run_id)
        if not os.path.isdir(run_path):
            raise ArtifactsMissing(run_id)
        for entry in os.scandir(run_path):
            if filter_fn is not None and not filter_fn(entry.name):
                continue
            shutil.copy(entry.path, os.path.join(local_path, entry.name))


class GCSArtifactManager(ArtifactManager):
    if TYPE_CHECKING:
        from gcloud.aio.storage import Storage

    session: ClientSession
    bucket_name: str
    storage: "Storage"

    def __init__(self, location, creds_path=None, trace_configs=None) -> None:
        hostname = URL(location).host
        if hostname is None:
            raise ValueError(f"missing bucket in {location}")
        self.bucket_name = hostname
        self.creds_path = creds_path
        self.trace_configs = trace_configs

    def __repr__(self) -> str:
        return "{}({!r})".format(type(self).__name__, f"gs://{self.bucket_name}/")

    async def __aenter__(self):
        from gcloud.aio.storage import Storage

        s = ClientSession(trace_configs=self.trace_configs)
        self.session = await s.__aenter__()
        self.storage = Storage(service_file=self.creds_path, session=self.session)  # type: ignore
        self.bucket = self.storage.get_bucket(self.bucket_name)

    async def __aexit__(self, exc_type, exc, tb):
        await self.session.__aexit__(exc_type, exc, tb)
        return False

    async def store_artifacts(self, run_id, local_path, names=None, timeout=None):
        if timeout is None:
            timeout = DEFAULT_GCS_TIMEOUT
        if names is None:
            names = os.listdir(local_path)
        if not names:
            return
        try:
            await asyncio.gather(
                *[
                    self.storage.upload_from_filename(
                        self.bucket_name,
                        f"{run_id}/{name}",
                        os.path.join(local_path, name),
                        timeout=timeout,
                    )
                    for name in names
                ]
            )
        except ClientResponseError as e:
            if e.status == 503:
                raise ServiceUnavailable() from e
            raise
        logging.info(
            "Uploaded %r to run %s in bucket %s.",
            names,
            run_id,
            self.bucket_name,
            extra={"run_id": run_id},
        )

    async def iter_ids(self):
        ids = set()
        for name in await self.bucket.list_blobs():
            log_id = name.split("/")[0]
            if log_id not in ids:
                yield log_id
            ids.add(log_id)

    async def retrieve_artifacts(
        self, run_id, local_path, filter_fn=None, timeout=None
    ):
        if timeout is None:
            timeout = DEFAULT_GCS_TIMEOUT
        names = await self.bucket.list_blobs(prefix=run_id + "/")
        if not names:
            raise ArtifactsMissing(run_id)

        async def download_blob(name):
            with open(os.path.join(local_path, os.path.basename(name)), "wb+") as f:
                f.write(
                    await self.storage.download(
                        bucket=self.bucket_name, object_name=name, timeout=timeout
                    )
                )

        await asyncio.gather(
            *[
                download_blob(name)
                for name in names
                if filter_fn is None or filter_fn(os.path.basename(name))
            ]
        )

    async def get_artifact(self, run_id, filename, timeout=DEFAULT_GCS_TIMEOUT):
        try:
            return BytesIO(
                await self.storage.download(
                    bucket=self.bucket_name,
                    object_name=f"{run_id}/{filename}",
                    timeout=timeout,
                )
            )
        except ClientResponseError as e:
            if e.status == 503:
                raise ServiceUnavailable() from e
            if e.status == 404:
                raise FileNotFoundError(filename) from e
            raise

    def public_artifact_url(self, run_id, filename):
        from urllib.parse import quote

        encoded_object_name = quote(f"{run_id}/{filename}", safe="")
        return (
            f"{self.storage._api_root_read}/{self.bucket_name}/o/{encoded_object_name}"
        )


def get_artifact_manager(location, trace_configs=None):
    if location.startswith("gs://"):
        return GCSArtifactManager(location, trace_configs=trace_configs)
    return LocalArtifactManager(location)


async def list_ids(manager):
    async with manager:
        async for id in manager.iter_ids():
            print(id)


async def upload_backup_artifacts(
    backup_artifact_manager, artifact_manager, timeout=None
):
    async for run_id in backup_artifact_manager.iter_ids():
        with tempfile.TemporaryDirectory(prefix="janitor-artifacts") as td:
            await backup_artifact_manager.retrieve_artifacts(
                run_id, td, timeout=timeout
            )
            try:
                await artifact_manager.store_artifacts(run_id, td, timeout=timeout)
            except Exception as e:
                logging.warning(
                    "Unable to upload backup artifacts (%r): %s",
                    run_id,
                    e,
                    extra={"run_id": run_id},
                )
            else:
                await backup_artifact_manager.delete_artifacts(run_id)


async def store_artifacts_with_backup(manager, backup_manager, from_dir, run_id, names):
    try:
        await manager.store_artifacts(run_id, from_dir, names)
    except Exception as e:
        logging.warning(
            "Unable to upload artifacts for %r: %r", run_id, e, extra={"run_id": run_id}
        )
        if backup_manager:
            await backup_manager.store_artifacts(run_id, from_dir, names)
            logging.info(
                "Uploading results to backup artifact " "location %r.",
                backup_manager,
                extra={"run_id": run_id},
            )
        else:
            logging.warning(
                "No backup artifact manager set. ", extra={"run_id": run_id}
            )
            raise


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command")
    list_parser = subparsers.add_parser("list")
    list_parser.add_argument("location", type=str)
    args = parser.parse_args()
    if args.command == "list":
        manager = get_artifact_manager(args.location)
        asyncio.run(list_ids(manager))
