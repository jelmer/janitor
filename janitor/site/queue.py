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

from janitor import state


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


async def iter_queue_with_last_run(
    db: state.Database, limit: Optional[int] = None
):
    query = """
SELECT
      queue.package AS package,
      queue.command AS command,
      queue.context AS context,
      queue.id AS id,
      queue.estimated_duration AS estimated_duration,
      queue.suite AS suite,
      queue.refresh AS refresh,
      queue.requestor AS requestor,
      run.id AS log_id,
      run.result_code AS result_code
  FROM
      queue
  LEFT JOIN
      run
  ON
      run.id = (
          SELECT id FROM run WHERE
            package = queue.package AND run.suite = queue.suite
          ORDER BY run.start_time desc LIMIT 1)
  ORDER BY
  queue.bucket ASC,
  queue.priority ASC,
  queue.id ASC
"""
    if limit:
        query += " LIMIT %d" % limit
    async with db.acquire() as conn:
        for row in await conn.fetch(query):
            yield row


async def get_queue(
    db: state.Database, limit: Optional[int] = None
) -> AsyncIterator[
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
    async for row in iter_queue_with_last_run(db, limit=limit):
        command = shlex.split(row["command"])
        while command and '=' in command[0]:
            command.pop(0)
        expecting = None
        if command:
            description = ' '.join(command)
        else:
            description = 'no-op'
        if expecting is not None:
            description += ", " + expecting
        if row["refresh"]:
            description += " (from scratch)"
        yield (
            row["id"],
            row["package"],
            row["requestor"],
            row["suite"],
            description,
            row["estimated_duration"],
            row["log_id"],
            row["result_code"],
        )


async def get_buckets(db):
    async with db.acquire() as conn:
        for row in await conn.fetch("SELECT bucket, count(*) FROM queue GROUP BY bucket ORDER BY bucket ASC"):
            yield row[0], row[1]


async def write_queue(
    client,
    db: state.Database,
    limit=None,
    is_admin=False,
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
    return {
        "queue": get_queue(db, limit),
        "buckets": get_buckets(db),
        "active_queue_ids": active_queue_ids,
        "processing": processing,
        "avoid_hosts": avoid_hosts,
        "rate_limit_hosts": rate_limit_hosts,
    }
