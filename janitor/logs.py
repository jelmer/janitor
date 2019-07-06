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

from gzip import GzipFile
import os


class LogFileManager(object):

    def __init__(self, log_directory):
        self.log_directory = log_directory

    def _get_paths(self, pkg, run_id, name):
        return [
            os.path.join(self.log_directory, pkg, run_id, name),
            os.path.join(self.log_directory, pkg, run_id, name) + '.gz'
        ]

    def has_log(self, pkg, run_id, name):
        return any(map(os.path.exists, self._get_paths(pkg, run_id, name)))

    def get_log(self, pkg, run_id, name):
        for path in self._get_paths(pkg, run_id, name):
            if not os.path.exists(path):
                continue
            if path.endswith('.gz'):
                return GzipFile(path, 'rb')
            else:
                return open(path, 'rb')
        raise FileNotFoundError(name)

    def import_log(self, pkg, run_id, orig_path):
        dest_dir = os.path.join(self.log_directory, pkg, run_id)
        os.makedirs(dest_dir, exist_ok=True)
        with open(orig_path, 'rb') as inf:
            dest_path = os.path.join(
                dest_dir, os.path.basename(orig_path) + '.gz')
            with GzipFile(dest_path, 'wb') as outf:
                outf.write(inf)



