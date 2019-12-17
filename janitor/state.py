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
                'debversion', format='text', encoder=str, decoder=Version)
            yield conn


async def store_packages(conn, packages):
    """Store packages in the database.

    Args:
      packages: list of tuples with (
        name, branch_url, subpath, maintainer_email, uploader_emails,
        unstable_version, vcs_type, vcs_url, vcs_browse, vcswatch_status,
        vcswatch_version, popcon_inst, removed)
    """
    await conn.executemany(
        "INSERT INTO package "
        "(name, branch_url, subpath, maintainer_email, uploader_emails, "
        "unstable_version, vcs_type, vcs_url, vcs_browse, vcswatch_status, "
        "vcswatch_version, popcon_inst, removed) "
        "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) "
        "ON CONFLICT (name) DO UPDATE SET "
        "branch_url = EXCLUDED.branch_url, "
        "subpath = EXCLUDED.subpath, "
        "maintainer_email = EXCLUDED.maintainer_email, "
        "uploader_emails = EXCLUDED.uploader_emails, "
        "unstable_version = EXCLUDED.unstable_version, "
        "vcs_type = EXCLUDED.vcs_type, "
        "vcs_url = EXCLUDED.vcs_url, "
        "vcs_browse = EXCLUDED.vcs_browse, "
        "vcswatch_status = EXCLUDED.vcswatch_status, "
        "vcswatch_version = EXCLUDED.vcswatch_version, "
        "popcon_inst = EXCLUDED.popcon_inst, "
        "removed = EXCLUDED.removed",
        packages)


async def popcon(conn):
    return await conn.fetch(
        "SELECT name, popcon_inst FROM package")


async def store_run(
        conn, run_id, name, vcs_url, start_time, finish_time,
        command, description, instigated_context, context,
        main_branch_revision, result_code, build_version,
        build_distribution, branch_name, revision, subworker_result, suite,
        logfilenames, value):
    """Store a run.

    :param run_id: Run id
    :param name: Package name
    :param vcs_url: Upstream branch URL
    :param start_time: Start time
    :param finish_time: Finish time
    :param command: Command
    :param description: A human-readable description
    :param instigated_context: Context that instigated this run
    :param context: Subworker-specific context
    :param main_branch_revision: Main branch revision
    :param result_code: Result code (as constant string)
    :param build_version: Version that was built
    :param build_distribution: Build distribution
    :param branch_name: Resulting branch name
    :param revision: Resulting revision id
    :param subworker_result: Subworker-specific result data (as json)
    :param suite: Suite
    :param logfilenames: List of log filenames
    :param value: Value of the run (as int)
    """
    await conn.execute(
        "INSERT INTO run (id, command, description, result_code, "
        "start_time, finish_time, package, instigated_context, context, "
        "build_version, build_distribution, main_branch_revision, "
        "branch_name, revision, result, suite, branch_url, logfilenames, "
        "value) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, "
        "$13, $14, $15, $16, $17, $18, $19)",
        run_id, ' '.join(command), description, result_code,
        start_time, finish_time, name, instigated_context, context,
        str(build_version) if build_version else None, build_distribution,
        main_branch_revision, branch_name, revision,
        subworker_result if subworker_result else None, suite,
        vcs_url, logfilenames, value)


async def store_publish(conn, package, branch_name, main_branch_revision,
                        revision, mode, result_code, description,
                        merge_proposal_url=None, publish_id=None):
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
        "merge_proposal_url, id) "
        "values ($1, $2, $3, $4, $5, $6, $7, $8, $9) ",
        package, branch_name, main_branch_revision, revision, mode,
        result_code, description, merge_proposal_url, publish_id)


class Package(object):

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
    def from_row(cls, row):
        return cls(row[0], row[1], row[2], row[3], row[4], row[5], row[6],
                   row[7], row[8], row[9])

    def __lt__(self, other):
        return tuple(self) < tuple(other)

    def __tuple__(self):
        return (self.name, self.maintainer_email, self.uploader_emails,
                self.branch_url, self.vcs_type, self.vcs_url, self.vcs_browse,
                self.removed, self.vcswatch_status, self.vcswatch_version)


async def iter_packages(conn, package=None):
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


async def get_package(conn, name):
    try:
        return list(await iter_packages(conn, package=name))[0]
    except IndexError:
        return None


