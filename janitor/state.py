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
import shlex
import psycopg2
from psycopg2.extras import Json


conn = psycopg2.connect(
    database="janitor",
    user="janitor",
    port=5432,
    host="brangwain.vpn.jelmer.uk")
conn.set_client_encoding('UTF8')


def _ensure_package(cur, name, vcs_url, maintainer_email):
    cur.execute(
        "INSERT INTO package (name, branch_url, maintainer_email) "
        "VALUES (%s, %s, %s) ON CONFLICT (name) DO UPDATE SET "
        "branch_url = EXCLUDED.branch_url, "
        "maintainer_email = EXCLUDED.maintainer_email",
        (name, vcs_url, maintainer_email))


def store_run(run_id, name, vcs_url, maintainer_email, start_time, finish_time,
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
    cur = conn.cursor()
    _ensure_package(cur, name, vcs_url, maintainer_email)
    cur.execute(
        "INSERT INTO run (id, command, description, result_code, start_time, "
        "finish_time, package, instigated_context, context, build_version, "
        "build_distribution, main_branch_revision, branch_name, revision, "
        "result) "
        "VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)",
        (run_id, ' '.join(command), description, result_code,
         start_time, finish_time, name, instigated_context, context,
         str(build_version) if build_version else None, build_distribution,
         main_branch_revision, branch_name, revision,
         Json(subworker_result) if subworker_result else None))
    conn.commit()


def store_publish(package, branch_name, main_branch_revision, revision, mode,
                  result_code, description, merge_proposal_url=None):
    cur = conn.cursor()
    if merge_proposal_url:
        cur.execute(
            "INSERT INTO merge_proposal (url, package, status) "
            "VALUES (%s, %s, 'open') ON CONFLICT (url) DO UPDATE SET "
            "package = EXCLUDED.package",
            (merge_proposal_url, package))
    cur.execute("""
INSERT INTO publish (package, branch_name, main_branch_revision, revision,
mode, result_code, description, merge_proposal_url) values (%s, %s, %s, %s, %s,
%s, %s, %s)
""", (package, branch_name, main_branch_revision, revision, mode, result_code,
      description, merge_proposal_url))
    conn.commit()


def iter_packages(package=None):
    cur = conn.cursor()
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
        query += " WHERE name = %s"
        args.append(name)
    query += " ORDER BY name ASC"
    cur.execute(query, args)
    return cur.fetchall()


def iter_runs(package=None, run_id=None, limit=None):
    """Iterate over runs.

    Args:
      package: package to restrict to
    Returns:
      iterator over (
        run_id, (start_time, finish_time), command, description,
        package_name, merge_proposal_url, build_version, build_distribution,
        result_code, branch_name)
    """
    cur = conn.cursor()
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
        query += " WHERE package.name = %s "
        args += (package,)
    if run_id is not None:
        if args:
            query += " AND "
        else:
            query += " WHERE "
        query += " run.id = %s "
        args += (run_id,)
    query += "ORDER BY start_time DESC"
    if limit:
        query += " LIMIT %d" % limit
    cur.execute(query, args)
    row = cur.fetchone()
    while row:
        yield (row[0],
               (row[2], row[3]),
               row[1], row[4], row[5], row[6],
               Version(row[7]) if row[7] else None, row[8],
               row[9] if row[9] else None, row[10])
        row = cur.fetchone()


def get_maintainer_email(vcs_url):
    cur = conn.cursor()
    cur.execute(
        """
SELECT
    maintainer_email
FROM
    package
LEFT JOIN merge_proposal ON merge_proposal.package = package.name
WHERE
    merge_proposal.url = %s""",
        (vcs_url, ))
    row = cur.fetchone()
    if row:
        return row[0]
    return None


def iter_proposals(package=None):
    cur = conn.cursor()
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
        query += " WHERE package = %s"
    cur.execute(query, args)
    return cur.fetchall()


def iter_all_proposals(branch_name=None):
    cur = conn.cursor()
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
        query += " WHERE branch_name = %s"
        args.append(branch_name)
    cur.execute(query, args)
    row = cur.fetchone()
    while row:
        yield row
        row = cur.fetchone()


def iter_queue(limit=None):
    cur = conn.cursor()
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
    cur.execute(query)
    for row in cur.fetchall():
        (branch_url, maintainer_email, package, committer,
            command, context, queue_id) = row
        env = {
            'PACKAGE': package,
            'MAINTAINER_EMAIL': maintainer_email,
            'COMMITTER': committer or None,
            'CONTEXT': context,
        }
        yield (queue_id, branch_url, env, shlex.split(command))


def drop_queue_item(queue_id):
    cur = conn.cursor()
    cur.execute("DELETE FROM queue WHERE id = %s", (queue_id,))
    conn.commit()


def add_to_queue(vcs_url, env, command, priority=0,
                 estimated_duration=None):
    package = env['PACKAGE']
    maintainer_email = env.get('MAINTAINER_EMAIL')
    context = env.get('CONTEXT')
    committer = env.get('COMMITTER')
    cur = conn.cursor()
    _ensure_package(cur, package, vcs_url, maintainer_email)
    cur.execute(
        "INSERT INTO queue "
        "(branch_url, package, command, committer, priority, context, "
        "estimated_duration) "
        "VALUES (%s, %s, %s, %s, %s, %s, %s) "
        "ON CONFLICT (package, command) DO UPDATE SET "
        "context = EXCLUDED.context, priority = EXCLUDED.priority, "
        "estimated_duration = EXCLUDED.estimated_duration "
        "WHERE queue.priority <= EXCLUDED.priority", (
            vcs_url, package, ' '.join(command), committer,
            priority, context, estimated_duration))
    conn.commit()
    return True


def set_proposal_status(url, status):
    cur = conn.cursor()
    cur.execute("""
INSERT INTO merge_proposal (url, status) VALUES (%s, %s)
ON CONFLICT (url) DO UPDATE SET status = EXCLUDED.status
""", (url, status))
    conn.commit()


def queue_length(minimum_priority=None):
    cur = conn.cursor()
    args = []
    query = 'SELECT COUNT(*) FROM queue'
    if minimum_priority is not None:
        query += ' WHERE priority >= %s'
        args.append(minimum_priority)
    cur.execute(query, args)
    return cur.fetchone()[0]


def queue_duration(minimum_priority=None):
    cur = conn.cursor()
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
        query += ' AND priority >= %s'
        args.append(minimum_priority)
    cur.execute(query, args)
    ret = cur.fetchone()[0]
    if ret is None:
        return datetime.timedelta()
    return ret


def iter_published_packages(suite):
    cur = conn.cursor()
    cur.execute("""
select distinct package, build_version from run where build_distribution = %s
""", (suite, ))
    return cur.fetchall()


def iter_previous_runs(package, command):
    cur = conn.cursor()
    cur.execute("""
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
  package = %s AND command = %s
ORDER BY start_time DESC
""", (package, ' '.join(command)))
    return cur.fetchall()


def iter_last_successes(suite=None):
    cur = conn.cursor()
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
        query += " WHERE build_distribution = %s"
        args.append(suite)
    query += """
ORDER BY package, command, result_code = 'success' DESC, start_time DESC
"""
    cur.execute(query, args)
    return cur.fetchall()


def iter_last_runs(command=None):
    cur = conn.cursor()
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
        query += " WHERE command = %s"
        args.append(command)
    query += " ORDER BY package, command, start_time DESC"
    cur.execute(query, args)
    return cur.fetchall()


def iter_build_failures():
    cur = conn.cursor()
    cur.execute("""
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
    return cur.fetchall()


def update_run_result(log_id, code, description):
    cur = conn.cursor()
    cur.execute(
        'UPDATE run SET result_code = %s, description = %s WHERE id = %s',
        (code, description, log_id))
    conn.commit()


def already_published(package, branch_name, revision, mode):
    cur = conn.cursor()
    cur.execute("""\
SELECT * FROM publish
WHERE mode = %s AND revision = %s AND package = %s AND branch_name = %s
""", (mode, revision, package, branch_name))
    if cur.fetchone():
        return True
    return False


def iter_publish_ready():
    cur = conn.cursor()
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
    cur.execute(query, args)
    return cur.fetchall()


def iter_unscanned_branches(last_scanned_minimum):
    cur = conn.cursor()
    cur.execute("""
SELECT
  name,
  'master',
  branch_url,
  last_scanned
FROM package
LEFT JOIN branch ON package.branch_url = branch.url
WHERE
  last_scanned is null or now() - last_scanned > %s
""", (last_scanned_minimum, ))
    return cur.fetchall()


def iter_package_branches():
    cur = conn.cursor()
    cur.execute("""
SELECT
  name,
  branch_url,
  revision
FROM
  package
LEFT JOIN branch ON package.branch_url = branch.url
""")
    return cur.fetchall()


def update_branch_status(
        branch_url, last_scanned=None, status=None, revision=None,
        description=None):
    cur = conn.cursor()
    cur.execute("""\
INSERT INTO branch (url, status, revision, last_scanned, description)
VALUES (%s, %s, %s, %s, %s)
ON CONFLICT (url) DO UPDATE SET
  status = EXCLUDED.status,
  revision = EXCLUDED.revision,
  last_scanned = EXCLUDED.last_scanned,
  description = EXCLUDED.description
""", (branch_url, status, revision.decode('utf-8') if revision else None,
      last_scanned, description))
    conn.commit()
