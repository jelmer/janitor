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


from ._common import config as _config_rs

Config = _config_rs.Config
Campaign = _config_rs.Campaign
AptRepository = _config_rs.AptRepository
read_config = _config_rs.read_config
get_distribution = _config_rs.get_distribution
get_campaign_config = _config_rs.get_campaign_config


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("config_file", type=str, help="Configuration file to read")
    args = parser.parse_args()
    with open(args.config_file) as f:
        config = read_config(f)
