#!/usr/bin/python3, Suite
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

__all__ = [
    "Config",
    "Suite",
    "read_config",
    "get_suite_config",
]

from google.protobuf import text_format  # type: ignore

from .config_pb2 import Config, Suite


def read_config(f):
    return text_format.Parse(f.read(), Config())


def get_suite_config(config: Config, name: str) -> Suite:
    for s in config.suite:
        if s.name == name:
            return s
    raise KeyError(name)
