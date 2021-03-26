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
import logging
import shlex
import asyncpg
from contextlib import asynccontextmanager
from typing import Optional, Tuple, List, Any, Union, Callable, AsyncIterable, Set, Dict
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


class Run(object):

    id: str
    times: Tuple[datetime.datetime, datetime.datetime]
    command: str
    description: Optional[str]
    package: str
    build_version: Optional[Version]
    build_distribution: Optional[str]
    result_code: str
    branch_name: Optional[str]
    main_branch_revision: Optional[bytes]
    revision: Optional[bytes]
    context: Optional[str]
    result: Optional[Any]
    suite: str
    instigated_context: Optional[str]
    branch_url: str
    logfilenames: Optional[List[str]]
    review_status: str
    review_comment: Optional[str]
    worker_name: Optional[str]
    result_branches: Optional[List[Tuple[str, str, bytes, bytes]]]
    result_tags: Optional[List[Tuple[str, bytes]]]

    __slots__ = [
        "id",
        "times",
        "command",
        "description",
        "package",
        "build_version",
        "build_distribution",
        "result_code",
        "branch_name",
        "main_branch_revision",
        "revision",
        "context",
        "result",
        "suite",
        "instigated_context",
        "branch_url",
        "logfilenames",
        "review_status",
        "review_comment",
        "worker_name",
        "result_branches",
        "result_tags",
    ]

    def __init__(
        self,
        run_id,
        times,
        command,
        description,
        package,
        build_version,
        build_distribution,
        result_code,
        branch_name,
        main_branch_revision,
        revision,
        context,
        result,
        suite,
        instigated_context,
        branch_url,
        logfilenames,
        review_status,
        review_comment,
        worker_name,
        result_branches,
        result_tags,
    ):
        self.id = run_id
        self.times = times
        self.command = command
        self.description = description
        self.package = package
        self.build_version = build_version
        self.build_distribution = build_distribution
        self.result_code = result_code
        self.branch_name = branch_name
        self.main_branch_revision = main_branch_revision
        self.revision = revision
        self.context = context
        self.result = result
        self.suite = suite
        self.instigated_context = instigated_context
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

    @property
    def duration(self) -> datetime.timedelta:
        return self.times[1] - self.times[0]

    def age(self):
        return datetime.datetime.now() - self.times[1]

    def has_artifacts(self):
        # Reasonable proxy, for now?
        return self.result_code == "success"

    def get_result_branch(self, role):
        for entry in self.result_branches:
            if role == entry[0]:
                return entry[1:]
        raise KeyError

    @classmethod
    def from_row(cls, row) -> "Run":
        return cls(
            run_id=row['id'],
            times=(row['start_time'], row['finish_time']),
            command=row['command'],
            description=row[4],
            package=row[5],
            build_version=Version(row[6]) if row[6] else None,
            build_distribution=row[7],
            result_code=(row[8] if row[8] else None),
            branch_name=row[9],
            main_branch_revision=(row[10].encode("utf-8") if row[10] else None),
            revision=(row[11].encode("utf-8") if row[11] else None),
            context=row[12],
            result=row[13],
            suite=row[14],
            instigated_context=row[15],
            branch_url=row[16],
            logfilenames=row[17],
            review_status=row[18],
            review_comment=row[19],
            worker_name=row[20],
            result_branches=row[21],
            result_tags=row[22],
        )

    def __len__(self) -> int:
        return len(self.__slots__)

    def __tuple__(self):
        return (
            self.id,
            self.times,
            self.command,
            self.description,
            self.package,
            self.build_version,
            self.build_distribution,
            self.result_code,
            self.branch_name,
            self.main_branch_revision,
            self.revision,
            self.context,
            self.result,
            self.suite,
            self.instigated_context,
            self.branch_url,
            self.logfilenames,
            self.review_status,
            self.review_comment,
            self.worker_name,
            self.result_branches,
            self.result_tags,
        )

    def __eq__(self, other) -> bool:
        if isinstance(other, Run):
            return self.__tuple__() == other.__tuple__()
        if isinstance(other, tuple):
            return self.id == other[0]
        return False

    def __lt__(self, other) -> bool:
        if not isinstance(other, type(self)):
            raise TypeError(other)
        return self.__tuple__() < other.__tuple__()

    def __getitem__(self, i):
        if isinstance(i, slice):
            return tuple(self).__getitem__(i)
        return getattr(self, self.__slots__[i])


