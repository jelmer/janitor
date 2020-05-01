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

"""Wrapper around the vcswatch table in UDD."""

import asyncpg


async def connect_udd_mirror() -> asyncpg.Connection:
    """Connect to the public UDD mirror."""
    return await asyncpg.connect(
        database="udd",
        user="udd-mirror",
        password="udd-mirror",
        port=5432,
        host="udd-mirror.debian.net")


class UDD(object):

    @classmethod
    async def public_udd_mirror(cls) -> 'UDD':
        return cls(await connect_udd_mirror())

    def __init__(self, conn) -> None:
        self._conn = conn

    async def fetch(self, *args, **kwargs):
        return await self._conn.fetch(*args, **kwargs)
