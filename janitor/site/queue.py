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
from janitor.worker import changer_subcommand


def lintian_tag_link(tag: str) -> str:
    return '<a href="https://lintian.debian.org/tags/%s.html">%s</a>' % (tag, tag)


class RunnerProcessingUnavailable(Exception):
    """Raised when unable to get processing data for runner."""


def get_processing(answer: Dict[str, Any]) -> Iterator[Dict[str, Any]]:
    for entry in answer["processing"]:
        entry = dict(entry.items())
        if entry.get("estimated_duration"):
            entry["estimated_duration"] = timedelta(seconds=entry["estimated_duration"])
        if entry.get("start_time"):
            entry["start_time"] = datetime.fromisoformat(entry["start_time"])
            entry["current_duration"] = datetime.now() - entry["start_time"]
        if entry.get('last-keepalive'):
            entry["keepalive_age"] = datetime.now() - datetime.fromisoformat(entry['last-keepalive'])
        yield entry


async def iter_queue_with_last_run(
    db: state.Database, limit: Optional[int] = None
) -> AsyncIterator[Tuple[state.QueueItem, Optional[str], Optional[str]]]:
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
    db: state.Database, only_command: Optional[str] = None, limit: Optional[int] = None
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
    async for row in (iter_queue_with_last_run(db, limit=limit)):
        command = shlex.split(row["command"])
        if only_command is not None and command != only_command:
            continue
        expecting = None
        if command[0] == "new-upstream":
            if "--snapshot" in command:
                description = "New upstream snapshot"
            else:
                description = "New upstream"
                if row["context"]:
                    expecting = (
                        "expecting to merge <a href='https://qa.debian.org"
                        "/cgi-bin/watch?pkg=%s'>%s</a>"
                        % (row["package"], row["context"])
                    )
        elif command[0] == "lintian-brush":
            description = "Lintian fixes"
            if row["context"]:
                expecting = "expecting to fix: " + ", ".join(
                    map(lintian_tag_link, row["context"].split(" "))
                )
        else:
            cs = changer_subcommand(command[0])
            description = cs.describe_command(command)
        if only_command is not None:
            description = expecting or ""
        elif expecting is not None:
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


async def write_queue(
    client,
    db: state.Database,
    only_command=None,
    limit=None,
    is_admin=False,
    queue_status=None,
):
    if queue_status:
        processing = get_processing(queue_status)
        active_queue_ids = set([p["queue_id"] for p in queue_status["processing"]])
    else:
        processing = iter([])
        active_queue_ids = set()
    return {
        "queue": get_queue(db, only_command, limit),
        "active_queue_ids": active_queue_ids,
        "processing": processing,
    }
