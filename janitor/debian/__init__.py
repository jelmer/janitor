#!/usr/bin/python3
# Copyright (C) 2018 Jelmer Vernooij <jelmer@jelmer.uk>
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

try:
    from functools import cache  # type: ignore
except ImportError:  # python < 3.9
    from functools import lru_cache

    def cache(user_function):  # type: ignore
        return lru_cache(maxsize=None)(user_function)


import os
import subprocess
from typing import Tuple, List, Optional

from debian.changelog import Changelog, Version
from debian.deb822 import Changes

from breezy import (
    osutils,
)
from breezy.workingtree import WorkingTree


class NoChangesFile(Exception):
    """No changes file found."""


class InconsistentChangesFiles(Exception):
    """Inconsistent changes files."""


def find_changes(path: str) -> Tuple[List[str], str, Version, str, List[str]]:
    names = []
    source: str
    version: Optional[Version] = None
    distribution: Optional[str] = None
    binary_packages: List[str] = []
    for entry in os.scandir(path):
        if not entry.name.endswith(".changes"):
            continue
        with open(entry.path, "r") as f:
            changes = Changes(f)
            names.append(entry.name)
            if version is not None and changes["Version"] != version:
                raise InconsistentChangesFiles(
                    names, 'Version', changes['Version'], version)
            version = changes['Version']
            if source is not None and changes['Source'] != source:
                raise InconsistentChangesFiles(
                    names, 'Source', changes['Source'], source)
            source = changes['Source']
            if distribution is not None and changes["Distribution"] != distribution:
                raise InconsistentChangesFiles(
                    names, 'Distribution', changes['Distribution'], distribution)
            distribution = changes['Distribution']
            binary_packages.extend(
                [entry['name'].split('_')[0]
                 for entry in changes['files']
                 if entry['name'].endswith('.deb')])
    if not names:
        raise NoChangesFile(path)
    if version is None:
        raise InconsistentChangesFiles('Version not set')
    if distribution is None:
        raise InconsistentChangesFiles('Distribution not set')
    return (names, source, version, distribution, binary_packages)


def changes_filenames(changes_location):
    """Read the source filenames from a changes file."""
    with open(changes_location) as f:
        changes_contents = f.read()
    changes = Changes(changes_contents)
    for file_details in changes["files"]:
        yield file_details["name"]


def tree_set_changelog_version(
    tree: WorkingTree, build_version: Version, subpath: str
) -> None:
    cl_path = osutils.pathjoin(subpath, "debian/changelog")
    with tree.get_file(cl_path) as f:
        cl = Changelog(f)
    if Version(str(cl.version) + "~") > build_version:
        return
    cl.version = build_version
    with open(tree.abspath(cl_path), "w") as f:
        cl.write_to_open_file(f)


@cache
def dpkg_vendor():
    return subprocess.check_output(['dpkg-vendor', '--query', 'vendor']).strip().decode()
