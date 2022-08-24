#!/usr/bin/python
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


from breezy.transport import http as _mod_http
from breezy.transport.http import urllib as _mod_urllib

import shlex
from .compat import shlex_join


version_info = (0, 1, 0)
version_string = ".".join(map(str, version_info))


def user_agent() -> str:
    return "Debian-Janitor/%s Bot (+https://janitor.debian.net/contact/)" % (
        version_string
    )


_mod_http.default_user_agent = user_agent
_mod_urllib.AbstractHTTPHandler._default_headers["User-agent"] = user_agent()


CAMPAIGN_REGEX = "[a-z0-9-]+"


def splitout_env(command):
    args = shlex.split(command)
    env = {}
    while len(args) > 0 and '=' in args[0]:
        (key, value) = args.pop(0).split('=', 1)
        env[key] = value
    return env, shlex_join(args)
