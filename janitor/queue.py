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

from datetime import timedelta
from typing import Optional, Set, List, Any


import asyncpg


class QueueItem(object):

    __slots__ = [
        "id",
        "package",
        "context",
        "command",
        "estimated_duration",
        "campaign",
        "refresh",
        "requestor",
        "change_set",
    ]

    def __init__(
        self,
        id,
        package,
        context,
        command,
        estimated_duration,
        campaign,
        refresh,
        requestor,
        change_set,
    ):
        self.id = id
        self.package = package
        self.context = context
        self.command = command
        self.estimated_duration = estimated_duration
        self.campaign = campaign
        self.refresh = refresh
        self.requestor = requestor
        self.change_set = change_set

    @classmethod
    def from_row(cls, row) -> "QueueItem":
        return cls(
            id=row['id'],
            package=row['package'],
            context=row['context'],
            command=row['command'],
            estimated_duration=row['estimated_duration'],
            campaign=row['campaign'],
            refresh=row['refresh'],
            requestor=row['requestor'],
            change_set=row['change_set'],
        )

    @property
    def codebase(self):
        return self.package

    def __eq__(self, other):
        if isinstance(other, QueueItem):
            return self.id == other.id
        return False

    def __lt__(self, other):
        return self.id < other.id

    def __hash__(self):
        return hash((type(self), self.id))


class Queue(object):

    def __init__(self, conn: asyncpg.Connection):
        self.conn = conn

    async def get_position(self, campaign, package):
        row = await self.conn.fetchrow(
            "SELECT position, wait_time FROM queue_positions "
            "WHERE package = $1 AND suite = $2",
            package, campaign)
        if not row:
            return (None, None)
        return row

    async def get_item(self, queue_id: int):
        query = """
SELECT
    queue.package AS package,
    queue.command AS command,
    queue.context AS context,
    queue.id AS id,
    queue.estimated_duration AS estimated_duration,
    queue.suite AS campaign,
    queue.refresh AS refresh,
    queue.requestor AS requestor,
    queue.change_set AS change_set
FROM
    queue
WHERE queue.id = $1
"""
        row = await self.conn.fetchrow(query, queue_id)
        if row:
            return QueueItem.from_row(row)
        return None

    async def next_item(self, package: Optional[str] = None,
                        campaign: Optional[str] = None,
                        exclude_hosts: Optional[Set[str]] = None,
                        assigned_queue_items: Optional[Set[int]] = None):
        query = """
SELECT
    queue.package AS package,
    queue.command AS command,
    queue.context AS context,
    queue.id AS id,
    queue.estimated_duration AS estimated_duration,
    queue.suite AS campaign,
    queue.refresh AS refresh,
    queue.requestor AS requestor,
    queue.change_set AS change_set,
    package.vcs_type AS vcs_type,
    package.branch_url AS branch_url,
    package.subpath AS subpath
FROM
    queue
LEFT JOIN package ON package.name = queue.package
"""
        conditions = []
        args: List[Any] = []
        if assigned_queue_items:
            args.append(assigned_queue_items)
            conditions.append("NOT (queue.id = ANY($%d::int[]))" % len(args))
        if package:
            args.append(package)
            conditions.append("queue.package = $%d" % len(args))
        if campaign:
            args.append(campaign)
            conditions.append("queue.suite = $%d" % len(args))
        if exclude_hosts:
            args.append(exclude_hosts)
            # TODO(jelmer): Use package.hostname when kali upgrades to postgres 12+
            conditions.append(
                "NOT (package.branch_url IS NOT NULL AND "
                "SUBSTRING(package.branch_url from '.*://(?:[^/@]*@)?([^/]*)') = ANY($%d::text[]))")

        if conditions:
            query += " WHERE " + " AND ".join(conditions)

        query += """
ORDER BY
queue.bucket ASC,
queue.priority ASC,
queue.id ASC
LIMIT 1
"""
        row = await self.conn.fetchrow(query, *args)
        if row is None:
            return None, None
        if row['vcs_type']:
            vcs_info = {
                'vcs_type': row['vcs_type'],
                'branch_url': row['branch_url'],
                'subpath': row['subpath'],
            }
        else:
            vcs_info = None
        return QueueItem.from_row(row), vcs_info

    async def iter_queue(self, limit: Optional[int] = None,
                         package: Optional[str] = None, campaign: Optional[str] = None):
        query = """
SELECT
    queue.package AS package,
    queue.command AS command,
    queue.context AS context,
    queue.id AS id,
    queue.estimated_duration AS estimated_duration,
    queue.suite AS campaign,
    queue.refresh AS refresh,
    queue.requestor AS requestor,
    queue.change_set AS change_set
FROM
    queue
"""
        conditions = []
        args: List[Any] = []
        if package:
            args.append(package)
            conditions.append("queue.package = $%d" % len(args))
        if campaign:
            args.append(campaign)
            conditions.append("queue.suite = $%d" % len(args))

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
        for row in await self.conn.fetch(query, *args):
            yield QueueItem.from_row(row)

    async def add(
            self,
            package: str,
            codebase: str,
            command: str,
            campaign: str,
            change_set: Optional[str] = None,
            offset: float = 0.0,
            bucket: str = "default",
            context: Optional[str] = None,
            estimated_duration: Optional[timedelta] = None,
            refresh: bool = False,
            requestor: Optional[str] = None) -> int:
        return await self.conn.fetchval(
            "INSERT INTO queue "
            "(package, command, priority, bucket, context, "
            "estimated_duration, suite, refresh, requestor, change_set, "
            "codebase) VALUES ($1, $2, "
            "(SELECT COALESCE(MIN(priority), 0) FROM queue)"
            + " + $3, $4, $5, $6, $7, $8, $9, $10, $11) "
            "ON CONFLICT (package, suite, coalesce(change_set, ''::text)) "
            "DO UPDATE SET "
            "context = EXCLUDED.context, priority = EXCLUDED.priority, "
            "bucket = EXCLUDED.bucket, "
            "estimated_duration = EXCLUDED.estimated_duration, "
            "refresh = EXCLUDED.refresh, requestor = EXCLUDED.requestor, "
            "command = EXCLUDED.command, codebase = EXCLUDED.codebase "
            "WHERE queue.bucket >= EXCLUDED.bucket OR "
            "(queue.bucket = EXCLUDED.bucket AND "
            "queue.priority >= EXCLUDED.priority) RETURNING id",
            package,
            command,
            offset,
            bucket,
            context,
            estimated_duration,
            campaign,
            refresh,
            requestor,
            change_set,
            codebase,
        )

    async def get_buckets(self):
        return await self.conn.fetch(
            "SELECT bucket, count(*) FROM queue GROUP BY bucket "
            "ORDER BY bucket ASC")