async def get_unchanged_run(conn: asyncpg.Connection, package, main_branch_revision):
    query = """
SELECT
    id, command, start_time, finish_time, description, package,
    debian_build.version AS build_version,
    debian_build.distribution AS build_distribution, result_code,
    branch_name, main_branch_revision, revision, context, result, suite,
    instigated_context, branch_url, logfilenames, review_status,
    review_comment, worker,
    array(SELECT row(role, remote_name, base_revision, revision) FROM
     new_result_branch WHERE run_id = id),
    result_tags
FROM
    last_runs
LEFT JOIN
    debian_build ON debian_build.run_id = last_runs.id
WHERE
    suite = 'unchanged' AND revision = $1 AND
    package = $2 AND
    result_code = 'success'
ORDER BY finish_time DESC
"""
    if isinstance(main_branch_revision, bytes):
        main_branch_revision = main_branch_revision.decode("utf-8")
    row = await conn.fetchrow(query, main_branch_revision, package)
    if row is not None:
        return Run.from_row(row)
    return None


async def iter_runs(
    db: Database,
    package: Optional[str] = None,
    run_id: Optional[str] = None,
    worker: Optional[str] = None,
    limit: Optional[int] = None,
):
    async with db.acquire() as conn:
        async for run in _iter_runs(
            conn, package=package, run_id=run_id, worker=worker, limit=limit
        ):
            yield run


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
    debian_build.version AS build_version,
    debian_build.distribution AS build_distribution, result_code,
    branch_name, main_branch_revision, revision, context, result, suite,
    instigated_context, branch_url, logfilenames, review_status,
    review_comment, worker,
    array(SELECT row(role, remote_name, base_revision,
     revision) FROM new_result_branch WHERE run_id = id),
    result_tags
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


async def get_run(conn: asyncpg.Connection, run_id, package=None):
    async for run in _iter_runs(conn, run_id=run_id, package=package):
        return run
    else:
        return None


async def iter_proposals(conn: asyncpg.Connection, package=None, suite=None):
    args = []
    query = """
SELECT
    DISTINCT ON (merge_proposal.url)
    merge_proposal.package, merge_proposal.url, merge_proposal.status,
    run.suite
FROM
    merge_proposal
LEFT JOIN run
ON merge_proposal.revision = run.revision AND run.result_code = 'success'
"""
    if package is not None:
        if isinstance(package, list):
            args.append(package)
            query += " WHERE run.package = ANY($1::text[])"
        else:
            args.append(package)
            query += " WHERE run.package = $1"
        if suite:
            query += " AND run.suite = $2"
            args.append(suite)
    elif suite:
        args.append(suite)
        query += " WHERE run.suite = $1"
    query += " ORDER BY merge_proposal.url, run.finish_time DESC"
    ret = []
    for package, url, status, mp_suite in await conn.fetch(query, *args):
        if suite is None or mp_suite == suite:
            ret.append((package, url, status))
    return ret


async def iter_proposals_with_run(
    conn: asyncpg.Connection, package: Optional[str] = None, suite: Optional[str] = None
):
    args = []
    query = """
SELECT
    DISTINCT ON (merge_proposal.url)
    run.id AS id,
    run.command AS command,
    run.start_time AS start_time,
    run.finish_time As finish_time,
    run.description AS description,
    run.package AS package,
    debian_build.version AS build_version,
    debian_build.distribution AS build_distribution,
    run.result_code,
    run.branch_name,
    run.main_branch_revision,
    run.revision,
    run.context,
    run.result,
    run.suite,
    run.instigated_context,
    run.branch_url,
    run.logfilenames,
    run.review_status,
    run.review_comment,
    run.worker,
    array(SELECT row(role, remote_name, base_revision,
     revision) FROM new_result_branch WHERE run_id = id),
    run.result_tags,
    merge_proposal.url, merge_proposal.status
FROM
    merge_proposal
LEFT JOIN new_result_branch ON
    new_result_branch.revision = merge_proposal.revision
LEFT JOIN run ON run.id = new_result_branch.run_id
LEFT JOIN debian_build ON run.id = debian_build.run_id
"""
    if package:
        if isinstance(package, list):
            args.append(package)
            query += " WHERE run.package = ANY($1::text[])"
        else:
            args.append(package)
            query += " WHERE run.package = $1"
        if suite:
            query += " AND run.suite = $2"
            args.append(suite)
    elif suite:
        args.append(suite)
        query += " WHERE run.suite = $1"
    query += " ORDER BY merge_proposal.url, run.finish_time DESC"
    for row in await conn.fetch(query, *args):
        yield Run.from_row(row[:23]), row[23], row[24]


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
            command=shlex.split(row['command']),
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


