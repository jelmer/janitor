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
from . import SUITES


DEFAULT_URL = 'postgresql://janitor@brangwain.vpn.jelmer.uk:5432/janitor'


pool = None


@asynccontextmanager
async def get_connection():
    global pool
    if pool is None:
        pool = await asyncpg.create_pool(DEFAULT_URL)

    async with pool.acquire() as conn:
        await conn.set_type_codec(
                    'json',
                    encoder=json.dumps,
                    decoder=json.loads,
                    schema='pg_catalog'
                )
        yield conn


async def store_packages(packages):
    """Store packages in the database.

    Args:
      packages: list of tuples with (
        name, branch_url, maintainer_email, uploader_emails, unstable_version,
        vcs_type, vcs_url, vcs_browse, popcon_inst, removed)
    """
    async with get_connection() as conn:
        await conn.executemany(
            "INSERT INTO package "
            "(name, branch_url, maintainer_email, uploader_emails, "
            "unstable_version, vcs_type, vcs_url, vcs_browse, popcon_inst, "
            "removed) "
            "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) "
            "ON CONFLICT (name) DO UPDATE SET "
            "branch_url = EXCLUDED.branch_url, "
            "maintainer_email = EXCLUDED.maintainer_email, "
            "uploader_emails = EXCLUDED.uploader_emails, "
            "unstable_version = EXCLUDED.unstable_version, "
            "vcs_type = EXCLUDED.vcs_type, "
            "vcs_url = EXCLUDED.vcs_url, "
            "vcs_browse = EXCLUDED.vcs_browse, "
            "popcon_inst = EXCLUDED.popcon_inst, "
            "removed = EXCLUDED.removed",
            packages)


async def popcon():
    async with get_connection() as conn:
        return await conn.fetch(
            "SELECT name, popcon_inst FROM package")


async def store_run(
        run_id, name, vcs_url, start_time, finish_time,
        command, description, instigated_context, context,
        main_branch_revision, result_code, build_version,
        build_distribution, branch_name, revision, subworker_result, suite):
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
    """
    async with get_connection() as conn:
        await conn.execute(
            "INSERT INTO run (id, command, description, result_code, "
            "start_time, finish_time, package, instigated_context, context, "
            "build_version, build_distribution, main_branch_revision, "
            "branch_name, revision, result, suite, branch_url) "
            "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, "
            "$14, $15, $16, $17)",
            run_id, ' '.join(command), description, result_code,
            start_time, finish_time, name, instigated_context, context,
            str(build_version) if build_version else None, build_distribution,
            main_branch_revision, branch_name, revision,
            subworker_result if subworker_result else None, suite,
            vcs_url)


async def store_publish(package, branch_name, main_branch_revision, revision,
                        mode, result_code, description,
                        merge_proposal_url=None):
    async with get_connection() as conn:
        if merge_proposal_url:
            await conn.execute(
                "INSERT INTO merge_proposal (url, package, status, revision) "
                "VALUES ($1, $2, 'open', $3) ON CONFLICT (url) DO UPDATE SET "
                "package = EXCLUDED.package, revision = EXCLUDED.revision",
                merge_proposal_url, package, revision)
        await conn.execute(
            "INSERT INTO publish (package, branch_name, "
            "main_branch_revision, revision, mode, result_code, description, "
            "merge_proposal_url) values ($1, $2, $3, $4, $5, $6, $7, $8) ",
            package, branch_name, main_branch_revision, revision, mode,
            result_code, description, merge_proposal_url)


class Package(object):

    def __init__(self, name, maintainer_email, uploader_emails, branch_url,
                 vcs_type, vcs_url, vcs_browse, removed):
        self.name = name
        self.maintainer_email = maintainer_email
        self.uploader_emails = uploader_emails
        self.branch_url = branch_url
        self.vcs_type = vcs_type
        self.vcs_url = vcs_url
        self.vcs_browse = vcs_browse
        self.removed = removed

    @classmethod
    def from_row(cls, row):
        return cls(row[0], row[1], row[2], row[3], row[4], row[5], row[6], row[7])

    def __lt__(self, other):
        return tuple(self) < tuple(other)

    def __tuple__(self):
        return (self.name, self.maintainer_email, self.uploader_emails,
                self.branch_url, self.vcs_type, self.vcs_url, self.vcs_browse,
                self.removed)


async def iter_packages(package=None):
    query = """
SELECT
  name,
  maintainer_email,
  uploader_emails,
  branch_url,
  vcs_type,
  vcs_url,
  vcs_browse,
  removed
FROM
  package
