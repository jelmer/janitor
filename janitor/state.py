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


async def store_run(
    conn: asyncpg.Connection,
    run_id: str,
    name: str,
    vcs_url: str,
    start_time: datetime.datetime,
    finish_time: datetime.datetime,
    command: List[str],
    description: str,
    instigated_context: Optional[str],
    context: Optional[str],
    main_branch_revision: Optional[bytes],
    result_code: str,
    branch_name: str,
    revision: Optional[bytes],
    subworker_result: Optional[Any],
    suite: str,
    logfilenames: List[str],
    value: Optional[int],
    worker_name: str,
    worker_link: Optional[str],
    result_branches: Optional[List[Tuple[str, str, bytes, bytes]]] = None,
    result_tags: Optional[List[Tuple[str, bytes]]] = None,
    failure_details: Optional[Any] = None
):
    """Store a run.

    Args:
      run_id: Run id
      name: Package name
      vcs_url: Upstream branch URL
      start_time: Start time
      finish_time: Finish time
      command: Command
      description: A human-readable description
      instigated_context: Context that instigated this run
      context: Subworker-specific context
      main_branch_revision: Main branch revision
      result_code: Result code (as constant string)
      branch_name: Resulting branch name
      revision: Resulting revision id
      subworker_result: Subworker-specific result data (as json)
      suite: Suite
      logfilenames: List of log filenames
      value: Value of the run (as int)
      worker_name: Name of the worker
      worker_link: Link to worker URL
      result_branches: Result branches
      result_tags: Result tags
      failure_details: Result failure details
    """
    if result_tags is None:
        result_tags_updated = None
    else:
        result_tags_updated = [(n, r.decode("utf-8")) for (n, r) in result_tags]

    async with conn.transaction():
        await conn.execute(
            "INSERT INTO run (id, command, description, result_code, "
            "start_time, finish_time, package, instigated_context, context, "
            "main_branch_revision, "
            "branch_name, revision, result, suite, branch_url, logfilenames, "
            "value, worker, worker_link, result_tags, "
            "failure_details) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, "
            "$12, $13, $14, $15, $16, $17, $18, $19, $20, $21)",
            run_id,
            " ".join(command),
            description,
            result_code,
            start_time,
            finish_time,
            name,
            instigated_context,
            context,
            main_branch_revision.decode("utf-8") if main_branch_revision else None,
            branch_name,
            revision.decode("utf-8") if revision else None,
            subworker_result if subworker_result else None,
            suite,
            vcs_url,
            logfilenames,
            value,
            worker_name,
            worker_link,
            result_tags_updated,
            failure_details,
        )

        if result_branches:
            await conn.executemany(
                "INSERT INTO new_result_branch "
                "(run_id, role, remote_name, base_revision, revision) "
                "VALUES ($1, $2, $3, $4, $5)",
                [
                    (run_id, role, remote_name, br.decode("utf-8"), r.decode("utf-8"))
                    for (role, remote_name, br, r) in result_branches
                ],
            )


