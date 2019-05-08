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
from breezy.branch import Branch
from breezy.errors import (
    NotBranchError,
    NoRepositoryPresent,
    InvalidHttpResponse,
    )
from breezy.git.remote import RemoteGitError
from breezy.controldir import ControlDir, format_registry
from silver_platter.utils import (
    open_branch,
    BranchUnavailable,
    )


class BranchOpenFailure(Exception):
    """Failure to open a branch."""

    def __init__(self, code, description):
        self.code = code
        self.description = description


def get_vcs_abbreviation(branch):
    vcs = getattr(branch.repository, 'vcs', None)
    if vcs:
        return vcs.abbreviation
    return 'bzr'


def is_alioth_url(url):
    return urllib.parse.urlparse(url).netloc in (
        'svn.debian.org', 'bzr.debian.org', 'anonscm.debian.org',
        'bzr.debian.org', 'git.debian.org')


def open_branch_ext(vcs_url, possible_transports=None):
    try:
        return open_branch(vcs_url, possible_transports)
    except BranchUnavailable as e:
        if str(e).startswith('Unsupported protocol for url '):
            code = 'unsupported-vcs-protocol'
        elif 'http code 429: Too Many Requests' in str(e):
            code = 'too-many-requests'
        elif str(e).startswith('Branch does not exist: Not a branch: '
                               '"https://anonscm.debian.org'):
            code = 'hosted-on-alioth'
        else:
            if is_alioth_url(vcs_url):
                code = 'hosted-on-alioth'
            else:
                code = 'branch-unavailable'
        raise BranchOpenFailure(code, str(e))
    except KeyError as e:
        if e.args == ('www-authenticate not found',):
            raise BranchOpenFailure(
                '401-without-www-authenticate', str(e))
        else:
            raise


def mirror_branches(vcs_result_dir, pkg, branch_map,
                    public_master_branch=None):
    vcses = set(get_vcs_abbreviation(br) for br in branch_map.values())
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
            from_branch.push(target_branch, overwrite=True)
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
            from_branch.push(target_branch, overwrite=True)
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
    if get_vcs_abbreviation(local_branch) == 'git':
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
        url = 'https://janitor.debian.net/git/%s,branch=%s' % (
            package, branch_name)
    elif vcs_type == 'bzr':
        url = 'https://janitor.debian.net/bzr/%s/%s' % (
            package, branch_name)
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
