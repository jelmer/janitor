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

import os

import urllib.parse
import breezy.git  # noqa: F401
import breezy.bzr  # noqa: F401
from breezy.branch import Branch
from breezy.errors import (
    ConnectionError,
    NotBranchError,
    NoSuchRevision,
    NoRepositoryPresent,
    InvalidHttpResponse,
    )
from breezy.git.remote import RemoteGitError
from breezy.controldir import ControlDir, format_registry
from breezy.repository import Repository
from silver_platter.utils import (
    open_branch,
    BranchMissing,
    BranchUnavailable,
    )

from .trace import note


SUPPORTED_VCSES = ['git', 'bzr']

CACHE_URL_BZR = 'https://janitor.debian.net/bzr/'
CACHE_URL_GIT = 'https://janitor.debian.net/git/'


class BranchOpenFailure(Exception):
    """Failure to open a branch."""

    def __init__(self, code, description):
        self.code = code
        self.description = description


def get_vcs_abbreviation(repository):
    vcs = getattr(repository, 'vcs', None)
    if vcs:
        return vcs.abbreviation
    return 'bzr'


def is_alioth_url(url):
    return urllib.parse.urlparse(url).netloc in (
        'svn.debian.org', 'bzr.debian.org', 'anonscm.debian.org',
        'hg.debian.org', 'git.debian.org', 'alioth.debian.org')


def open_branch_ext(vcs_url, possible_transports=None, vcs_type=None):
    if ' [' in vcs_url and vcs_url.endswith(']'):
        subpath = vcs_url[vcs_url.rindex('['):vcs_url.rindex(']')]
        vcs_url = vcs_url[:vcs_url.rindex('[')].rstrip()
    else:
        subpath = None
    try:
        return open_branch(vcs_url, possible_transports, vcs_type), subpath
    except BranchUnavailable as e:
        if str(e).startswith('Unsupported protocol for url '):
            if ('anonscm.debian.org' in str(e) or
                    'svn.debian.org' in str(e)):
                code = 'hosted-on-alioth'
            else:
                code = 'unsupported-vcs-protocol'
        elif 'http code 429: Too Many Requests' in str(e):
            code = 'too-many-requests'
        elif is_alioth_url(vcs_url):
            code = 'hosted-on-alioth'
        elif 'Unable to handle http code 401: Unauthorized' in str(e):
            code = '401-unauthorized'
        else:
            code = 'branch-unavailable'
        raise BranchOpenFailure(code, str(e))
    except BranchMissing as e:
        if str(e).startswith('Branch does not exist: Not a branch: '
                             '"https://anonscm.debian.org'):
            code = 'hosted-on-alioth'
        else:
            code = 'branch-missing'
        raise BranchOpenFailure(code, str(e))
    except KeyError as e:
        if e.args == ('www-authenticate not found',):
            raise BranchOpenFailure(
                '401-without-www-authenticate', str(e))
        else:
            raise


class MirrorFailure(Exception):
    """Branch failed to mirror."""

    def __init__(self, branch_name, reason):
        self.branch_name = branch_name
        self.reason = reason


def mirror_branches(vcs_result_dir, pkg, branch_map,
                    public_master_branch=None):
    vcses = set(get_vcs_abbreviation(br.repository) for name, br in branch_map)
    if len(vcses) == 0:
        return
    if len(vcses) > 1:
        raise AssertionError('more than one VCS: %r' % branch_map)
    vcs = vcses.pop()
    if vcs == 'git':
        path = os.path.join(vcs_result_dir, 'git', pkg)
        os.makedirs(path, exist_ok=True)
        try:
            vcs_result_controldir = ControlDir.open(path)
        except NotBranchError:
            vcs_result_controldir = ControlDir.create(
                path, format=format_registry.get('git-bare')())
        for (target_branch_name, from_branch) in branch_map:
            try:
                target_branch = vcs_result_controldir.open_branch(
                    name=target_branch_name)
            except NotBranchError:
                target_branch = vcs_result_controldir.create_branch(
                    name=target_branch_name)
            # TODO(jelmer): Set depth
            try:
                from_branch.push(target_branch, overwrite=True)
            except NoSuchRevision as e:
                raise MirrorFailure(target_branch_name, e)
    elif vcs == 'bzr':
        path = os.path.join(vcs_result_dir, 'bzr', pkg)
        os.makedirs(path, exist_ok=True)
        try:
            vcs_result_controldir = ControlDir.open(path)
        except NotBranchError:
            vcs_result_controldir = ControlDir.create(
                path, format=format_registry.get('bzr')())
        try:
            vcs_result_controldir.open_repository()
        except NoRepositoryPresent:
            vcs_result_controldir.create_repository(shared=True)
        for (target_branch_name, from_branch) in branch_map:
            target_branch_path = os.path.join(path, target_branch_name)
            try:
                target_branch = Branch.open(target_branch_path)
            except NotBranchError:
                target_branch = ControlDir.create_branch_convenience(
                    target_branch_path)
            if public_master_branch:
                target_branch.set_stacked_on_url(public_master_branch.user_url)
            try:
                from_branch.push(target_branch, overwrite=True)
            except NoSuchRevision as e:
                raise MirrorFailure(target_branch_name, e)
    else:
        raise AssertionError('unsupported vcs %s' % vcs.abbreviation)