async def iter_queue(conn: asyncpg.Connection, limit=None):
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
ORDER BY
queue.bucket ASC,
queue.priority ASC,
queue.id ASC
"""
    if limit:
        query += " LIMIT %d" % limit
    for row in await conn.fetch(query):
        yield QueueItem.from_row(row)


async def iter_previous_runs(
    conn: asyncpg.Connection, package: str, suite: str
) -> AsyncIterable[Run]:
    for row in await conn.fetch(
        """
SELECT
  id,
  command,
  start_time,
  finish_time,
  description,
  package,
  debian_build.version AS build_version,
  debian_build.distribution AS build_distribution,
  result_code,
  branch_name,
  main_branch_revision,
  revision,
  context,
  result,
  suite,
  instigated_context,
  branch_url,
  logfilenames,
  review_status,
  review_comment,
  worker,
  array(SELECT row(role, remote_name, base_revision,
   revision) FROM new_result_branch WHERE run_id = id),
  result_tags
FROM
  run
LEFT JOIN debian_build ON run.id = debian_build.run_id
WHERE
  package = $1 AND suite = $2
ORDER BY start_time DESC
""",
        package,
        suite,
    ):
        yield Run.from_row(row)


async def get_last_unabsorbed_run(
    conn: asyncpg.Connection, package: str, suite: str
) -> Optional[Run]:
    args = []
    query = """
SELECT
  id,
  command,
  start_time,
  finish_time,
  description,
  package,
  debian_build.version AS build_version,
  debian_build.distribution AS build_distribution,
  result_code,
  branch_name,
  main_branch_revision,
  revision,
  context,
  result,
  suite,
  instigated_context,
  branch_url,
  logfilenames,
  review_status,
  review_comment,
  worker,
  array(SELECT row(role, remote_name, base_revision,
   revision) FROM new_result_branch WHERE run_id = id),
  result_tags
FROM
  last_unabsorbed_runs
LEFT JOIN debian_build ON last_unabsorbed_runs.id = debian_build.run_id
WHERE package = $1 AND suite = $2
ORDER BY package, suite DESC, start_time DESC
LIMIT 1
"""
    args = [package, suite]
    row = await conn.fetchrow(query, *args)
    if row is None:
        return None
    return Run.from_row(row)


async def iter_last_unabsorbed_runs(
    conn: asyncpg.Connection, suite=None, packages=None
):
    query = """
SELECT DISTINCT ON (package)
  id,
  command,
  start_time,
  finish_time,
  description,
  package,
  debian_build.version AS build_version,
  debian_build.distribution AS build_distribution,
  result_code,
  branch_name,
  main_branch_revision,
  revision,
  context,
  result,
  suite,
  instigated_context,
  branch_url,
  logfilenames,
  review_status,
  review_comment,
  worker,
  array(SELECT row(role, remote_name, base_revision,
   revision) FROM new_result_branch WHERE run_id = id),
  result_tags
FROM
  last_unabsorbed_runs
LEFT JOIN debian_build ON last_unabsorbed_runs.id = debian_build.run_id
"""
    args = []
    if suite is not None or packages is not None:
        query += " WHERE "
    if suite is not None:
        query += "suite = $1"
        args.append(suite)
    if packages is not None:
        if suite is not None:
            query += "  AND "
        args.append(packages)
        query += "package = ANY($%d::text[])" % len(args)

    query += """
