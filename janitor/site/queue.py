#!/usr/bin/python3
# Copyright (C) 2019 Jelmer Vernooij <jelmer@jelmer.uk>
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

from datetime import datetime, timedelta
import shlex
from typing import AsyncIterator, Tuple, Optional, Dict, Any, Iterator

import asyncpg
from ..queue import Queue


class RunnerProcessingUnavailable(Exception):
    """Raised when unable to get processing data for runner."""


def get_processing(answer: Dict[str, Any]) -> Iterator[Dict[str, Any]]:
    for entry in answer["processing"]:
        entry = dict(entry.items())
        if entry.get("estimated_duration"):
            entry["estimated_duration"] = timedelta(seconds=entry["estimated_duration"])
        if entry.get("start_time"):
            entry["start_time"] = datetime.fromisoformat(entry["start_time"])
            entry["current_duration"] = datetime.utcnow() - entry["start_time"]
        if entry.get('last-keepalive'):
            entry["keepalive_age"] = timedelta(seconds=entry["keepalive_age"])
        yield entry


async def iter_queue_items_with_last_run(db: asyncpg.pool.Pool, queue: Queue, limit: int):
    with db.acquire() as conn:
        items = []
        qs = []
        vals = []
        async for item in queue.iter_queue(limit=limit):
            items.append(item)
            vals.append(item.package)
            vals.append(item.suite)
            qs.append(
                '(package = $%d AND suite = $%d)' % (len(vals) - 1, len(vals)))

        runs = {}
        if qs:
            for row in await conn.fetch(
                    'SELECT package, suite, id, result_code FROM last_runs '
                    'WHERE ($1)', ' OR '.join(qs)):
                runs[(row['package'], row['suite'])] = dict(row)

    for item in items:
        yield (item, runs[(item.package, item.suite)])


async def get_queue(db: asyncpg.pool.Pool, queue: Queue, limit: int) -> AsyncIterator[
    Tuple[
        int,
        str,
        Optional[str],
        str,
        str,
        Optional[timedelta],
        Optional[str],
        Optional[str],
    ]
]:
    async for queue_item, row in iter_queue_items_with_last_run(db, queue, limit=limit):
        command = shlex.split(queue_item.command)
        while command and '=' in command[0]:
            command.pop(0)
        expecting = None
        if command:
            description = ' '.join(command)
        else:
            description = 'no-op'
        if expecting is not None:
            description += ", " + expecting
        if queue_item.refresh:
            description += " (from scratch)"
        yield (queue_item, row, description)


async def write_queue(
    client,
    db: asyncpg.pool.Pool,
    limit=None,
    queue_status=None,
):
    if queue_status:
        processing = get_processing(queue_status)
        active_queue_ids = set([p["queue_id"] for p in queue_status["processing"]])
        avoid_hosts = queue_status["avoid_hosts"]
        rate_limit_hosts = {
            host: datetime.fromisoformat(ts)
            for (host, ts) in queue_status["rate_limit_hosts"].items()}
    else:
        processing = iter([])
        active_queue_ids = set()
        avoid_hosts = None
        rate_limit_hosts = None
    with db.acquire() as conn:
        queue = Queue(conn)
        return {
            "queue": get_queue(db, queue, limit),
            "buckets": queue.get_buckets(),
            "active_queue_ids": active_queue_ids,
            "processing": processing,
            "avoid_hosts": avoid_hosts,
            "rate_limit_hosts": rate_limit_hosts,
        }
