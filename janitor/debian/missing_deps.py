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

from dataclasses import dataclass
import logging
from typing import Optional, List, Union

import apt_pkg
from debian.changelog import Version

from lintian_brush.debianize import find_upstream, UpstreamInfo
from ognibuild.requirements import Requirement
from ognibuild.resolver.apt import resolve_requirement_apt


@dataclass
class NewPackage:

    upstream_info: UpstreamInfo

    def json(self):
        return {'action': 'new-package', 'upstream-info': self.upstream_info.json()}


@dataclass
class UpdatePackage:

    name: str
    desired_version: Optional[Version] = None

    def json(self):
        return {
            'action': 'update-package',
            'package': self.name,
            'desired-version': self.desired_version,
            }


def resolve_requirement(apt_mgr, requirement: Requirement) -> List[List[Union[NewPackage, UpdatePackage]]]:
    apt_opts = resolve_requirement_apt(apt_mgr, requirement)
    options = []
    if apt_opts:
        for apt_req in apt_opts:
            option: Optional[List[Union[NewPackage, UpdatePackage]]] = []
            for entry in apt_req.relations:
                for r in entry:
                    versions = apt_mgr.package_versions(r['name'])
                    if not versions:
                        upstream = find_upstream(apt_req)
                        if upstream:
                            option.append(NewPackage(upstream))
                        else:
                            option = None
                            break
                    else:
                        if not r.get('version'):
                            logging.debug('package already available: %s', r['name'])
                        elif r['version'][0] == '>=':
                            depcache = apt_pkg.DepCache(apt_mgr.apt_cache._cache)
                            depcache.init()
                            version = depcache.get_candidate_ver(apt_mgr.apt_cache._cache[r['name']])
                            if not version:
                                logging.warning(
                                    'unable to find source package matching %s', r['name'])
                                option = None
                                break
                            file, index = version.file_list.pop(0)
                            records = apt_pkg.PackageRecords(apt_mgr.apt_cache._cache)
                            records.lookup((file, index))
                            option.append(UpdatePackage(records.source_pkg, r['version'][1]))
                        else:
                            logging.warning("don't know what to do with constraint %r", r['version'])
                            option = None
                            break
                if option is None:
                    break
            if option == []:
                return [[]]
            if option is not None:
                options.append(option)
    else:
        upstream = find_upstream(requirement)
        if upstream:
            options.append([NewPackage(upstream)])

    return options



