#!/usr/bin/python3
# Copyright (C) 2018-2022 Jelmer Vernooij <jelmer@jelmer.uk>
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

from typing import Optional

import aiohttp
from aiohttp import BasicAuth, web


async def is_worker(db, request: web.Request) -> Optional[str]:
    auth_header = request.headers.get(aiohttp.hdrs.AUTHORIZATION)
    if not auth_header:
        return None
    auth = BasicAuth.decode(auth_header=auth_header)
    async with db.acquire() as conn:
        val = await conn.fetchval(
            "select 1 from worker where name = $1 "
            "AND password = crypt($2, password)",
            auth.login,
            auth.password,
        )
        if val:
            return auth.login
    return None


async def check_worker_creds(db, request: web.Request) -> Optional[str]:
    auth_header = request.headers.get(aiohttp.hdrs.AUTHORIZATION)
    if not auth_header:
        raise web.HTTPUnauthorized(
            text="worker login required",
            headers={"WWW-Authenticate": 'Basic Realm="Debian Janitor"'},
        )
    login = await is_worker(db, request)
    if not login:
        raise web.HTTPUnauthorized(
            text="worker login required",
            headers={"WWW-Authenticate": 'Basic Realm="Debian Janitor"'},
        )

    return login
