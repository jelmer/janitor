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
import shlex
import asyncpg
from contextlib import asynccontextmanager
from typing import Optional, Tuple, List, Any, Union, Callable, AsyncIterable
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
                        'json',
                        encoder=json.dumps,
                        decoder=json.loads,
                        schema='pg_catalog'
                    )
            await conn.set_type_codec(
                        'jsonb',
                        encoder=json.dumps,
                        decoder=json.loads,
                        schema='pg_catalog'
                    )
            await conn.set_type_codec(
                'debversion', format='text', encoder=str, decoder=Version)
            yield conn


async def popcon(conn: asyncpg.Connection):
    return await conn.fetch(
        "SELECT name, popcon_inst FROM package")


async def store_run(
        conn: asyncpg.Connection,
        run_id: str, name: str, vcs_url: str, start_time: datetime.datetime,
        finish_time: datetime.datetime, command: List[str], description: str,
        instigated_context: Optional[str], context: Optional[str],
        main_branch_revision: Optional[bytes], result_code: str,
        build_version: Optional[Version],
        build_distribution: Optional[str], branch_name: str,
        revision: Optional[bytes], subworker_result: Optional[Any], suite: str,
        logfilenames: List[str], value: Optional[int], worker_name: str):
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
      build_version: Version that was built
      build_distribution: Build distribution
      branch_name: Resulting branch name
      revision: Resulting revision id
      subworker_result: Subworker-specific result data (as json)
      suite: Suite
      logfilenames: List of log filenames
      value: Value of the run (as int)
      worker_name: Name of the worker
    """
    await conn.execute(
        "INSERT INTO run (id, command, description, result_code, "
        "start_time, finish_time, package, instigated_context, context, "
        "build_version, build_distribution, main_branch_revision, "
        "branch_name, revision, result, suite, branch_url, logfilenames, "
        "value, worker) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, "
        "$12, $13, $14, $15, $16, $17, $18, $19, $20)",
        run_id, ' '.join(command), description, result_code,
        start_time, finish_time, name, instigated_context, context,
        str(build_version) if build_version else None, build_distribution,
        main_branch_revision.decode('utf-8') if main_branch_revision else None,
        branch_name,
        revision.decode('utf-8') if revision else None,
        subworker_result if subworker_result else None, suite,
        vcs_url, logfilenames, value, worker_name)


async def store_publish(conn: asyncpg.Connection,
                        package, branch_name, main_branch_revision,
                        revision, mode, result_code, description,
                        merge_proposal_url=None, publish_id=None,
                        requestor=None):
    if isinstance(revision, bytes):
        revision = revision.decode('utf-8')
    if isinstance(main_branch_revision, bytes):
        main_branch_revision = main_branch_revision.decode('utf-8')
    if merge_proposal_url:
        await conn.execute(
            "INSERT INTO merge_proposal (url, package, status, "
            "revision) VALUES ($1, $2, 'open', $3) ON CONFLICT (url) "
            "DO UPDATE SET package = EXCLUDED.package, "
            "revision = EXCLUDED.revision",
            merge_proposal_url, package, revision)
    await conn.execute(
        "INSERT INTO publish (package, branch_name, "
        "main_branch_revision, revision, mode, result_code, description, "
        "merge_proposal_url, id, requestor) "
        "values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) ",
        package, branch_name, main_branch_revision, revision, mode,
        result_code, description, merge_proposal_url, publish_id,
        requestor)


class Package(object):

    name: str
    maintainer_email: str
    uploader_emails: List[str]
    branch_url: str
    subpath: Optional[str]
    archive_version: Optional[Version]
    vcs_type: Optional[str]
    vcs_url: Optional[str]
    vcs_browse: Optional[str]
    popcon_inst: Optional[int]
    removed: bool
    vcswatch_status: str
    vcswatch_version: str
    upstream_branch_url: Optional[str]

    def __init__(self, name, maintainer_email, uploader_emails, branch_url,
                 vcs_type, vcs_url, vcs_browse, removed, vcswatch_status,
                 vcswatch_version):
        self.name = name
        self.maintainer_email = maintainer_email
        self.uploader_emails = uploader_emails
        self.branch_url = branch_url
        self.vcs_type = vcs_type
        self.vcs_url = vcs_url
        self.vcs_browse = vcs_browse
        self.removed = removed
        self.vcswatch_status = vcswatch_status
        self.vcswatch_version = vcswatch_version

    @classmethod
    def from_row(cls, row) -> 'Package':
        return cls(row[0], row[1], row[2], row[3], row[4], row[5], row[6],
                   row[7], row[8], row[9])

    def __lt__(self, other) -> bool:
        if not isinstance(other, type(self)):
            raise TypeError(other)
        return self.__tuple__ < other.__tuple__()

    def __tuple__(self):
        return (self.name, self.maintainer_email, self.uploader_emails,
                self.branch_url, self.vcs_type, self.vcs_url, self.vcs_browse,
                self.removed, self.vcswatch_status, self.vcswatch_version)


async def iter_packages(conn: asyncpg.Connection, package=None):
    query = """