"""
    args = []
    if package:
        query += " WHERE name = $1"
        args.append(package)
    query += " ORDER BY name ASC"
    async with get_connection() as conn:
        return [
            Package.from_row(row) for row in await conn.fetch(query, *args)]


async def get_package(name):
    return list(await iter_packages(package=name))[0]


class Run(object):

    __slots__ = [
            'id', 'times', 'command', 'description', 'package',
            'build_version',
            'build_distribution', 'result_code', 'branch_name',
            'main_branch_revision', 'revision', 'context', 'result',
            'suite', 'instigated_context', 'branch_url']

    def __init__(self, run_id, times, command, description, package,
                 build_version,
                 build_distribution, result_code, branch_name,
                 main_branch_revision, revision, context, result,
                 suite, instigated_context, branch_url):
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
                   instigated_context=row[15], branch_url=row[16])

    def __len__(self):
        return len(self.__slots__)

    def __tuple__(self):
        return (self.run_id, self.times, self.command, self.description,
                self.package, self.build_version, self.build_distribution,
                self.result_code, self.branch_name, self.main_branch_revision,
                self.revision, self.context, self.result, self.suite,
                self.instigated_context, self.branch_url)

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


async def iter_runs(package=None, run_id=None, limit=None):
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
    instigated_context, branch_url
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
    async with get_connection() as conn:
        for row in await conn.fetch(query, *args):
            yield Run.from_row(row)


async def get_maintainer_email_for_proposal(vcs_url):
    async with get_connection() as conn:
        return await conn.fetchval("""
SELECT
    maintainer_email
FROM
    package
LEFT JOIN merge_proposal ON merge_proposal.package = package.name
WHERE
    merge_proposal.url = $1""", vcs_url)


async def iter_proposals(package=None):
    args = []
    query = """
SELECT
    package, url, status, revision
FROM
    merge_proposal
"""
    if package:
        args.append(package)
        query += " WHERE package = $1"
    async with get_connection() as conn:
        return await conn.fetch(query, *args)


async def iter_all_proposals(branch_name=None):
    args = []
    query = """
SELECT
    merge_proposal.url,
    merge_proposal.status,
    merge_proposal.package,
    merge_proposal.revision
FROM
    merge_proposal
LEFT JOIN publish ON publish.merge_proposal_url = merge_proposal.url
"""
    if branch_name:
        query += " WHERE publish.branch_name = $1"
        args.append(branch_name)
    async with get_connection() as conn:
        return await conn.fetch(query, *args)


class QueueItem(object):

    __slots__ = ['id', 'branch_url', 'package', 'env', 'command',
                 'estimated_duration', 'suite']

    def __init__(self, id, branch_url, package, env, command,
                 estimated_duration, suite):
        self.id = id
        self.package = package
        self.branch_url = branch_url
        self.env = env
        self.command = command
        self.estimated_duration = estimated_duration
        self.suite = suite

    @classmethod
    def from_row(cls, row):
        (branch_url, package, committer,
            command, context, queue_id, estimated_duration,
            suite) = row
        env = {
            'COMMITTER': committer or None,
            'CONTEXT': context,
        }
        return cls(
                id=queue_id, branch_url=branch_url,
                package=package, env=env,
                command=shlex.split(command),
                estimated_duration=estimated_duration,
                suite=suite)

    def _tuple(self):
        return (self.id, self.branch_url, self.package, self.env, self.command,
                self.estimated_duration, self.suite)

    def __eq__(self, other):
        if isinstance(other, QueueItem):
            return self.id == other.id
        return False

    def __lt__(self, other):
        return self.id < other.id


async def iter_queue(limit=None):
    query = """
SELECT
    queue.branch_url,
    queue.package,
    queue.committer,
    queue.command,
    queue.context,
    queue.id,
    queue.estimated_duration,
    queue.suite
FROM
    queue
ORDER BY
queue.priority ASC,
queue.id ASC
"""
    if limit:
        query += " LIMIT %d" % limit
    async with get_connection() as conn:
        for row in await conn.fetch(query):
            yield QueueItem.from_row(row)


async def drop_queue_item(queue_id):
    async with get_connection() as conn:
        await conn.execute("DELETE FROM queue WHERE id = $1", queue_id)


async def add_to_queue(vcs_url, package, command, suite, offset=0,
                       context=None, committer=None, estimated_duration=None):
    async with get_connection() as conn:
        await conn.execute(
            "INSERT INTO queue "
            "(branch_url, package, command, committer, priority, context, "
            "estimated_duration, suite) "
            "VALUES "
            "($1, $2, $3, $4,"
            "(SELECT COALESCE(MIN(priority), 0) FROM queue) + $5, "
            "$6, $7, $8) ON CONFLICT (package, command) DO UPDATE SET "
            "context = EXCLUDED.context, priority = EXCLUDED.priority, "
            "estimated_duration = EXCLUDED.estimated_duration "
            "WHERE queue.priority >= EXCLUDED.priority",
            vcs_url, package, ' '.join(command), committer,
            offset, context, estimated_duration, suite)
        return True


async def set_proposal_status(url, status):
    async with get_connection() as conn:
        await conn.execute("""