async def get_package_by_branch_url(conn, branch_url):
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

    __slots__ = [
            'id', 'times', 'command', 'description', 'package',
            'build_version',
            'build_distribution', 'result_code', 'branch_name',
            'main_branch_revision', 'revision', 'context', 'result',
            'suite', 'instigated_context', 'branch_url', 'logfilenames',
            'review_status']

    def __init__(self, run_id, times, command, description, package,
                 build_version,
                 build_distribution, result_code, branch_name,
                 main_branch_revision, revision, context, result,
                 suite, instigated_context, branch_url, logfilenames,
                 review_status):
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

    @property
    def duration(self):
        return self.times[1] - self.times[0]

    @classmethod
    def from_row(cls, row):
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
                   logfilenames=row[17], review_status=row[18])

    def __len__(self):
        return len(self.__slots__)

    def __tuple__(self):
        return (self.run_id, self.times, self.command, self.description,
                self.package, self.build_version, self.build_distribution,
                self.result_code, self.branch_name, self.main_branch_revision,
                self.revision, self.context, self.result, self.suite,
                self.instigated_context, self.branch_url,
                self.logfilenames, self.review_status)

    def __eq__(self, other):
        if isinstance(other, Run):
            return tuple(self) == tuple(other)
        if isinstance(other, tuple):
            return self.id == other.id
        return False

    def __lt__(self, other):
        return tuple(self) < tuple(other)

    def __getitem__(self, i):
        if isinstance(i, slice):
            return tuple(self).__getitem__(i)
        return getattr(self, self.__slots__[i])


async def get_unchanged_run(conn, main_branch_revision):
    query = """
SELECT
    id, command, start_time, finish_time, description, package,
    build_version, build_distribution, result_code,
    branch_name, main_branch_revision, revision, context, result, suite,
    instigated_context, branch_url, logfilenames, review_status
FROM
    last_runs
WHERE
    suite = 'unchanged' AND main_branch_revision = $1 AND
    build_version IS NOT NULL
"""
    if isinstance(main_branch_revision, bytes):
        main_branch_revision = main_branch_revision.decode('utf-8')
    row = await conn.fetchrow(query, main_branch_revision)
    if row is not None:
        return Run.from_row(row)
    return None


async def iter_runs(conn, package=None, run_id=None, limit=None):
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
    instigated_context, branch_url, logfilenames, review_status
FROM
    run
"""
    args = ()
    if package is not None:
        query += " WHERE package = $1 "
        args += (package,)
    if run_id is not None:
        if args:
            query += " AND id = $2 "
        else:
            query += " WHERE id = $1 "
        args += (run_id,)
    query += "ORDER BY start_time DESC"
    if limit:
        query += " LIMIT %d" % limit
    for row in await conn.fetch(query, *args):
        yield Run.from_row(row)


async def get_run(conn, run_id, package=None):
    async for run in iter_runs(conn, run_id=run_id, package=package):
        return run
    else:
        return None


async def iter_proposals(conn, package=None, suite=None):
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


async def iter_proposals_with_run(conn, package=None, suite=None):
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
        yield Run.from_row(row[:19]), row[19], row[20]


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
    def from_row(cls, row):
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


async def get_queue_position(conn, suite, package):
    ret = list(await get_queue_positions(conn, suite, [package]))
    if len(ret) == 0:
        return (None, None)
    return ret[0][1], ret[0][2]


async def get_queue_positions(conn, suite, packages):
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


async def iter_queue(conn, limit=None):
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
    package.upstream_branch_url
FROM
    queue
LEFT JOIN package ON package.name = queue.package
ORDER BY
queue.priority ASC,
queue.id ASC
"""
    if limit:
        query += " LIMIT %d" % limit
    for row in await conn.fetch(query):
        yield QueueItem.from_row(row)


async def iter_queue_with_last_run(conn, limit=None):
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
      package.upstream_branch_url,
      run.id,
      run.result_code
  FROM
      queue
  LEFT JOIN
      run
  ON
      run.id = (
          SELECT id FROM run WHERE
            package = queue.package AND run.suite = queue.suite
          ORDER BY run.start_time desc LIMIT 1)
  LEFT JOIN
      package
  ON package.name = queue.package
  ORDER BY
  queue.priority ASC,
  queue.id ASC
"""
    if limit:
        query += " LIMIT %d" % limit
    for row in await conn.fetch(query):
        yield (
            QueueItem.from_row(row[:-2]),
            row[-2], row[-1])


async def drop_queue_item(conn, queue_id):
    await conn.execute("DELETE FROM queue WHERE id = $1", queue_id)


async def add_to_queue(conn, package, command, suite, offset=0,
                       context=None, estimated_duration=None,
                       refresh=False, requestor=None,
                       requestor_relative=False):
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
        package, ' '.join(command), offset, context, estimated_duration, suite,
        refresh, requestor)
    return True


async def set_proposal_info(conn, url, status, revision, package, merged_by):
    await conn.execute("""