SELECT
  name,
  maintainer_email,
  uploader_emails,
  branch_url,
  vcs_type,
  vcs_url,
  vcs_browse,
  removed,
  vcswatch_status,
  vcswatch_version
FROM
  package
"""
    args = []
    if package:
        query += " WHERE name = $1"
        args.append(package)
    query += " ORDER BY name ASC"
    return [
        Package.from_row(row) for row in await conn.fetch(query, *args)]


async def get_package(conn: asyncpg.Connection, name):
    try:
        return list(await iter_packages(conn, package=name))[0]
    except IndexError:
        return None


async def get_package_by_branch_url(
        conn: asyncpg.Connection, branch_url: str) -> Optional[Package]:
    query = """
SELECT
  name,
  maintainer_email,
  uploader_emails,
  branch_url,
  vcs_type,
  vcs_url,
  vcs_browse,
  removed,
  vcswatch_status,
  vcswatch_version
FROM
  package
WHERE
  branch_url = $1 OR branch_url = $2
"""
    branch_url2 = urlutils.split_segment_parameters(branch_url)[0]
    row = await conn.fetchrow(query, branch_url, branch_url2)
    if row is None:
        return None
    return Package.from_row(row)


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

    __slots__ = [
            'id', 'times', 'command', 'description', 'package',
            'build_version',
            'build_distribution', 'result_code', 'branch_name',
            'main_branch_revision', 'revision', 'context', 'result',
            'suite', 'instigated_context', 'branch_url', 'logfilenames',
            'review_status', 'review_comment', 'worker_name']

    def __init__(self, run_id, times, command, description, package,
                 build_version,
                 build_distribution, result_code, branch_name,
                 main_branch_revision, revision, context, result,
                 suite, instigated_context, branch_url, logfilenames,
                 review_status, review_comment, worker_name):
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

    @property
    def duration(self) -> datetime.timedelta:
        return self.times[1] - self.times[0]

    def age(self):
        return datetime.datetime.now() - self.times[1]

    @classmethod
    def from_row(cls, row) -> 'Run':
        return cls(run_id=row[0],
                   times=(row[2], row[3]),
                   command=row[1], description=row[4], package=row[5],
                   build_version=Version(row[6]) if row[6] else None,
                   build_distribution=row[7],
                   result_code=(row[8] if row[8] else None),
                   branch_name=row[9],
                   main_branch_revision=(
                       row[10].encode('utf-8') if row[10] else None),
                   revision=(row[11].encode('utf-8') if row[11] else None),
                   context=row[12], result=row[13], suite=row[14],
                   instigated_context=row[15], branch_url=row[16],
                   logfilenames=row[17], review_status=row[18],
                   review_comment=row[19], worker_name=row[20])

    def __len__(self) -> int:
        return len(self.__slots__)

    def __tuple__(self):
        return (self.id, self.times, self.command, self.description,
                self.package, self.build_version, self.build_distribution,
                self.result_code, self.branch_name, self.main_branch_revision,
                self.revision, self.context, self.result, self.suite,
                self.instigated_context, self.branch_url,
                self.logfilenames, self.review_status,
                self.review_comment, self.worker_name)

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


async def get_unchanged_run(conn: asyncpg.Connection, main_branch_revision):
    query = """
SELECT
    id, command, start_time, finish_time, description, package,
    build_version, build_distribution, result_code,
    branch_name, main_branch_revision, revision, context, result, suite,
    instigated_context, branch_url, logfilenames, review_status,
    review_comment, worker
FROM
    last_runs
WHERE
    suite = 'unchanged' AND revision = $1 AND
    build_version IS NOT NULL
ORDER BY finish_time DESC
"""
    if isinstance(main_branch_revision, bytes):
        main_branch_revision = main_branch_revision.decode('utf-8')
    row = await conn.fetchrow(query, main_branch_revision)
    if row is not None:
        return Run.from_row(row)
    return None


async def iter_runs(conn: asyncpg.Connection,
                    package: Optional[str] = None,
                    run_id: Optional[str] = None,
                    worker: Optional[str] = None,
                    limit: Optional[int] = None):
    """Iterate over runs.

    Args:
      package: package to restrict to
    Returns:
      iterator over Run objects
    """
    query = """