async def store_publish(
    conn: asyncpg.Connection,
    package,
    branch_name,
    main_branch_revision,
    revision,
    role,
    mode,
    result_code,
    description,
    merge_proposal_url=None,
    publish_id=None,
    requestor=None,
):
    if isinstance(revision, bytes):
        revision = revision.decode("utf-8")
    if isinstance(main_branch_revision, bytes):
        main_branch_revision = main_branch_revision.decode("utf-8")
    if merge_proposal_url:
        await conn.execute(
            "INSERT INTO merge_proposal (url, package, status, "
            "revision) VALUES ($1, $2, 'open', $3) ON CONFLICT (url) "
            "DO UPDATE SET package = EXCLUDED.package, "
            "revision = EXCLUDED.revision",
            merge_proposal_url,
            package,
            revision,
        )
    await conn.execute(
        "INSERT INTO publish (package, branch_name, "
        "main_branch_revision, revision, role, mode, result_code, "
        "description, merge_proposal_url, id, requestor) "
        "values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) ",
        package,
        branch_name,
        main_branch_revision,
        revision,
        role,
        mode,
        result_code,
        description,
        merge_proposal_url,
        publish_id,
        requestor,
    )


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
            run_id=row['run_id'],
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
        (
            branch_url,
            subpath,
            package,
            command,
            context,
            queue_id,
            estimated_duration,
            suite,
            refresh,
            requestor,
            vcs_type,
            upstream_branch_url,
        ) = row
        return cls(
            id=queue_id,
            branch_url=branch_url,
            subpath=subpath,
            package=package,
            context=context,
            command=shlex.split(command),
            estimated_duration=estimated_duration,
            suite=suite,
            refresh=refresh,
            requestor=requestor,
            vcs_type=vcs_type,
            upstream_branch_url=upstream_branch_url,
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
    ret = list(await get_queue_positions(conn, suite, [package]))
    if len(ret) == 0:
        return (None, None)
    return ret[0][1], ret[0][2]


async def get_queue_positions(conn: asyncpg.Connection, suite, packages):

    query = (
        "SELECT package, position, wait_time FROM queue_positions "
        "WHERE package = ANY($1::text[]) AND suite = $2"
    )
    return await conn.fetch(query, packages, suite)


async def iter_queue(conn: asyncpg.Connection, limit=None):
    query = """
SELECT
    package.branch_url,
    package.subpath,
    queue.package,
    queue.command,
    queue.context,
    queue.id,
    queue.estimated_duration,
    queue.suite,
    queue.refresh,
    queue.requestor,
    package.vcs_type,
    upstream.upstream_branch_url
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


async def add_to_queue(
    conn: asyncpg.Connection,
    package: str,
    command: List[str],
    suite: str,
    offset: float = 0.0,
    bucket: str = "default",
    context: Optional[str] = None,
    estimated_duration: Optional[datetime.timedelta] = None,
    refresh: bool = False,
    requestor: Optional[str] = None,
) -> bool:
    await conn.execute(
        "INSERT INTO queue "
        "(package, command, priority, bucket, context, "
        "estimated_duration, suite, refresh, requestor) "
        "VALUES "
        "($1, $2, "
        "(SELECT COALESCE(MIN(priority), 0) FROM queue)"
        + " + $3, $4, $5, $6, $7, $8, $9) "
        "ON CONFLICT (package, suite) DO UPDATE SET "
        "context = EXCLUDED.context, priority = EXCLUDED.priority, "
        "bucket = EXCLUDED.bucket, "
        "estimated_duration = EXCLUDED.estimated_duration, "
        "refresh = EXCLUDED.refresh, requestor = EXCLUDED.requestor, "
        "command = EXCLUDED.command "
        "WHERE queue.bucket >= EXCLUDED.bucket OR "
        "(queue.bucket = EXCLUDED.bucket AND "
        "queue.priority >= EXCLUDED.priority)",
        package,
        " ".join(command),
        offset,
        bucket,
        context,
        estimated_duration,
        suite,
        refresh,
        requestor,
    )
    return True


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
        List[str],
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
            [Run.from_row(record[:23])]
            + list(record[23:-3])
            + [
                record[-3],  # type: ignore
                shlex.split(record[-2]) if record[-2] else None,  # type: ignore
                record[-1],
            ]
        )  # type: ignore


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


async def get_never_processed(conn: asyncpg.Connection, suites=None):
    query = """\
select c.package, c.suite from candidate c
where not exists (
    SELECT FROM run WHERE run.package = c.package AND c.suite = suite)
"""
    args = []
    if suites:
        query += " AND suite = ANY($1::text[])"
        args.append(suites)

    return await conn.fetch(query, *args)


async def get_merge_proposal_run(
    conn: asyncpg.Connection, mp_url: str
) -> Tuple[Run, Tuple[str, str, bytes, bytes]]:
    query = """
SELECT
    run.id AS id, run.command AS command,
    run.start_time AS start_time,
    run.finish_time AS finish_time, run.description,
    run.package, debian_build.version, debian_build.distribution, run.result_code,
    run.branch_name, run.main_branch_revision, run.revision, run.context,
    run.result, run.suite, run.instigated_context, run.branch_url,
    run.logfilenames, run.review_status, run.review_comment, run.worker,
    array(SELECT row(role, remote_name, base_revision, revision)
     FROM new_result_branch WHERE run_id = run.id), run.result_tags, rb.role,
     rb.remote_name, rb.base_revision, rb.revision
FROM new_result_branch rb
RIGHT JOIN run ON rb.run_id = run.id
LEFT JOIN debian_build ON run.id = debian_build.run_id
WHERE rb.revision IN (
    SELECT revision from merge_proposal WHERE merge_proposal.url = $1)
ORDER BY run.finish_time ASC
LIMIT 1
"""
    row = await conn.fetchrow(query, mp_url)
    if row:
        return Run.from_row(row[:23]), (
            row[23],
            row[24],
            row[25].encode("utf-8"),
            row[26].encode("utf-8"),
        )
    raise KeyError


async def get_publish(
    conn: asyncpg.Connection, publish_id: str
) -> Optional[Tuple[str, str, bytes, bytes, str, str, str, str]]:
    query = """
SELECT
  package,
  branch_name,
  main_branch_revision,
  revision,
  mode,
  merge_proposal_url,
  result_code,
  description
FROM publish WHERE id = $1
"""
    row = await conn.fetchrow(query, publish_id)
    if row:
        return None
    return (
        row[0],
        row[1],
        row[2].encode("utf-8") if row[2] else None,
        row[3].encode("utf-8") if row[3] else None,
        row[4],
        row[5],
        row[6],
        row[7],
    )


async def set_run_review_status(
    conn: asyncpg.Connection,
    run_id: str,
    review_status: str,
    review_comment: Optional[str] = None,
) -> None:
    await conn.execute(
        "UPDATE run SET review_status = $1, review_comment = $2 WHERE id = $3",
        review_status,
        review_comment,
        run_id,
    )


async def iter_policy(conn: asyncpg.Connection, package: Optional[str] = None):
    query = "SELECT package, suite, publish, update_changelog, command " "FROM policy"
    args = []
    if package:
        query += " WHERE package = $1"
        args.append(package)
    for row in await conn.fetch(query, *args):
        yield (
            row[0],
            row[1],
            (
                {k[0]: (k[1], k[2]) for k in row[2]},
                row[3],
                shlex.split(row[4]) if row[4] else None,
            ),
        )


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
            {k: (v, f) for k, v, f in row[0]},
            row[1],
            shlex.split(row[2]) if row[2] else None,
        )
    return None, None, None


async def store_site_session(
    conn: asyncpg.Connection, session_id: str, user: Any
) -> None:
    await conn.execute(
        """
INSERT INTO site_session (id, userinfo) VALUES ($1, $2)
ON CONFLICT (id) DO UPDATE SET userinfo = EXCLUDED.userinfo""",
        session_id,
        user,
    )


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



