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
import asyncio

loop = asyncio.get_event_loop()
pool = loop.run_until_complete(asyncpg.create_pool(
    database="janitor",
    user="janitor",
    port=5432,
    host="brangwain.vpn.jelmer.uk"))


from contextlib import asynccontextmanager

@asynccontextmanager
async def get_connection():
    async with pool.acquire() as conn:
        await conn.set_type_codec(
                    'json',
                    encoder=json.dumps,
                    decoder=json.loads,
                    schema='pg_catalog'
                )
        yield conn


async def _ensure_package(conn, name, vcs_url, maintainer_email):
    await conn.execute(
        "INSERT INTO package (name, branch_url, maintainer_email) "
        "VALUES ($1, $2, $3) ON CONFLICT (name) DO UPDATE SET "
        "branch_url = EXCLUDED.branch_url, "
        "maintainer_email = EXCLUDED.maintainer_email",
        (name, vcs_url, maintainer_email))


async def store_run(
        run_id, name, vcs_url, maintainer_email, start_time, finish_time,
        command, description, instigated_context, context,
        main_branch_revision, result_code, build_version,
        build_distribution, branch_name, revision, subworker_result):
    """Store a run.

    :param run_id: Run id
    :param name: Package name
    :param vcs_url: Upstream branch URL
    :param maintainer_email: Maintainer email
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
    """
    async with get_connection() as conn:
        await _ensure_package(conn, name, vcs_url, maintainer_email)
        await conn.execute(
            "INSERT INTO run (id, command, description, result_code, start_time, "
            "finish_time, package, instigated_context, context, build_version, "
            "build_distribution, main_branch_revision, branch_name, revision, "
            "result) "
            "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14"
            ", $15)",
            (run_id, ' '.join(command), description, result_code,
             start_time, finish_time, name, instigated_context, context,
             str(build_version) if build_version else None, build_distribution,
             main_branch_revision, branch_name, revision,
             subworker_result if subworker_result else None))


async def store_publish(package, branch_name, main_branch_revision, revision,
                        mode, result_code, description,
                        merge_proposal_url=None):
    async with get_connection() as conn:
        if merge_proposal_url:
            await conn.execute(
                "INSERT INTO merge_proposal (url, package, status) "
                "VALUES ($1, $2, 'open') ON CONFLICT (url) DO UPDATE SET "
                "package = EXCLUDED.package",
                (merge_proposal_url, package))
        await conn.execute("""
INSERT INTO publish (package, branch_name, main_branch_revision, revision,
mode, result_code, description, merge_proposal_url) values ($1, $2, $3, $4, $5,
$6, $7, $8)
""", (package, branch_name, main_branch_revision, revision, mode, result_code,
      description, merge_proposal_url))


async def iter_packages(package=None):
    query = """
SELECT
  name,
  maintainer_email,
  branch_url
FROM
  package
"""
    args = []
    if package:
        query += " WHERE name = $1"
        args.append(package)
    query += " ORDER BY name ASC"
    async with get_connection() as conn:
        return await conn.fetch(query, *args)


async def iter_runs(package=None, run_id=None, limit=None):
    """Iterate over runs.

    Args:
      package: package to restrict to
    Returns:
      iterator over (
        run_id, (start_time, finish_time), command, description,
        package_name, merge_proposal_url, build_version, build_distribution,
        result_code, branch_name)
    """
    query = """
SELECT
    run.id, command, start_time, finish_time, description, package.name,
    run.merge_proposal_url, build_version, build_distribution, result_code,
    branch_name
FROM
    run
LEFT JOIN package ON package.name = run.package
"""
    args = ()
    if package is not None:
        query += " WHERE package.name = $1 "
        args += (package,)
    if run_id is not None:
        if args:
            query += " AND run.id = $2 "
        else:
            query += " WHERE run.id = $1 "
        args += (run_id,)
    query += "ORDER BY start_time DESC"
    if limit:
        query += " LIMIT %d" % limit
    async with get_connection() as conn:
        cur = await conn.cursor(query, *args)
        row = await cur.fetchrow()
        while row:
            yield (row[0],
                   (row[2], row[3]),
                   row[1], row[4], row[5], row[6],
                   Version(row[7]) if row[7] else None, row[8],
                   row[9] if row[9] else None, row[10])
            row = await cur.fetchrow()


async def get_maintainer_email(vcs_url):
    async with get_connection() as conn:
        return await conn.fetchval("""
SELECT
    maintainer_email
FROM
    package
LEFT JOIN merge_proposal ON merge_proposal.package = package.name
WHERE
    merge_proposal.url = $1""", (vcs_url, ))


async def iter_proposals(package=None):
    args = []
    query = """
SELECT
    package, url, status
FROM
    merge_proposal
LEFT JOIN package ON merge_proposal.package = package.name
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
    merge_proposal.url, merge_proposal.status, package.name
FROM
    merge_proposal
LEFT JOIN package ON merge_proposal.package = package.name
LEFT JOIN publish ON publish.merge_proposal_url = merge_proposal.url
"""
    if branch_name:
        query += " WHERE branch_name = $1"
        args.append(branch_name)
    async with get_connection() as conn:
        return await conn.fetch(query, *args)