SELECT
    id, command, start_time, finish_time, description, package,
    build_version, build_distribution, result_code,
    branch_name, main_branch_revision, revision, context, result, suite,
    instigated_context, branch_url, logfilenames, review_status,
    review_comment, worker
FROM
    run
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
    if conditions:
        query += " WHERE " + " AND ".join(conditions)
    query += "ORDER BY start_time DESC"
    if limit:
        query += " LIMIT %d" % limit
    for row in await conn.fetch(query, *args):
        yield Run.from_row(row)


async def get_run(conn: asyncpg.Connection, run_id, package=None):
    async for run in iter_runs(conn, run_id=run_id, package=package):
        return run
    else:
        return None


async def iter_proposals(conn: asyncpg.Connection, package=None, suite=None):
    args = []
    query = """
SELECT
    DISTINCT ON (merge_proposal.url)
    merge_proposal.package, merge_proposal.url, merge_proposal.status
FROM
    merge_proposal
LEFT JOIN run ON merge_proposal.revision = run.revision
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
    return await conn.fetch(query, *args)


async def iter_proposals_with_run(
        conn: asyncpg.Connection,
        package: Optional[str] = None,
        suite: Optional[str] = None):
    args = []
    query = """
SELECT
    DISTINCT ON (merge_proposal.url)
    run.id,
    run.command,
    run.start_time,
    run.finish_time,
    run.description,
    run.package,
    run.build_version,
    run.build_distribution,
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
    merge_proposal.url, merge_proposal.status
FROM
    merge_proposal
LEFT JOIN run ON merge_proposal.revision = run.revision
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
        yield Run.from_row(row[:21]), row[21], row[22]


class QueueItem(object):

    __slots__ = ['id', 'branch_url', 'subpath', 'package', 'context',
                 'command', 'estimated_duration', 'suite', 'refresh',
                 'requestor', 'vcs_type', 'upstream_branch_url']

    def __init__(self, id, branch_url, subpath, package, context, command,
                 estimated_duration, suite, refresh, requestor, vcs_type,
                 upstream_branch_url):
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
    def from_row(cls, row) -> 'QueueItem':
        (branch_url, subpath, package,
            command, context, queue_id, estimated_duration,
            suite, refresh, requestor, vcs_type,
            upstream_branch_url) = row
        return cls(
                id=queue_id, branch_url=branch_url,
                subpath=subpath, package=package, context=context,
                command=shlex.split(command),
                estimated_duration=estimated_duration,
                suite=suite, refresh=refresh, requestor=requestor,
                vcs_type=vcs_type, upstream_branch_url=upstream_branch_url)

    def _tuple(self):
        return (self.id, self.branch_url, self.subpath, self.package,
                self.context, self.command, self.estimated_duration,
                self.suite, self.refresh, self.requestor, self.vcs_type,
                self.upstream_branch_url)

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
    subquery = """
SELECT
    package,
    suite,
    row_number() OVER (ORDER BY priority ASC, id ASC) AS position,
    SUM(estimated_duration) OVER (ORDER BY priority ASC, id ASC)
        - coalesce(estimated_duration, interval '0') AS wait_time
FROM
    queue
ORDER BY priority ASC, id ASC
"""
    query = (
        "SELECT package, position, wait_time FROM (" + subquery + ") AS q "
        "WHERE package = ANY($1::text[]) AND suite = $2")
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
queue.priority ASC,
queue.id ASC
"""
    if limit:
        query += " LIMIT %d" % limit
    for row in await conn.fetch(query):
        yield QueueItem.from_row(row)


async def drop_queue_item(conn: asyncpg.Connection, queue_id):
    await conn.execute("DELETE FROM queue WHERE id = $1", queue_id)


async def add_to_queue(conn: asyncpg.Connection,
                       package: str,
                       command: List[str],
                       suite: str,
                       offset: int = 0,
                       context: Optional[str] = None,
                       estimated_duration: Optional[datetime.timedelta] = None,
                       refresh: bool = False,
                       requestor: Optional[str] = None,
                       requestor_relative: bool = False) -> bool:
    await conn.execute(
        "INSERT INTO queue "
        "(package, command, priority, context, "
        "estimated_duration, suite, refresh, requestor) "
        "VALUES "
        "($1, $2, "
        "(SELECT COALESCE(MIN(priority), 0) FROM queue " +
        ("WHERE requestor = $8" if requestor_relative else "") +
        ") + $3, $4, $5, $6, $7, $8) "
        "ON CONFLICT (package, suite) DO UPDATE SET "
        "context = EXCLUDED.context, priority = EXCLUDED.priority, "
        "estimated_duration = EXCLUDED.estimated_duration, "
        "refresh = EXCLUDED.refresh, requestor = EXCLUDED.requestor, "
        "command = EXCLUDED.command "
        "WHERE queue.priority >= EXCLUDED.priority",
        package, ' '.join(command), offset, context, estimated_duration,
        suite, refresh, requestor)
    return True


async def set_proposal_info(
        conn: asyncpg.Connection, url: str, status: str, revision: Optional[bytes],
        package: Optional[str], merged_by: Optional[str],
        merged_at: Optional[str]) -> None:
    await conn.execute("""