INSERT INTO merge_proposal (url, status) VALUES ($1, $2)
ON CONFLICT (url) DO UPDATE SET status = EXCLUDED.status
""", url, status)


async def queue_length(minimum_priority=None):
    args = []
    query = 'SELECT COUNT(*) FROM queue'
    if minimum_priority is not None:
        query += ' WHERE priority >= $1'
        args.append(minimum_priority)
    async with get_connection() as conn:
        return await conn.fetchval(query, *args)


async def current_tick():
    async with get_connection() as conn:
        ret = await conn.fetchval('SELECT MIN(priority) FROM queue')
        if ret is None:
            ret = 0
        return ret


async def queue_duration(minimum_priority=None):
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
    async with get_connection() as conn:
        ret = (await conn.fetchrow(query, *args))[0]
        if ret is None:
            return datetime.timedelta()
        return ret


async def iter_published_packages(suite):
    async with get_connection() as conn:
        return await conn.fetch("""
select distinct package, build_version from run where build_distribution = $1
""", suite, )


async def get_published_by_suite():
    async with get_connection() as conn:
        return await conn.fetch("""
select suite, count(distinct package) from run where build_version is not null
group by 1
""")


async def iter_previous_runs(package, suite):
    async with get_connection() as conn:
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
  branch_url
FROM
  run
WHERE
  package = $1 AND suite = $2
ORDER BY start_time DESC
""", package, suite):
            yield Run.from_row(row)


async def get_last_success(package, suite):
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
  branch_url
FROM
  run
WHERE package = $1 AND build_distribution = $2
ORDER BY package, command, result_code = 'success' DESC, start_time DESC
LIMIT 1
"""
    args = [package, suite]
    async with get_connection() as conn:
        row = await conn.fetchrow(query, *args)
        if row is None:
            return None
        return Run.from_row(row)


async def iter_last_successes(suite, packages):
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
  branch_url
FROM
  run
WHERE suite = $1 AND package = ANY($2::text[])
ORDER BY package, command, result_code = 'success' DESC, start_time DESC
"""
    async with get_connection() as conn:
        for row in await conn.fetch(query, suite, packages):
            yield Run.from_row(row)


async def stats_by_result_codes():
    query = """\
select result_code, count(result_code) from (select distinct on(package, suite)
package, suite, result_code from run order by 1, 2, start_time desc) AS results
group by 1 order by 2 desc
"""
    async with get_connection() as conn:
        return await conn.fetch(query)


async def iter_last_runs(result_code):
    query = """
SELECT package, suite, command, id, description, start_time, duration FROM (
SELECT DISTINCT ON (package, suite)
  package,
  suite,
  command,
  id,
  description,
  start_time,
  finish_time - start_time AS duration,
  result_code
FROM
  run
ORDER BY package, suite, start_time DESC) AS runs
WHERE result_code = $1 ORDER BY start_time DESC
"""
    async with get_connection() as conn:
        return await conn.fetch(query, result_code)


async def iter_build_failures():
    async with get_connection() as conn:
        return await conn.fetch("""
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
""")


async def update_run_result(log_id, code, description):
    async with get_connection() as conn:
        await conn.execute(
            'UPDATE run SET result_code = $1, description = $2 WHERE id = $3',
            code, description, log_id)


async def already_published(package, branch_name, revision, mode):
    async with get_connection() as conn:
        row = await conn.fetchrow("""\
SELECT * FROM publish
WHERE mode = $1 AND revision = $2 AND package = $3 AND branch_name = $4
""", mode, revision, package, branch_name)
        if row:
            return True
        return False


async def iter_publish_ready(suite=None):
    args = []
    query = """
SELECT DISTINCT ON (package, command)
  package.name,
  run.command,
  run.build_version,
  run.result_code,
  run.context,
  run.start_time,
  run.id,
  run.revision,
  run.result,
  run.branch_name,
  run.suite,
  package.maintainer_email,
  package.uploader_emails,
  package.branch_url,
  main_branch_revision
FROM
  run
LEFT JOIN package ON package.name = run.package
WHERE result_code = 'success' AND result IS NOT NULL
"""
    if suite is not None:
        query += " AND build_distribution = $1 "
        args.append(suite)

    query += """
ORDER BY
  package,
  command,
  finish_time DESC
"""
    async with get_connection() as conn:
        return await conn.fetch(query, *args)