async def iter_queue(limit=None):
    query = """
SELECT
    package.branch_url,
    package.maintainer_email,
    package.name,
    queue.committer,
    queue.command,
    queue.context,
    queue.id
FROM
    queue
LEFT JOIN package ON package.name = queue.package
ORDER BY
queue.priority DESC,
queue.id ASC
"""
    if limit:
        query += " LIMIT %d" % limit
    async with get_connection() as conn:
        for row in await conn.fetch(query):
            (branch_url, maintainer_email, package, committer,
                command, context, queue_id) = row
            env = {
                'PACKAGE': package,
                'MAINTAINER_EMAIL': maintainer_email,
                'COMMITTER': committer or None,
                'CONTEXT': context,
            }
            yield (queue_id, branch_url, env, shlex.split(command))


async def drop_queue_item(queue_id):
    async with get_connection() as conn:
        await conn.execute("DELETE FROM queue WHERE id = $1", (queue_id,))


async def add_to_queue(vcs_url, env, command, priority=0,
                       estimated_duration=None):
    package = env['PACKAGE']
    maintainer_email = env.get('MAINTAINER_EMAIL')
    context = env.get('CONTEXT')
    committer = env.get('COMMITTER')
    async with get_connection() as conn:
        await _ensure_package(conn, package, vcs_url, maintainer_email)
        await conn.execute(
            "INSERT INTO queue "
            "(branch_url, package, command, committer, priority, context, "
            "estimated_duration) "
            "VALUES ($1, $2, $3, $4, $5, $6, $7) "
            "ON CONFLICT (package, command) DO UPDATE SET "
            "context = EXCLUDED.context, priority = EXCLUDED.priority, "
            "estimated_duration = EXCLUDED.estimated_duration "
            "WHERE queue.priority <= EXCLUDED.priority", (
                vcs_url, package, ' '.join(command), committer,
                priority, context, estimated_duration))
        return True


async def set_proposal_status(url, status):
    await conn.execute("""
INSERT INTO merge_proposal (url, status) VALUES ($1, $2)
ON CONFLICT (url) DO UPDATE SET status = EXCLUDED.status
""", (url, status))


async def queue_length(minimum_priority=None):
    args = []
    query = 'SELECT COUNT(*) FROM queue'
    if minimum_priority is not None:
        query += ' WHERE priority >= $1'
        args.append(minimum_priority)
    async with get_connection() as conn:
        return await conn.fetchval(query, *args)


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
        ret = await conn.fetchrow(query, *args)[0]
        if ret is None:
            return datetime.timedelta()
        return ret


async def iter_published_packages(suite):
    async with get_connection() as conn:
        return await conn.fetch("""
select distinct package, build_version from run where build_distribution = $1
""", (suite, ))


async def iter_previous_runs(package, command):
    async with get_connection() as conn:
        return await conn.fetch("""
SELECT
  start_time,
  (finish_time - start_time) AS duration,
  instigated_context,
  context,
  main_branch_revision,
  result_code
FROM
  run
WHERE
  package = $1 AND command = $2
ORDER BY start_time DESC
""", (package, ' '.join(command)))


async def iter_last_successes(suite=None):
    args = []
    query = """
SELECT DISTINCT ON (package, command)
  package,
  command,
  build_version,
  result_code,
  context,
  start_time,
  id
FROM
  run
"""
    if suite is not None:
        query += " WHERE build_distribution = $1"
        args.append(suite)
    query += """
ORDER BY package, command, result_code = 'success' DESC, start_time DESC
"""
    async with get_connection() as conn:
        return await conn.fetch(query, *args)


async def iter_last_runs(command=None):
    args = []
    query = """
SELECT DISTINCT ON (package, command)
  package,
  command,
  result_code,
  id,
  description,
  finish_time - start_time
FROM
  run
"""
    if command:
        query += " WHERE command = $1"
        args.append(command)
    query += " ORDER BY package, command, start_time DESC"
    async with get_connection() as conn:
        return await conn.fetch(query, *args)


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
        (code, description, log_id))


async def already_published(package, branch_name, revision, mode):
    async with get_connection() as conn:
        row = await conn.fetchrow("""\
SELECT * FROM publish
WHERE mode = $1 AND revision = $2 AND package = $3 AND branch_name = $4
""", (mode, revision, package, branch_name))
        if row:
            return True
        return False


async def iter_publish_ready():
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
  package.maintainer_email,
  package.branch_url,
  main_branch_revision
FROM
  run
LEFT JOIN package ON package.name = run.package
WHERE result_code = 'success' AND result IS NOT NULL
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
""", (last_scanned_minimum, ))


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
        await conn.execute("""\
INSERT INTO branch (url, status, revision, last_scanned, description)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (url) DO UPDATE SET
  status = EXCLUDED.status,
  revision = EXCLUDED.revision,
  last_scanned = EXCLUDED.last_scanned,
  description = EXCLUDED.description
""", (branch_url, status, revision.decode('utf-8') if revision else None,
      last_scanned, description))