INSERT INTO merge_proposal (
    url, status, revision, package, merged_by, merged_at)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (url)
DO UPDATE SET
  status = EXCLUDED.status,
  revision = EXCLUDED.revision,
  package = EXCLUDED.package,
  merged_by = EXCLUDED.merged_by,
  merged_at = EXCLUDED.merged_at
""", url, status, (revision.decode('utf-8') if revision is not None else None), package, merged_by, merged_at)


async def queue_length(conn: asyncpg.Connection, minimum_priority=None):
    args = []
    query = 'SELECT COUNT(*) FROM queue'
    if minimum_priority is not None:
        query += ' WHERE priority >= $1'
        args.append(minimum_priority)
    return await conn.fetchval(query, *args)


async def current_tick(conn: asyncpg.Connection):
    ret = await conn.fetchval('SELECT MIN(priority) FROM queue')
    if ret is None:
        ret = 0
    return ret


async def queue_duration(conn: asyncpg.Connection, minimum_priority=None):
    args = []
    query = """
SELECT
  SUM(estimated_duration)
FROM
  queue
WHERE
  estimated_duration IS NOT NULL
"""
    if minimum_priority is not None:
        query += ' AND priority >= $1'
        args.append(minimum_priority)
    ret = (await conn.fetchrow(query, *args))[0]
    if ret is None:
        return datetime.timedelta()
    return ret


async def iter_published_packages(conn: asyncpg.Connection, suite):
    return await conn.fetch("""
select distinct on (package.name) package.name, build_version, archive_version
from run left join package on package.name = run.package
where run.build_distribution = $1 and not package.removed
order by package.name, build_version desc
""", suite)


async def get_published_by_suite(conn: asyncpg.Connection):
    return await conn.fetch("""
select suite, count(distinct package) from run where build_version is not null
group by 1
""")


async def iter_previous_runs(
        conn: asyncpg.Connection,
        package: str, suite: str) -> AsyncIterable[Run]:
    for row in await conn.fetch("""
SELECT
  id,
  command,
  start_time,
  finish_time,
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
  worker
FROM
  run
WHERE
  package = $1 AND suite = $2
ORDER BY start_time DESC
""", package, suite):
        yield Run.from_row(row)


async def get_last_unabsorbed_run(
        conn: asyncpg.Connection,
        package: str, suite: str) -> Optional[Run]:
    args = []
    query = """
SELECT
  id,
  command,
  start_time,
  finish_time,
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
  worker
FROM
  last_unabsorbed_runs
WHERE package = $1 AND suite = $2
ORDER BY package, command DESC, start_time DESC
LIMIT 1
"""
    args = [package, suite]
    row = await conn.fetchrow(query, *args)
    if row is None:
        return None
    return Run.from_row(row)


async def get_last_effective_run(
        conn: asyncpg.Connection,
        package: str, suite: str) -> Optional[Run]:
    args = []
    query = """
SELECT
  id,
  command,
  start_time,
  finish_time,
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
  worker
FROM
  last_effective_runs
WHERE package = $1 AND suite = $2
ORDER BY package, command DESC, start_time DESC
LIMIT 1
"""
    args = [package, suite]
    row = await conn.fetchrow(query, *args)
    if row is None:
        return None
    return Run.from_row(row)


async def iter_last_unabsorbed_runs(
        conn: asyncpg.Connection, suite=None, packages=None):
    query = """
SELECT DISTINCT ON (package)
  id,
  command,
  start_time,
  finish_time,
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
  worker
FROM
  last_unabsorbed_runs
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
ORDER BY package, command, start_time DESC
"""
    for row in await conn.fetch(query, *args):
        yield Run.from_row(row)


async def stats_by_result_codes(conn: asyncpg.Connection, suite=None):
    query = """\
select (
    case when result_code = 'nothing-new-to-do' then 'success'
    else result_code end), count(result_code) from last_runs