async def iter_unscanned_branches(last_scanned_minimum):
    async with get_connection() as conn:
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


async def iter_package_branches():
    async with get_connection() as conn:
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
        branch_url, last_scanned=None, status=None, revision=None,
        description=None):
    async with get_connection() as conn:
        await conn.execute(
            "INSERT INTO branch (url, status, revision, last_scanned, "
            "description) VALUES ($1, $2, $3, $4, $5) "
            "ON CONFLICT (url) DO UPDATE SET "
            "status = EXCLUDED.status, revision = EXCLUDED.revision, "
            "last_scanned = EXCLUDED.last_scanned, "
            "description = EXCLUDED.description",
            branch_url, status, revision.decode('utf-8') if revision else None,
            last_scanned, description)


async def iter_lintian_tags():
    async with get_connection() as conn:
        return await conn.fetch("""
select tag, count(tag) from (
    select distinct on (package)
      package,
      json_array_elements(
        json_array_elements(
          result->'applied')->'fixed_lintian_tags') #>> '{}' as tag
    from
      run
    where
      build_distribution = 'lintian-fixes'
    order by package, start_time desc) as bypackage group by 1
""")


async def iter_last_successes_by_lintian_tag(tag):
    async with get_connection() as conn:
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


async def get_run_result_by_revision(revision):
    async with get_connection() as conn:
        return await conn.fetchval("""
SELECT result FROM run WHERE revision = $1""", revision.decode('utf-8'))


async def get_last_build_version(package, suite):
    async with get_connection() as conn:
        return await conn.fetchval(
            "SELECT build_version FROM run WHERE "
            "build_version IS NOT NULL AND package = $1 AND "
            "build_distribution = $2 ORDER BY build_version DESC",
            package, suite)


async def estimate_duration(package, suite=None):
    query = """
SELECT finish_time - start_time FROM run
WHERE package = $1"""
    args = [package]
    if suite is not None:
        query += " AND suite = $2"
        args.append(suite)
    query += " ORDER BY start_time DESC LIMIT 1"
    async with get_connection() as conn:
        return await conn.fetchval(query, *args)


async def store_candidates(entries):
    async with get_connection() as conn:
        await conn.executemany(
            "INSERT INTO candidate (package, suite, command, context, value) "
            "VALUES ($1, $2, $3, $4, $5) ON CONFLICT (package, suite) "
            "DO UPDATE SET command = EXCLUDED.command, "
            "context = EXCLUDED.context, value = EXCLUDED.value",
            entries)


async def iter_candidates(packages=None, suite=None):
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
  candidate.suite,
  candidate.command,
  candidate.context,
  candidate.value
FROM candidate
INNER JOIN package on package.name = candidate.package
"""
    args = []
    if suite is not None and packages is not None:
        query += " WHERE package = ANY($1::text[]) AND suite = $2"
        args.extend([packages, suite])
    elif suite is not None:
        query += " WHERE suite = $1"
        args.append(suite)
    elif packages is not None:
        query += " WHERE package = ANY($1::text[])"
        args.append(packages)
    async with get_connection() as conn:
        return [([Package.from_row(row)] + list(row[8:]))
                for row in await conn.fetch(query, *args)]


async def get_candidate(package, suite):
    async with get_connection() as conn:
        return await conn.fetchrow(
            "SELECT command, context, value FROM candidate "
            "WHERE package = $1 AND suite = $2", package, suite)


async def iter_sources_with_unstable_version(packages):
    async with get_connection() as conn:
        return await conn.fetch(
            "SELECT name, unstable_version FROM package "
            "WHERE name = any($1::text[])", packages)


async def iter_packages_by_maintainer(maintainer):
    async with get_connection() as conn:
        return [row[0] for row in await conn.fetch(
            "SELECT name FROM package WHERE "
            "maintainer_email = $1 OR $1 = any(uploader_emails)",
            maintainer)]


async def get_never_processed():
    async with get_connection() as conn:
        query = """\
SELECT COUNT(*) FROM package p CROSS JOIN UNNEST ($1::text[]) suite WHERE NOT EXISTS (
    SELECT FROM run WHERE run.package = p.name AND run.suite = suite)
    """
        return await conn.fetchval(query, list(SUITES))
