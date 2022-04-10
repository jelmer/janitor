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

import datetime
from debian.changelog import Version
import json
import asyncpg
import logging
from contextlib import asynccontextmanager
from typing import Optional, Tuple, List, Any

from breezy import urlutils


class Database(object):
    def __init__(self, url):
        self.url = url
        self.pool = None

    @asynccontextmanager
    async def acquire(self):
        if self.pool is None:
            self.pool = await asyncpg.create_pool(self.url)
        async with self.pool.acquire() as conn:
            await conn.set_type_codec(
                "json", encoder=json.dumps, decoder=json.loads, schema="pg_catalog"
            )
            await conn.set_type_codec(
                "jsonb", encoder=json.dumps, decoder=json.loads, schema="pg_catalog"
            )
            await conn.set_type_codec(
                "debversion", format="text", encoder=str, decoder=Version
            )
            yield conn


def get_result_branch(result_branches, role):
    for entry in result_branches:
        if role == entry[0]:
            return entry[1:]
    raise KeyError



class Run(object):

    id: str
    times: Tuple[datetime.datetime, datetime.datetime]
    command: str
    description: Optional[str]
    package: str
    result_code: str
    main_branch_revision: Optional[bytes]
    revision: Optional[bytes]
    context: Optional[str]
    result: Optional[Any]
    suite: str
    instigated_context: Optional[str]
    vcs_type: str
    branch_url: str
    logfilenames: Optional[List[str]]
    review_status: str
    review_comment: Optional[str]
    worker_name: Optional[str]
    result_branches: Optional[List[Tuple[str, str, bytes, bytes]]]
    result_tags: Optional[List[Tuple[str, bytes]]]
    target_branch_url: Optional[str]
    change_set: Optional[str]

    __slots__ = [
        "id",
        "start_time",
        "finish_time",
        "command",
        "description",
        "package",
        "result_code",
        "value",
        "main_branch_revision",
        "revision",
        "context",
        "result",
        "suite",
        "instigated_context",
        "vcs_type",
        "branch_url",
        "logfilenames",
        "review_status",
        "review_comment",
        "worker_name",
        "result_branches",
        "result_tags",
        "target_branch_url",
        "change_set",
    ]

    def __init__(
        self,
        run_id,
        start_time,
        finish_time,
        command,
        description,
        package,
        result_code,
        value,
        main_branch_revision,
        revision,
        context,
        result,
        suite,
        instigated_context,
        vcs_type,
        branch_url,
        logfilenames,
        review_status,
        review_comment,
        worker_name,
        result_branches,
        result_tags,
        target_branch_url,
        change_set,
    ):
        self.id = run_id
        self.start_time = start_time
        self.finish_time = finish_time
        self.command = command
        self.description = description
        self.package = package
        self.result_code = result_code
        self.value = value
        self.main_branch_revision = main_branch_revision
        self.revision = revision
        self.context = context
        self.result = result
        self.suite = suite
        self.instigated_context = instigated_context
        self.vcs_type = vcs_type
        self.branch_url = branch_url
        self.logfilenames = logfilenames
        self.review_status = review_status
        self.review_comment = review_comment
        self.worker_name = worker_name
        if result_branches is None:
            self.result_branches = None
        else:
            self.result_branches = [
                (
                    role,
                    name,
                    br.encode("utf-8") if br else None,
                    r.encode("utf-8") if r else None,
                )
                for (role, name, br, r) in result_branches
            ]
        if result_tags is None:
            self.result_tags = result_tags
        else:
            self.result_tags = [(name, r.encode("utf-8")) for (name, r) in result_tags]
        self.target_branch_url = target_branch_url
        self.change_set = change_set

    @property
    def duration(self) -> datetime.timedelta:
        return self.finish_time - self.start_time

    def get_result_branch(self, role):
        return get_result_branch(self.result_branches, role)

    @classmethod
    def from_row(cls, row) -> "Run":
        return cls(
            run_id=row['id'],
            start_time=row['start_time'],
            finish_time=row['finish_time'],
            command=row['command'],
            description=row['description'],
            package=row['package'],
            result_code=row['result_code'],
            main_branch_revision=(row['main_branch_revision'].encode("utf-8") if row['main_branch_revision'] else None),
            revision=(row['revision'].encode("utf-8") if row['revision'] else None),
            context=row['context'],
            result=row['result'],
            value=row['value'],
            suite=row['suite'],
            instigated_context=row['instigated_context'],
            vcs_type=row['vcs_type'],
            branch_url=row['branch_url'],
            logfilenames=row['logfilenames'],
            review_status=row['review_status'],
            review_comment=row['review_comment'],
            worker_name=row['worker'],
            result_branches=row['result_branches'],
            result_tags=row['result_tags'],
            target_branch_url=row['target_branch_url'],
            change_set=row['change_set'],
        )

    def __eq__(self, other) -> bool:
        if isinstance(other, Run):
            return self.id == other.id
        return False

    def __lt__(self, other) -> bool:
        if not isinstance(other, type(self)):
            raise TypeError(other)
        return self.id < other.id