"""
    args = []
    if suite:
        args.append(suite)
        query += " WHERE suite = $1"
    query += " group by 1 order by 2 desc"
    return await conn.fetch(query, *args)


async def iter_last_runs(
        conn: asyncpg.Connection,
        result_code: Optional[str] = None,
        suite: Optional[str] = None,
        main_branch_revision: Optional[bytes] = None
        ) -> AsyncIterable[Run]:
    query = """
SELECT
  id,
  command,
  start_time,
  finish_time,
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
  worker
FROM last_runs
"""
    where = []
    args: List[Any] = []
    if result_code is not None:
        args.append(result_code)
        where.append('result_code = $%d' % len(args))
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


async def update_run_result(
        conn: asyncpg.Connection, log_id: str, code: str, description: str
        ) -> None:
    await conn.execute(
        'UPDATE run SET result_code = $1, description = $2 WHERE id = $3',
        code, description, log_id)


async def already_published(
        conn: asyncpg.Connection,
        package: str, branch_name: str, revision: bytes, mode: str) -> bool:
    row = await conn.fetchrow("""\
SELECT * FROM publish
WHERE mode = $1 AND revision = $2 AND package = $3 AND branch_name = $4
""", mode, revision.decode('utf-8'), package, branch_name)
    if row:
        return True
    return False


async def iter_publish_ready(
        conn: asyncpg.Connection,
        suites: Optional[List[str]] = None,
        review_status: Optional[Union[str, List[str]]] = None,
        limit: Optional[int] = None,
        publishable_only: bool = False
        ) -> AsyncIterable[
            Tuple[Run, str, List[str], str, str, str, List[str]]]:
    args: List[Any] = []
    query = """
SELECT
  run.id,
  run.command,
  run.start_time,
  run.finish_time,
  run.description,
  run.package,
  run.build_version,
  run.build_distribution,
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
  package.maintainer_email,
  package.uploader_emails,
  run.branch_url,
  publish_policy.mode,
  publish_policy.update_changelog,
  publish_policy.command
FROM
  last_unabsorbed_runs AS run
LEFT JOIN package ON package.name = run.package
LEFT JOIN publish_policy ON
    publish_policy.package = run.package AND publish_policy.suite = run.suite
WHERE result_code = 'success' AND result IS NOT NULL
"""
    if suites is not None:
        query += " AND run.suite = ANY($1::text[]) "
        args.append(suites)
    if review_status is not None:
        if not isinstance(review_status, list):
            review_status = [review_status]
        args.append(review_status)
        query += " AND review_status = ANY($%d::review_status[]) " % (
            len(args),)

    if publishable_only:
        query += """ AND publish_policy.mode in (
        'propose', 'attempt-push', 'push-derived', 'push') """

    query += """
ORDER BY
  publish_policy.mode in (
        'propose', 'attempt-push', 'push-derived', 'push') DESC,
  value DESC NULLS LAST,
  run.finish_time DESC
"""
    if limit is not None:
        query += " LIMIT %d" % limit
    for record in await conn.fetch(query, *args):
        yield tuple(
            [Run.from_row(record[:21])] + list(record[21:-1]) +
            [shlex.split(record[-1]) if record[-1] else None])  # type: ignore


async def iter_unscanned_branches(
        conn: asyncpg.Connection, last_scanned_minimum: datetime.datetime
        ) -> AsyncIterable[Tuple[str, str, str, datetime.datetime]]:
    return await conn.fetch("""
SELECT
  name,
  'master',
  branch_url,
  last_scanned
FROM package
LEFT JOIN branch ON package.branch_url = branch.url
WHERE
  last_scanned is null or now() - last_scanned > $1
""", last_scanned_minimum)


async def iter_package_branches(conn: asyncpg.Connection):
    return await conn.fetch("""
SELECT
  name,
  branch_url,
  revision,
  last_scanned,
  description
FROM
  package