INSERT INTO merge_proposal (
    url, status, revision, package, merged_by)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (url)
DO UPDATE SET
  status = EXCLUDED.status,
  revision = EXCLUDED.revision,
  package = EXCLUDED.package,
  merged_by = EXCLUDED.merged_by
""", url, status, revision.decode('utf-8'), package, merged_by)


async def queue_length(conn, minimum_priority=None):
    args = []
    query = 'SELECT COUNT(*) FROM queue'
    if minimum_priority is not None:
        query += ' WHERE priority >= $1'
        args.append(minimum_priority)
    return await conn.fetchval(query, *args)


async def current_tick(conn):
    ret = await conn.fetchval('SELECT MIN(priority) FROM queue')
    if ret is None:
        ret = 0
    return ret


async def queue_duration(conn, minimum_priority=None):
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


async def iter_published_packages(conn, suite):
    return await conn.fetch("""
select distinct on (package.name) package.name, build_version, unstable_version
from run left join package on package.name = run.package
where run.build_distribution = $1 and not package.removed
order by package.name, build_version desc
""", suite)


async def get_published_by_suite(conn):
    return await conn.fetch("""
select suite, count(distinct package) from run where build_version is not null
group by 1
""")


async def iter_previous_runs(conn, package, suite):
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
  review_status
FROM
  run
WHERE
  package = $1 AND suite = $2
ORDER BY start_time DESC
""", package, suite):
        yield Run.from_row(row)


async def get_last_unabsorbed_run(conn, package, suite):
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
  review_status
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


async def iter_last_unabsorbed_runs(conn, suite, packages):
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
  review_status
FROM
  last_unabsorbed_runs
WHERE suite = $1 AND package = ANY($2::text[])
ORDER BY package, command, start_time DESC
"""
    for row in await conn.fetch(query, suite, packages):
        yield Run.from_row(row)


async def stats_by_result_codes(conn, suite=None):
    query = """\
select result_code, count(result_code) from
last_runs"""
    args = []
    if suite:
        args.append(suite)
        query += " WHERE suite = $1"
    query += " group by 1 order by 2 desc"
    return await conn.fetch(query, *args)


async def iter_last_runs(conn, result_code, suite=None):
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
  review_status
FROM last_runs
WHERE result_code = $1
"""
    args = [result_code]
    if suite:
        args.append(suite)
        query += " AND suite = $2"
    query += " ORDER BY start_time DESC"
    async with conn.transaction():
        async for row in conn.cursor(query, *args):
            yield Run.from_row(row)


async def iter_build_failures(conn):
    async with conn.transaction():
        async for row in conn.cursor("""
SELECT
  package,
  id,
  result_code,
  description
FROM run
WHERE
  (result_code = 'build-failed' OR
   result_code LIKE 'build-failed-stage-%' OR
   result_code LIKE 'build-%')
   """):
            yield row


async def update_run_result(conn, log_id, code, description):
    await conn.execute(
        'UPDATE run SET result_code = $1, description = $2 WHERE id = $3',
        code, description, log_id)


async def already_published(conn, package, branch_name, revision, mode):
    if isinstance(revision, bytes):
        revision = revision.decode('utf-8')
    row = await conn.fetchrow("""\
SELECT * FROM publish
WHERE mode = $1 AND revision = $2 AND package = $3 AND branch_name = $4
""", mode, revision, package, branch_name)
    if row:
        return True
    return False


async def iter_publish_ready(conn, suite=None, review_status=None, limit=None):
    args = []
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
  package.maintainer_email,
  package.uploader_emails,
  package.branch_url,
  publish_policy.mode,
  publish_policy.update_changelog,
  publish_policy.command
FROM
  last_unabsorbed_runs AS run
LEFT JOIN package ON package.name = run.package
LEFT JOIN publish_policy ON
    publish_policy.package = run.package AND publish_policy.suite = run.suite
WHERE result_code IN ('success', 'nothing-to-do') AND result IS NOT NULL
"""
    if suite is not None:
        query += " AND run.suite = $1 "
        args.append(suite)
    if review_status is not None:
        if not isinstance(review_status, list):
            review_status = [review_status]
        args.append(review_status)
        query += " AND review_status = ANY($%d::review_status[]) " % (
            len(args),)

    query += """
