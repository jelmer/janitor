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

import os
import shutil


class ArtifactManager(object):

    async def store_artifacts(self, run_id, local_path, names=None):
        raise NotImplementedError(self.store_artifacts)

    async def retrieve_artifacts(self, run_id, local_path):
        raise NotImplementedError(self.retrieve_artifacts)


class LocalArtifactManager(ArtifactManager):

    def __init__(self, path):
        self.path = os.path.abspath(path)

    async def store_artifacts(self, run_id, local_path, names=None):
        run_dir = os.path.join(self.path, run_id)
        # TODO(jelmer): Handle directory already existing
        os.mkdir(run_dir)
        if names is None:
            names = os.listdir(local_path)
        for name in names:
            shutil.copy(
                os.path.join(local_path, name),
                os.path.join(run_dir, name))


def get_artifact_manager(location):
    # TODO(jelmer): Support uploading to GCS
    return LocalArtifactManager(location)