LEFT JOIN branch ON package.branch_url = branch.url
""")


async def update_branch_status(
        conn: asyncpg.Connection,
        branch_url: str, canonical_branch_url: Optional[str],
        last_scanned: Union[
            datetime.datetime, Callable[[], datetime.datetime]
            ] = datetime.datetime.now,
        status: Optional[str] = None,
        revision: Optional[bytes] = None, description: Optional[str] = None):
    if callable(last_scanned):
        last_scanned = last_scanned()
    await conn.execute(
        "INSERT INTO branch (url, canonical_url, status, revision, "
        "last_scanned, description) VALUES ($1, $2, $3, $4, $5, $6) "
        "ON CONFLICT (url) DO UPDATE SET "
        "status = EXCLUDED.status, revision = EXCLUDED.revision, "
        "last_scanned = EXCLUDED.last_scanned, "
        "description = EXCLUDED.description, "
        "canonical_url = EXCLUDED.canonical_url",
        branch_url, canonical_branch_url,
        status, revision.decode('utf-8') if revision else None,
        last_scanned, description)


async def get_run_result_by_revision(
        conn: asyncpg.Connection, suite: str, revision: bytes
        ) -> Tuple[Optional[str], Optional[str], Optional[str]]:
    row = await conn.fetchrow(
        "SELECT result, branch_name, review_status FROM run "
        "WHERE suite = $1 AND revision = $2 AND result_code = 'success'",
        suite, revision.decode('utf-8'))
    if row is not None:
        return row[0], row[1], row[2]
    return None, None, None


async def get_last_build_version(
        conn: asyncpg.Connection,
        package: str,
        suite: str) -> Optional[str]:
    return await conn.fetchval(
        "SELECT build_version FROM run WHERE "
        "build_version IS NOT NULL AND package = $1 AND "
        "build_distribution = $2 ORDER BY build_version DESC",
        package, suite)


async def estimate_duration(
        conn: asyncpg.Connection, package: Optional[str] = None,
        suite: Optional[str] = None) -> Optional[datetime.timedelta]:
    query = """
SELECT AVG(finish_time - start_time) FROM run
WHERE """
    args = []
    if package is not None:
        query += " package = $1"
        args.append(package)
    if suite is not None:
        if package:
            query += " AND"
        query += " suite = $%d" % (len(args) + 1)
        args.append(suite)
    return await conn.fetchval(query, *args)


async def store_candidates(conn: asyncpg.Connection, entries):
    await conn.executemany(
        "INSERT INTO candidate "
        "(package, suite, context, value, success_chance) "
        "VALUES ($1, $2, $3, $4, $5) ON CONFLICT (package, suite) "
        "DO UPDATE SET context = EXCLUDED.context, value = EXCLUDED.value, "
        "success_chance = EXCLUDED.success_chance",
        entries)


async def iter_candidates(
        conn: asyncpg.Connection, packages: Optional[List[str]] = None,
        suite: Optional[str] = None
        ) -> List[Tuple[
            Package, str, Optional[str], Optional[int], Optional[float]]]:
    query = """
SELECT
  package.name,
  package.maintainer_email,
  package.uploader_emails,
  package.branch_url,
  package.vcs_type,
  package.vcs_url,
  package.vcs_browse,
  package.removed,
  package.vcswatch_status,
  package.vcswatch_version,
  candidate.suite,
  candidate.context,
  candidate.value,
  candidate.success_chance
FROM candidate
INNER JOIN package on package.name = candidate.package
WHERE NOT package.removed
"""
    args = []
    if suite is not None and packages is not None:
        query += " AND package.name = ANY($1::text[]) AND suite = $2"
        args.extend([packages, suite])
    elif suite is not None:
        query += " AND suite = $1"
        args.append(suite)
    elif packages is not None:
        query += " AND package.name = ANY($1::text[])"
        args.append(packages)
    return [tuple([Package.from_row(row)] + list(row[10:]))  # type: ignore
            for row in await conn.fetch(query, *args)]


async def iter_candidates_with_policy(
        conn: asyncpg.Connection,
        packages: Optional[List[str]] = None,
        suite: Optional[str] = None
        ) -> List[Tuple[
            Package, str, Optional[str], Optional[int], Optional[float], str,
            str, List[str]]]:
    query = """
SELECT
  package.name,
  package.maintainer_email,
  package.uploader_emails,
  package.branch_url,
  package.vcs_type,
  package.vcs_url,
  package.vcs_browse,
  package.removed,
  package.vcswatch_status,
  package.vcswatch_version,
  candidate.suite,
  candidate.context,
  candidate.value,
  candidate.success_chance,
  publish_policy.mode,
  publish_policy.update_changelog,
  publish_policy.command
FROM candidate
INNER JOIN package on package.name = candidate.package
LEFT JOIN publish_policy ON
    publish_policy.package = package.name AND
    publish_policy.suite = candidate.suite