ORDER BY
  publish_policy.mode in (
        'propose', 'attempt-push', 'push-derived', 'push') DESC,
  run.finish_time DESC
"""
    if limit is not None:
        query += " LIMIT %d" % limit
    for record in await conn.fetch(query, *args):
        yield tuple(
            [Run.from_row(record[:19])] + list(record[19:-1]) +
            [shlex.split(record[-1]) if record[-1] else None])


async def iter_unscanned_branches(conn, last_scanned_minimum):
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


async def iter_package_branches(conn):
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
        conn, branch_url, canonical_branch_url,
        last_scanned=datetime.datetime.now, status=None,
        revision=None, description=None):
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


async def iter_lintian_tags(conn):
    return await conn.fetch("""
select tag, count(tag) from (
    select
      json_array_elements(
        json_array_elements(
          result->'applied')->'fixed_lintian_tags') #>> '{}' as tag
    from
      last_runs
    where
      build_distribution = 'lintian-fixes'
   ) as bypackage group by 1 order by 2
 desc
""")


async def iter_last_successes_by_lintian_tag(conn, tag):
    return await conn.fetch("""
select distinct on (package) * from (
select
  package,
  command,
  build_version,
  result_code,
  context,
  start_time,
  id,
  (json_array_elements(
     json_array_elements(
       result->'applied')->'fixed_lintian_tags') #>> '{}') as tag
from
  run
where
  build_distribution  = 'lintian-fixes' and
  result_code = 'success'
) as package where tag = $1 order by package, start_time desc
""", tag)


async def get_run_result_by_revision(conn, revision):
    row = await conn.fetchrow("""
SELECT result, branch_name, review_status FROM run WHERE revision = $1""",
revision.decode('utf-8'))
    if row is not None:
        return row[0], row[1], row[2]
    return None, None


async def get_last_build_version(conn, package, suite):
    return await conn.fetchval(
        "SELECT build_version FROM run WHERE "
        "build_version IS NOT NULL AND package = $1 AND "
        "build_distribution = $2 ORDER BY build_version DESC",
        package, suite)


async def estimate_duration(conn, package=None, suite=None):
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


async def store_candidates(conn, entries):
    await conn.executemany(
        "INSERT INTO candidate (package, suite, context, value) "
        "VALUES ($1, $2, $3, $4) ON CONFLICT (package, suite) "
        "DO UPDATE SET context = EXCLUDED.context, value = EXCLUDED.value",
        entries)


async def iter_candidates(conn, packages=None, suite=None):
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
  candidate.value
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
    return [([Package.from_row(row)] + list(row[10:]))
            for row in await conn.fetch(query, *args)]


async def get_candidate(conn, package, suite):
    return await conn.fetchrow(
        "SELECT context, value FROM candidate "
        "WHERE package = $1 AND suite = $2", package, suite)


async def iter_sources_with_unstable_version(conn, packages):
    return await conn.fetch(
        "SELECT name, unstable_version FROM package "
        "WHERE name = any($1::text[])", packages)


async def iter_packages_by_maintainer(conn, maintainer):
    return [(row[0], row[1]) for row in await conn.fetch(
        "SELECT name, removed FROM package WHERE "
        "maintainer_email = $1 OR $1 = any(uploader_emails)",
        maintainer)]


async def get_never_processed(conn, suites=None):
    if suites is not None:
        args = [suites]
        query = """\
SELECT suite, COUNT(suite) FROM package p CROSS JOIN UNNEST ($1::text[]) suite
WHERE NOT EXISTS
(SELECT FROM run WHERE run.package = p.name AND run.suite = suite)
GROUP BY suite
    """
    else:
        args = []
        query = """\
SELECT suites.name, COUNT(suites.name) FROM package p CROSS JOIN suites
WHERE NOT EXISTS
(SELECT FROM run WHERE run.package = p.name AND run.suite = suites.name)
GROUP BY suites.name
    """
    return await conn.fetch(query, *args)


async def iter_by_suite_result_code(conn):
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


async def get_merge_proposal_run(conn, mp_url):
    query = """
SELECT
    run.id, run.command, run.start_time, run.finish_time, run.description,
    run.package, run.build_version, run.build_distribution, run.result_code,
    run.branch_name, run.main_branch_revision, run.revision, run.context,
    run.result, run.suite, run.instigated_context, run.branch_url,
    run.logfilenames, run.review_status
FROM run inner join merge_proposal on merge_proposal.revision = run.revision
WHERE merge_proposal.url = $1
ORDER BY run.finish_time DESC
"""
    row = await conn.fetchrow(query, mp_url)
    if row:
        return Run.from_row(row)
    return None


async def get_proposal_info(conn, url):
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
    return (row[1].encode('utf-8'), row[2], row[3], row[0])


async def iter_publish_history(conn, limit=None):
    query = """
SELECT
    publish.package, publish.branch_name, publish.main_branch_revision,
    publish.revision, publish.mode, publish.merge_proposal_url,
    publish.result_code, publish.description, package.vcs_browse
FROM
    publish
JOIN package ON publish.package = package.name
ORDER BY timestamp DESC
"""
    if limit:
        query += " LIMIT %d" % limit
    for row in await conn.fetch(query):
        yield row


async def get_open_merge_proposal(conn, package, branch_name):
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


async def get_publish(conn, publish_id):
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
    return await conn.fetchrow(query, publish_id)


async def update_removals(conn, items):
    if not items:
        return
    query = """\
UPDATE package SET removed = True WHERE name = $1 AND unstable_version <= $2
"""
    await conn.executemany(query, items)


async def iter_failed_lintian_fixers(conn):
    query = """
select json_object_keys(result->'failed'), count(*) from last_runs
where
  suite = 'lintian-fixes' and
  json_typeof(result->'failed') = 'object' group by 1 order by 2 desc
"""
    return await conn.fetch(query)


async def iter_lintian_brush_fixer_failures(conn, fixer):
    query = """
select id, package, result->'failed'->$1 FROM last_runs
where
  suite = 'lintian-fixes' and (result->'failed')::jsonb?$1
"""
    return await conn.fetch(query, fixer)


async def iter_lintian_fixes_regressions(conn):
    query = """
SELECT l.package, l.id, u.id, l.result_code FROM last_runs l
   INNER JOIN last_runs u ON l.main_branch_revision = u.main_branch_revision
   WHERE
    l.suite = 'lintian-fixes' AND
    u.suite = 'unchanged' AND
    l.result_code NOT IN ('success', 'nothing-to-do') AND
    u.result_code = 'success'
"""
    return await conn.fetch(query)


async def version_available(conn, package, suite, version=None):
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
  unstable_version
FROM
  package
WHERE name = $1 AND %(version_match2)s
"""
    args = [package, suite]
    if version:
        query = query % {
            'version_match1': "build_version %s $3" % (version[0], ),
            'version_match2': "unstable_version %s $3" % (version[0], )}
        args.append(version[1])
    else:
        query = query % {
            'version_match1': 'True',
            'version_match2': 'True'}
    return await conn.fetch(query, *args)


async def set_run_review_status(conn, run_id, review_status):
    await conn.execute('UPDATE run SET review_status = $1 WHERE id = $2',
                       review_status, run_id)


async def iter_vcs_regressions(conn):
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


async def iter_review_status(conn):
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


async def iter_missing_upstream_branch_packages(conn):
    query = """\
select
  package.name,
  package.unstable_version
from
  last_runs
inner join package on last_runs.package = package.name
where
  result_code = 'upstream-branch-unknown' and
  package.upstream_branch_url is null
order by package.name asc
"""
    for row in await conn.fetch(query):
        yield row[0], row[1]


async def set_upstream_branch_url(conn, package, url):
    await conn.execute(
        'update package set upstream_branch_url = $1 where name = $2',
        url, package)


async def iter_upstream_branch_urls(conn):
    query = """
select
  name,
  upstream_branch_url
from package
where upstream_branch_url is not null
"""
    return await conn.fetch(query)


async def update_branch_url(conn, package, vcs_type, vcs_url):
    await conn.execute(
        'update package set vcs_type = $1, branch_url = $2 '
        'where name = $3', vcs_type, vcs_url, package)


async def update_publish_policy(
        conn, name, suite, publish_mode, changelog_mode, command):
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


async def iter_publish_policy(conn, package=None):
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


async def get_publish_policy(conn, package, suite):
    row = await conn.fetchrow(
        'SELECT mode, update_changelog, command '
        'FROM publish_policy WHERE package = $1 AND suite = $2', package,
        suite)
    if row:
        return (row[0], row[1], shlex.split(row[2]) if row[2] else None)


async def iter_absorbed_lintian_fixes(conn):
    return await conn.fetch(
        "select unnest(fixed_lintian_tags), count(*) from absorbed_lintian_fixes "
        "group by 1 order by 2 desc")
