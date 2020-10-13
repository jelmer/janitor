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

import asyncio
from contextlib import ExitStack
import os
import itertools
import re
import shutil

from aiohttp import (
    ClientSession,
    ClientTimeout,
    ClientConnectionError,
    MultipartWriter,
    )

from debian.deb822 import Changes

from breezy import (
    urlutils,
    )
from breezy.trace import note

from lintian_brush.salsa import (
    guess_repository_url,
    salsa_url_from_alioth_url,
    )

from silver_platter.debian import (
    select_probers,
    )

from . import (
    state,
    )
from .vcs import (
    open_branch_ext,
    BranchOpenFailure,
    )

# Timeout in seconds for uploads
UPLOAD_TIMEOUT = 30 * 60


class NoChangesFile(Exception):
    """No changes file found."""


class UploadFailedError(Exception):
    """Upload failed."""


def find_changes(path, package):
    for name in os.listdir(path):
        if name.startswith('%s_' % package) and name.endswith('.changes'):
            break
    else:
        raise NoChangesFile(path, package)

    with open(os.path.join(path, name), 'r') as f:
        changes = Changes(f)
        return (name, changes["Version"], changes["Distribution"])


async def upload_changes(changes_path: str, incoming_url: str):
    """Upload changes to the archiver.

    Args:
      changes_path: Changes path
      incoming_url: Incoming URL
    """
    async with ClientSession() as session:
        with ExitStack() as es:
            with MultipartWriter() as mpwriter:
                f = open(changes_path, 'r')
                es.enter_context(f)
                dsc = Changes(f)
                f.seek(0)
                mpwriter.append(f)
                for file_details in dsc['files']:
                    name = file_details['name']
                    path = os.path.join(os.path.dirname(changes_path), name)
                    g = open(path, 'rb')
                    es.enter_context(g)
                    mpwriter.append(g)
            try:
                async with session.post(
                        incoming_url, data=mpwriter,
                        timeout=ClientTimeout(UPLOAD_TIMEOUT)) as resp:
                    if resp.status != 200:
                        raise UploadFailedError(resp)
            except ClientConnectionError as e:
                raise UploadFailedError(e)
            except asyncio.TimeoutError as e:
                raise UploadFailedError(e)


def possible_salsa_urls_from_package_name(package_name, maintainer_email=None):
    yield guess_repository_url(package_name, maintainer_email)
    yield 'https://salsa.debian.org/debian/%s.git' % package_name


def possible_urls_from_alioth_url(vcs_type, vcs_url):
    # These are the same transformations applied by vcswatc. The goal is mostly
    # to get a URL that properly redirects.
    https_alioth_url = re.sub(
        r'(https?|git)://(anonscm|git).debian.org/(git/)?',
        r'https://anonscm.debian.org/git/',
        vcs_url)

    yield https_alioth_url
    yield salsa_url_from_alioth_url(vcs_type, vcs_url)


async def open_guessed_salsa_branch(
        conn, pkg, vcs_type, vcs_url, possible_transports=None):
    package = await state.get_package(conn, pkg)
    probers = select_probers('git')
    vcs_url, params = urlutils.split_segment_parameters_raw(vcs_url)

    tried = set(vcs_url)

    for salsa_url in itertools.chain(
            possible_urls_from_alioth_url(vcs_type, vcs_url),
            possible_salsa_urls_from_package_name(
                package.name, package.maintainer_email)):
        if not salsa_url or salsa_url in tried:
            continue

        tried.add(salsa_url)

        salsa_url = urlutils.join_segment_parameters_raw(salsa_url, *params)

        note('Trying to access salsa URL %s instead.', salsa_url)
        try:
            branch = open_branch_ext(
                salsa_url, possible_transports=possible_transports,
                probers=probers)
        except BranchOpenFailure:
            pass
        else:
            note('Converting alioth URL: %s -> %s', vcs_url, salsa_url)
            return branch
    return None


def changes_filenames(changes_location):
    """Read the source filenames from a changes file."""
    with open(changes_location) as f:
        changes_contents = f.read()
    changes = Changes(changes_contents)
    for file_details in changes['files']:
        yield file_details['name']


def dget(changes_location, target_dir):
    """Copy all files referenced by a .changes file.

    Args:
      changes_location: Source file location
      target_dir: Target directory
    Return:
      path to target source file
    """
    srcdir = os.path.dirname(changes_location)
    for name in changes_filenames(changes_location):
        shutil.copy(
            os.path.join(srcdir, name),
            os.path.join(target_dir, name))
    shutil.copy(
        changes_location,
        os.path.join(target_dir, os.path.basename(changes_location)))