WHERE NOT package.removed
"""
    args = []
    if suite is not None and packages is not None:
        query += " AND package.name = ANY($1::text[]) AND candidate.suite = $2"
        args.extend([packages, suite])
    elif suite is not None:
        query += " AND candidate.suite = $1"
        args.append(suite)
    elif packages is not None:
        query += " AND package.name = ANY($1::text[])"
        args.append(packages)
    return [(Package.from_row(row), row[10], row[11], row[12], row[13],
             (row[14], row[15],
              shlex.split(row[16]) if row[16] is not None else None)
             )   # type: ignore
            for row in await conn.fetch(query, *args)]


async def get_candidate(conn: asyncpg.Connection, package, suite):
    return await conn.fetchrow(
        "SELECT context, value, success_chance FROM candidate "
        "WHERE package = $1 AND suite = $2", package, suite)


async def iter_sources_with_archive_version(
        conn: asyncpg.Connection, packages: List[str]
        ) -> List[Tuple[str, Version]]:
    return await conn.fetch(
        "SELECT name, archive_version FROM package "
        "WHERE name = any($1::text[])", packages)


async def iter_packages_by_maintainer(conn: asyncpg.Connection, maintainer):
    return [(row[0], row[1]) for row in await conn.fetch(
        "SELECT name, removed FROM package WHERE "
        "maintainer_email = $1 OR $1 = any(uploader_emails)",
        maintainer)]


async def get_never_processed(conn: asyncpg.Connection, suites=None):
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


async def iter_by_suite_result_code(conn: asyncpg.Connection):
    query = """
SELECT DISTINCT ON (package, suite)
  package,
  suite,
  finish_time - start_time AS duration,
  result_code
FROM
  run
ORDER BY package, suite, start_time DESC
"""
    async with conn.transaction():
        async for record in conn.cursor(query):
            yield record


async def get_merge_proposal_run(
        conn: asyncpg.Connection, mp_url: str) -> Optional[Run]:
    query = """
SELECT
    run.id, run.command, run.start_time, run.finish_time, run.description,
    run.package, run.build_version, run.build_distribution, run.result_code,
    run.branch_name, run.main_branch_revision, run.revision, run.context,
    run.result, run.suite, run.instigated_context, run.branch_url,
    run.logfilenames, run.review_status, run.review_comment, run.worker
FROM run inner join merge_proposal on merge_proposal.revision = run.revision
WHERE merge_proposal.url = $1
ORDER BY run.finish_time ASC
LIMIT 1
"""
    row = await conn.fetchrow(query, mp_url)
    if row:
        return Run.from_row(row)
    return None


async def get_proposal_info(conn: asyncpg.Connection, url) -> Tuple[Optional[bytes], str, str, str]:
    row = await conn.fetchrow("""\
SELECT
    package.maintainer_email,
    merge_proposal.revision,
    merge_proposal.status,
    package.name
FROM
    merge_proposal
LEFT JOIN package ON merge_proposal.package = package.name
WHERE
    merge_proposal.url = $1
""", url)
    if not row:
        raise KeyError
    return (row[1].encode('utf-8') if row[1] else None, row[2], row[3], row[0])


async def get_open_merge_proposal(
        conn: asyncpg.Connection, package: str, branch_name: str) -> bytes:
    query = """\
SELECT
    merge_proposal.revision
FROM
    merge_proposal
INNER JOIN publish ON merge_proposal.url = publish.merge_proposal_url
WHERE
    merge_proposal.status = 'open' AND
    merge_proposal.package = $1 AND
    publish.branch_name = $2
ORDER BY timestamp DESC
"""
    return await conn.fetchrow(query, package, branch_name)


async def get_publish(
        conn: asyncpg.Connection, publish_id: str) -> Optional[
            Tuple[str, str, bytes, bytes, str, str, str, str]]:
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
    return (row[0], row[1], row[2].encode('utf-8') if row[2] else None,
            row[3].encode('utf-8') if row[3] else None, row[4],
            row[5], row[6], row[7])


async def update_removals(
        conn: asyncpg.Connection, distribution: str,
        items: List[Tuple[str, Version]]) -> None:
    if not items:
        return
    query = """\
UPDATE package SET removed = True WHERE name = $1 AND distribution = $2 AND archive_version <= $3
"""
    await conn.executemany(
        query, [(name, distribution, archive_version)
                for (name, archive_version) in items])


async def version_available(
        conn: asyncpg.Connection, package: str, suite: str,
        version: Optional[Version] = None) -> List[Tuple[str, str, Version]]:
    query = """\
SELECT
  package,
  suite,
  build_version
FROM
  run
WHERE
  package = $1 AND (suite = $2 OR suite = 'unchanged')
  AND %(version_match1)s

UNION

SELECT
  name,
  'unchanged',
  archive_version
FROM
  package
WHERE name = $1 AND %(version_match2)s
"""
    args = [package, suite]
    if version:
        query = query % {
            'version_match1': "build_version %s $3" % (version[0], ),
            'version_match2': "archive_version %s $3" % (version[0], )}
        args.append(version[1])
    else:
        query = query % {
            'version_match1': 'True',
            'version_match2': 'True'}
    return await conn.fetch(query, *args)


async def set_run_review_status(
        conn: asyncpg.Connection, run_id: str,
        review_status: str, review_comment: Optional[str] = None) -> None:
    await conn.execute(
        'UPDATE run SET review_status = $1, review_comment = $2 WHERE id = $3',
        review_status, review_comment, run_id)


async def iter_vcs_regressions(conn: asyncpg.Connection):
    query = """\