async def _iter_runs(
    conn: asyncpg.Connection,
    package: Optional[str] = None,
    run_id: Optional[str] = None,
    worker: Optional[str] = None,
    suite: Optional[str] = None,
    limit: Optional[int] = None,
):
    """Iterate over runs.

    Args:
      package: package to restrict to
    Returns:
      iterator over Run objects
    """
    query = """
SELECT
    id, command, start_time, finish_time, description, package,
    result_code,
    value, main_branch_revision, revision, context, result, suite,
    instigated_context, vcs_type, branch_url, logfilenames, review_status,
    review_comment, worker,
    array(SELECT row(role, remote_name, base_revision,
     revision) FROM new_result_branch WHERE run_id = id) AS result_branches,
    result_tags, target_branch_url, change_set
FROM
    run
LEFT JOIN
    debian_build ON debian_build.run_id = run.id
"""
    conditions = []
    args = []
    if package is not None:
        args.append(package)
        conditions.append("package = $%d" % len(args))
    if run_id is not None:
        args.append(run_id)
        conditions.append("id = $%d" % len(args))
    if worker is not None:
        args.append(worker)
        conditions.append("worker = $%d" % len(args))
    if suite is not None:
        args.append(suite)
        conditions.append("suite = $%d" % len(args))
    if conditions:
        query += " WHERE " + " AND ".join(conditions)
    query += "ORDER BY finish_time DESC"
    if limit:
        query += " LIMIT %d" % limit
    for row in await conn.fetch(query, *args):
        yield Run.from_row(row)


async def iter_publishable_suites(
    conn: asyncpg.Connection,
    package: str
) -> List[
    Tuple[
        str,
    ]
]:
    query = """
SELECT DISTINCT candidate.suite
FROM candidate
INNER JOIN package on package.name = candidate.package
LEFT JOIN policy ON
    policy.package = package.name AND
    policy.suite = candidate.suite
WHERE NOT package.removed AND package.name = $1
"""
    return [
        row[0] for row in await conn.fetch(query, package)
    ]


async def has_cotenants(
    conn: asyncpg.Connection, package: str, url: str
) -> Optional[bool]:
    url = urlutils.split_segment_parameters(url)[0].rstrip("/")
    rows = await conn.fetch(
        "SELECT name FROM package where "
        "branch_url = $1 or "
        "branch_url like $1 || ',branch=%' or "
        "branch_url like $1 || '/,branch=%'",
        url,
    )
    if len(rows) > 1:
        return True
    elif len(rows) == 1 and rows[0][0] == package:
        return False
    else:
        # Uhm, we actually don't really know
        logging.warning("Unable to figure out if %s has cotenants on %s", package, url)
        return None
