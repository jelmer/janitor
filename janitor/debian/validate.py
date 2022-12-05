#!/usr/bin/python3
# Copyright (C) 2022 Jelmer Vernooij <jelmer@jelmer.uk>
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

import logging
import os

from breezy.plugins.debian.vcs_up_to_date import (
    check_up_to_date,
    PackageMissingInArchive,
    MissingChangelogError,
    TreeVersionNotInArchive,
    NewArchiveVersion,
)
from breezy.plugins.debian.apt_repo import RemoteApt


class ValidateError(Exception):

    def __init__(self, code, description):
        self.code = code
        self.description = description


def validate_from_config(local_tree, subpath, config):
    if config.get('base-apt-repository'):
        apt = RemoteApt.from_string(
            config['base-apt-repository'],
            config.get('base-apt-repository-signed-by'))
        try:
            check_up_to_date(local_tree, subpath, apt)
        except MissingChangelogError as exc:
            if not os.path.isdir(local_tree.abspath(os.path.join(subpath, 'debian'))):
                raise ValidateError(
                    'not-debian-package', "Not a Debian package") from exc
            raise ValidateError('missing-changelog', str(exc)) from exc
        except PackageMissingInArchive as exc:
            logging.warning(
                'Package %s is not present in archive', exc.package)
        except TreeVersionNotInArchive as exc:
            logging.warning(
                'Last tree version %s not present in the archive',
                exc.tree_version)
        except NewArchiveVersion as exc:
            raise ValidateError('new-archive-version', str(exc)) from exc