select
  package.name,
  run.suite,
  run.id,
  run.result_code,
  package.vcswatch_status
from
  last_runs run left join package on run.package = package.name
where
  result_code in (
    'branch-missing',
    'branch-unavailable',
    '401-unauthorized',
    'hosted-on-alioth'
  )
and
  vcswatch_status in ('old', 'new', 'commits', 'ok')
"""
    return await conn.fetch(query)


async def iter_review_status(conn: asyncpg.Connection):
    query = """\
select
  review_status,
  count(review_status)
from
  last_runs
where result_code = 'success'
group by 1
"""
    return await conn.fetch(query)


async def iter_upstream_branch_urls(conn: asyncpg.Connection):
    query = """
select
  name,
  upstream_branch_url
from upstream
where upstream_branch_url is not null
"""
    return await conn.fetch(query)


async def update_branch_url(
        conn: asyncpg.Connection, package: str, vcs_type: str,
        vcs_url: str) -> None:
    await conn.execute(
        'update package set vcs_type = $1, branch_url = $2 '
        'where name = $3', vcs_type, vcs_url, package)


async def update_publish_policy(
        conn: asyncpg.Connection, name: str, suite: str, publish_mode: str,
        changelog_mode: str, command: List[str]) -> None:
    await conn.execute(
        'INSERT INTO publish_policy '
        '(package, suite, mode, update_changelog, command) '
        'VALUES ($1, $2, $3, $4, $5) '
        'ON CONFLICT (package, suite) DO UPDATE SET '
        'mode = EXCLUDED.mode, '
        'update_changelog = EXCLUDED.update_changelog, '
        'command = EXCLUDED.command',
        name, suite, publish_mode, changelog_mode,
        (' '.join(command) if command else None))


async def iter_publish_policy(conn: asyncpg.Connection, package: Optional[str] = None):
    query = (
        'SELECT package, suite, mode, update_changelog, command '
        'FROM publish_policy')
    args = []
    if package:
        query += ' WHERE package = $1'
        args.append(package)
    for row in await conn.fetch(query, *args):
        yield (row[0], row[1], (row[2], row[3],
               shlex.split(row[4]) if row[4] else None))


async def get_publish_policy(
        conn: asyncpg.Connection, package: str, suite: str
        ) -> Tuple[Optional[str], Optional[str], Optional[List[str]]]:
    row = await conn.fetchrow(
        'SELECT mode, update_changelog, command '
        'FROM publish_policy WHERE package = $1 AND suite = $2', package,
        suite)
    if row:
        return (  # type: ignore
            row[0], row[1],
            shlex.split(row[2]) if row[2] else None)
    return None, None, None


async def get_successful_push_count(
        conn: asyncpg.Connection) -> Optional[int]:
    return await conn.fetchval(
        "select count(*) from publish where result_code = "
        "'success' and mode = 'push'")


async def get_publish_attempt_count(
        conn: asyncpg.Connection, revision: bytes) -> int:
    return await conn.fetchval(
        "select count(*) from publish where revision = $1",
        revision.decode('utf-8'))


async def check_worker_credentials(
        conn: asyncpg.Connection, login: str, password: str) -> bool:
    row = await conn.fetchrow(
        "select 1 from worker where name = $1 "
        "AND password = crypt($2, password)", login, password)
    return bool(row)


async def package_exists(conn, package):
    return bool(await conn.fetchrow(
        "SELECT 1 FROM package WHERE name = $1", package))


async def guess_package_from_revision(
        conn: asyncpg.Connection, revision: bytes
        ) -> Tuple[Optional[str], Optional[str]]:
    query = """\
select distinct package, maintainer_email from run
left join package on package.name = run.package
where revision = $1 and run.package is not null
"""
    rows = await conn.fetch(query, revision.decode('utf-8'))
    if len(rows) == 1:
        return rows[0][0], rows[0][1]
    return None, None


async def get_publish_history(
        conn: asyncpg.Connection, revision: bytes) -> Tuple[
                str, Optional[str], str, str, str, datetime.datetime]:
    return await conn.fetch(
        "select mode, merge_proposal_url, description, result_code, "
        "requestor, timestamp from publish where revision = $1 "
        "ORDER BY timestamp DESC",
        revision.decode('utf-8'))
