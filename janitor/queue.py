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

import asyncpg
from typing import Optional, Set


class QueueItem(object):

    __slots__ = [
        "id",
        "branch_url",
        "subpath",
        "package",
        "context",
        "command",
        "estimated_duration",
        "suite",
        "refresh",
        "requestor",
        "vcs_type",
        "upstream_branch_url",
    ]

    def __init__(
        self,
        id,
        branch_url,
        subpath,
        package,
        context,
        command,
        estimated_duration,
        suite,
        refresh,
        requestor,
        vcs_type,
        upstream_branch_url,
    ):
        self.id = id
        self.package = package
        self.branch_url = branch_url
        self.subpath = subpath
        self.context = context
        self.command = command
        self.estimated_duration = estimated_duration
        self.suite = suite
        self.refresh = refresh
        self.requestor = requestor
        self.vcs_type = vcs_type
        self.upstream_branch_url = upstream_branch_url

    @classmethod
    def from_row(cls, row) -> "QueueItem":
        return cls(
            id=row['id'],
            branch_url=row['branch_url'],
            subpath=row['subpath'],
            package=row['package'],
            context=row['context'],
            command=row['command'],
            estimated_duration=row['estimated_duration'],
            suite=row['suite'],
            refresh=row['refresh'],
            requestor=row['requestor'],
            vcs_type=row['vcs_type'],
            upstream_branch_url=row['upstream_branch_url'],
        )

    def _tuple(self):
        return (
            self.id,
            self.branch_url,
            self.subpath,
            self.package,
            self.context,
            self.command,
            self.estimated_duration,
            self.suite,
            self.refresh,
            self.requestor,
            self.vcs_type,
            self.upstream_branch_url,
        )

    def __eq__(self, other):
        if isinstance(other, QueueItem):
            return self.id == other.id
        return False

    def __lt__(self, other):
        return self.id < other.id

    def __hash__(self):
        return hash((type(self), self.id))


async def get_queue_position(conn: asyncpg.Connection, suite, package):
    row = await conn.fetchrow(
        "SELECT position, wait_time FROM queue_positions "
        "WHERE package = $1 AND suite = $2",
        package, suite)
    if not row:
        return (None, None)
    return row


async def get_queue_item(conn: asyncpg.Connection, queue_id: int):
    query = """
SELECT
    package.branch_url AS branch_url,
    package.subpath AS subpath,
    queue.package AS package,
    queue.command AS command,
    queue.context AS context,
    queue.id AS id,
    queue.estimated_duration AS estimated_duration,
    queue.suite AS suite,
    queue.refresh AS refresh,
    queue.requestor AS requestor,
    package.vcs_type AS vcs_type,
    upstream.upstream_branch_url AS upstream_branch_url
FROM
    queue
LEFT JOIN package ON package.name = queue.package
LEFT OUTER JOIN upstream ON upstream.name = package.name
WHERE queue.id = $1
"""
    row = await conn.fetchrow(query, queue_id)
    if row:
        return QueueItem.from_row(row)
    return None


async def iter_queue(conn: asyncpg.Connection, limit: Optional[int] = None, package: Optional[str] = None, campaign: Optional[str] = None, avoid_hosts: Optional[Set[str]] = None):
    query = """
SELECT
    package.branch_url AS branch_url,
    package.subpath AS subpath,
    queue.package AS package,
    queue.command AS command,
    queue.context AS context,
    queue.id AS id,
    queue.estimated_duration AS estimated_duration,
    queue.suite AS suite,
    queue.refresh AS refresh,
    queue.requestor AS requestor,
    package.vcs_type AS vcs_type,
    upstream.upstream_branch_url AS upstream_branch_url
FROM
    queue
LEFT JOIN package ON package.name = queue.package
LEFT OUTER JOIN upstream ON upstream.name = package.name
"""
    conditions = []
    args = []
    if package:
        args.append(package)
        conditions.append("queue.package = $%d" % len(args))
    if campaign:
        args.append(campaign)
        conditions.append("queue.suite = $%d" % len(args))

    if avoid_hosts:
        for host in avoid_hosts:
            assert isinstance(host, str), "not a string: %r" % host
            args.append(host)
            conditions.append("package.branch_url NOT LIKE CONCAT('%%/', $%d::text, '/%%')" % len(args))

    if conditions:
        query += " WHERE " + " AND ".join(conditions)

    query += """
ORDER BY
queue.bucket ASC,
queue.priority ASC,
queue.id ASC
"""
    if limit:
        query += " LIMIT %d" % limit
    for row in await conn.fetch(query, *args):
        yield QueueItem.from_row(row)