ORDER BY package, suite, start_time DESC
"""
    for row in await conn.fetch(query, *args):
        yield Run.from_row(row)


async def iter_last_runs(
    conn: asyncpg.Connection,
    result_code: Optional[str] = None,
    suite: Optional[str] = None,
    main_branch_revision: Optional[bytes] = None,
) -> AsyncIterable[Run]:
    query = """
SELECT
  id,
  command,
  start_time,
  finish_time,
  description,
  package,
  debian_build.version AS build_version,
  debian_build.distribution AS build_distribution,
  result_code,
  branch_name,
  main_branch_revision,
  revision,
  context,
  result,
  suite,
  instigated_context,
  branch_url,
  logfilenames,
  review_status,
  review_comment,
  worker,
  array(SELECT row(role, remote_name, base_revision,
   revision) FROM new_result_branch WHERE run_id = id),
  result_tags
FROM last_runs
LEFT JOIN debian_build ON last_runs.id = debian_build.run_id
"""
    where = []
    args: List[Any] = []
    if result_code is not None:
        args.append(result_code)
        where.append("result_code = $%d" % len(args))
    if suite:
        args.append(suite)
        where.append("suite = $%d" % len(args))
    if main_branch_revision:
        args.append(main_branch_revision)
        where.append("main_branch_revision = $%d" % len(args))
    if where:
        query += " WHERE " + " AND ".join(where)
    query += " ORDER BY start_time DESC"
    async with conn.transaction():
        async for row in conn.cursor(query, *args):
            yield Run.from_row(row)


async def iter_publish_ready(
    conn: asyncpg.Connection,
    suites: Optional[List[str]] = None,
    review_status: Optional[Union[str, List[str]]] = None,
    limit: Optional[int] = None,
    publishable_only: bool = False,
) -> AsyncIterable[
    Tuple[
        Run,
        int,
        str,
        List[str],
        str,
        str,
        List[Tuple[str, str, bytes, bytes, Optional[str], Optional[int]]],
    ]
]:
    args: List[Any] = []
    query = """
SELECT * FROM publish_ready
"""
    conditions = []
    if suites is not None:
        conditions.append("suite = ANY($1::text[])")
        args.append(suites)
    if review_status is not None:
        if not isinstance(review_status, list):
            review_status = [review_status]
        args.append(review_status)
        conditions.append("review_status = ANY($%d::review_status[])" % (len(args),))

    publishable_condition = (
        "exists (select from unnest(unpublished_branches) where "
        "mode in ('propose', 'attempt-push', 'push-derived', 'push'))"
    )

    order_by = []

    if publishable_only:
        conditions.append(publishable_condition)
    else:
        order_by.append(publishable_condition + " DESC")

    if conditions:
        query += " WHERE " + " AND ".join(conditions)

    order_by.extend(["value DESC NULLS LAST", "finish_time DESC"])

    if order_by:
        query += " ORDER BY " + ", ".join(order_by) + " "

    if limit is not None:
        query += " LIMIT %d" % limit
    for record in await conn.fetch(query, *args):
        yield tuple(  # type: ignore
            [Run.from_row(record),
             record['value'],
             record['maintainer_email'],
             record['uploader_emails'],
             record['update_changelog'],
             record['policy_command'],
             record['unpublished_branches']
             ]
        )


async def get_never_processed_count(conn: asyncpg.Connection, suites=None):
    query = """\
select suite, count(*) from candidate c
where not exists (
    SELECT FROM run WHERE run.package = c.package AND c.suite = suite)
"""
    args = []
    if suites:
        query += " AND suite = ANY($1::text[])"
        args.append(suites)

    query += " group by suite"

    return await conn.fetch(query, *args)


async def get_publish_policy(
    conn: asyncpg.Connection, package: str, suite: str
) -> Tuple[Optional[Dict[str, Tuple[str, Optional[int]]]], Optional[str], Optional[List[str]]]:
    row = await conn.fetchrow(
        "SELECT publish, update_changelog, command "
        "FROM policy WHERE package = $1 AND suite = $2",
        package,
        suite,
    )
    if row:
        return (  # type: ignore
            {k: (v, f) for k, v, f in row['publish']},
            row['update_changelog'],
            row['command']
        )
    return None, None, None
