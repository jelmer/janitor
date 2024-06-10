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

__all__ = [
    "Config",
    "Campaign",
    "AptRepository",
    "read_config",
    "get_campaign_config",
    "get_distribution",
]

import sys

from google.protobuf import text_format  # type: ignore

from . import config_pb2

if sys.version_info >= (3, 10):
    from typing import TypeAlias

    Config: TypeAlias = config_pb2.Config
    Campaign: TypeAlias = config_pb2.Campaign
    AptRepository: TypeAlias = config_pb2.AptRepository
else:
    Config = config_pb2.Config
    Campaign = config_pb2.Campaign
    AptRepository = config_pb2.AptRepository


def read_config(f):
    return text_format.Parse(f.read(), config_pb2.Config())


def get_distribution(config: Config, name: str) -> config_pb2.Distribution:
    for d in config.distribution:
        if d.name == name:
            return d
    raise KeyError(name)


def get_campaign_config(config: config_pb2.Config, name: str) -> config_pb2.Campaign:
    for c in config.campaign:
        if c.name == name:
            return c
    raise KeyError(name)


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("config_file", type=str, help="Configuration file to read")
    args = parser.parse_args()
    with open(args.config_file) as f:
        config = read_config(f)
