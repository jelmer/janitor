#!/usr/bin/python3
# Copyright (C) 2021-2022 Jelmer Vernooij <jelmer@jelmer.uk>
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


from debian.deb822 import PkgRelation
from debmutate.control import suppress_substvar_warnings
import warnings


def has_relation(v, pkg):
    if not v:
        return False
    for r in PkgRelation.parse_relations(v):
        for o in r:
            if o['name'] == pkg:
                return True
    return False


def has_build_relation(c, pkg):
    for f in ["Build-Depends", "Build-Depends-Indep", "Build-Depends-Arch",
              "Build-Conflicts", "Build-Conflicts-Indep",
              "Build-Conflicts-Arch"]:
        if has_relation(c.get(f, ""), pkg):
            return True
    return False


def has_runtime_relation(c, pkg):
    for f in ["Depends", "Recommends", "Suggests",
              "Breaks", "Replaces"]:
        if has_relation(c.get(f, ""), pkg):
            return True
    return False


def find_reverse_source_deps(apt, binary_packages):
    # TODO(jelmer): in the future, we may want to do more than trigger
    # control builds here, e.g. trigger fresh-releases
    # (or maybe just if the control build fails?)

    need_control = set()
    with apt:
        for source in apt.iter_sources():
            if any([has_build_relation(source, p) for p in binary_packages]):
                need_control.add(source['Package'])
                break

        for binary in apt.iter_binaries():
            if any([has_runtime_relation(binary, p) for p in binary_packages]):
                need_control.add(binary['Source'].split(' ')[0])
                break

    return need_control