def copy_vcs_dir(main_branch, local_branch, vcs_result_dir, pkg, name,
                 additional_colocated_branches=None):
    """Publish resulting changes in VCS form.

    This creates a repository with the following branches:
     * master - the original Debian packaging branch
     * KIND - whatever command was run
     * upstream - the upstream branch (optional)
     * pristine-tar the pristine tar packaging branch (optional)
    """
    branch_map = [
        (name, local_branch),
        ('master', main_branch),
    ]
    if get_vcs_abbreviation(local_branch.repository) == 'git':
        for branch_name in (additional_colocated_branches or []):
            try:
                from_branch = local_branch.controldir.open_branch(
                    name=branch_name)
            except NotBranchError:
                continue
            branch_map.append((branch_name, from_branch))
    mirror_branches(
        vcs_result_dir, pkg, branch_map, public_master_branch=main_branch)


def get_cached_branch(vcs_type, package, branch_name):
    if vcs_type == 'git':
        url = '%s%s,branch=%s' % (
            CACHE_URL_GIT, package, branch_name)
    elif vcs_type == 'bzr':
        url = '%s%s/%s' % (
            CACHE_URL_BZR, package, branch_name)
    else:
        raise AssertionError('unknown vcs type %r' % vcs_type)
    try:
        return Branch.open(url)
    except NotBranchError:
        return None
    except RemoteGitError:
        return None
    except InvalidHttpResponse:
        return None
    except ConnectionError as e:
        note('Unable to reach cache server: %s', e)
        return None


def get_local_vcs_branch(vcs_directory, pkg, branch_name):
    for vcs in SUPPORTED_VCSES:
        if os.path.exists(os.path.join(vcs_directory, 'git', pkg)):
            break
    else:
        return None
    if vcs == 'git':
        return open_branch(
            'file:%s,branch=%s' % (
                os.path.join(vcs_directory, 'git', pkg), branch_name))
    elif vcs == 'bzr':
        return open_branch(
            os.path.join(vcs_directory, 'bzr', pkg, branch_name))


def get_local_vcs_repo(vcs_directory, package, vcs_type=None):
    for vcs in (SUPPORTED_VCSES if not vcs_type else [vcs_type]):
        path = os.path.join(vcs_directory, vcs, package)
        if not os.path.exists(path):
            continue
        return Repository.open(path)
    return None


class VcsManager(object):

    def get_branch(self, package, branch_name, vcs_type=None):
        raise NotImplementedError(self.get_branch)

    def import_branches(self, main_branch, local_branch, pkg, name,
                        additional_colocated_branches=None):
        raise NotImplementedError(self.import_branches)

    def get_repository(self, package, vcs_type=None):
        raise NotImplementedError(self.get_repository)


class LocalVcsManager(VcsManager):

    def __init__(self, base_path):
        self.base_path = base_path

    def get_branch(self, package, branch_name, vcs_type=None):
        try:
            return get_local_vcs_branch(self.base_path, package, branch_name)
        except (BranchUnavailable, BranchMissing):
            return None

    def import_branches(self, main_branch, local_branch, pkg, name,
                        additional_colocated_branches=None):
        copy_vcs_dir(
            main_branch, local_branch, self.base_path, pkg, name,
            additional_colocated_branches=additional_colocated_branches)

    def get_repository(self, package, vcs_type=None):
        return get_local_vcs_repo(self.base_path, package, vcs_type)


class RemoteVcsManager(VcsManager):

    def __init__(self, cache_url_git=CACHE_URL_BZR,
                 cache_url_bzr=CACHE_URL_BZR):
        self.cache_url_git = cache_url_git
        self.cache_url_bzr = cache_url_bzr

    def get_branch(self, package, branch_name, vcs_type=None):
        if vcs_type:
            return get_cached_branch(package, branch_name, vcs_type)
        for vcs_type in SUPPORTED_VCSES:
            branch = get_cached_branch(package, branch_name, vcs_type)
            if branch:
                return branch
        else:
            return None
