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

from debian.changelog import Version
import os
import shlex
import psycopg2


conn = psycopg2.connect(
    database="janitor",
    user="janitor",
    port=5432,
    host="brangwain.vpn.jelmer.uk")
conn.set_client_encoding('UTF8')


def store_run(run_id, name, vcs_url, maintainer_email, start_time, finish_time,
              command, description, result_code, merge_proposal_url,
              build_version, build_distribution):
    """Store a run.

    :param run_id: Run id
    :param name: Package name
    :param vcs_url: Upstream branch URL
    :param maintainer_email: Maintainer email
    :param start_time: Start time
    :param finish_time: Finish time
    :param command: Command
    :param description: A human-readable description
    :param result_code: Result code (as constant string)
    :param merge_proposal_url: Optional merge proposal URL
    :param build_version: Version that was built
    :param build_distribution: Build distribution
    """
    cur = conn.cursor()
    cur.execute(
        "INSERT INTO package (name, branch_url, maintainer_email) "
        "VALUES (%s, %s, %s) ON CONFLICT DO NOTHING",
        (name, vcs_url, maintainer_email))
    if merge_proposal_url:
        cur.execute(
            "INSERT INTO merge_proposal (url, package, status) "
            "VALUES (%s, %s, 'open') ON CONFLICT DO NOTHING",
            (merge_proposal_url, name))
    else:
        merge_proposal_url = None
    cur.execute(
        "INSERT INTO run (id, command, description, result_code, start_time, "
        "finish_time, package, merge_proposal_url, build_version, "
        "build_distribution) "
        "VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s)", (
            run_id, ' '.join(command), description, result_code,
            start_time, finish_time, name, merge_proposal_url,
            str(build_version) if build_version else None, build_distribution))
    conn.commit()


def iter_packages():
    cur = conn.cursor()
    cur.execute("""
SELECT
  name,
  maintainer_email,
  branch_url
FROM
  package
ORDER BY name ASC
""")
    return cur.fetchall()


def iter_runs(package=None):
    """Iterate over runs.

    Args:
      package: package to restrict to
    Returns:
      iterator over (
        run_id, (start_time, finish_time), command, description,
        package_name, merge_proposal_url, build_version, build_distribution,
        result_code)
    """
    cur = conn.cursor()
    query = """
SELECT
    run.id, command, start_time, finish_time, description, package.name,
    run.merge_proposal_url, build_version, build_distribution, result_code
FROM
    run
LEFT JOIN package ON package.name = run.package
"""
    args = ()
    if package is not None:
        query += " WHERE package.name = %s "
        args += (package,)
    query += "ORDER BY start_time DESC"
    cur.execute(query, args)
    row = cur.fetchone()
    while row:
        yield (row[0],
               (row[2], row[3]),
               row[1], row[4], row[5], row[6],
               Version(row[7]) if row[7] else None, row[8],
               row[9] if row[9] else None)
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


def iter_proposals(package):
    cur = conn.cursor()
    cur.execute(
        """
SELECT
    url
FROM
    merge_proposal
LEFT JOIN package ON merge_proposal.package = package.name
WHERE
    package.name =%s
""",
        (package, ))
    return cur.fetchall()


def iter_all_proposals():
    cur = conn.cursor()
    cur.execute("""
SELECT
    merge_proposal.url, merge_proposal.status, package.name
FROM
    merge_proposal
LEFT JOIN package ON merge_proposal.package = package.name
""")
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
    queue.mode,
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
            command, mode, queue_id) = row
        env = {
            'PACKAGE': package,
            'MAINTAINER_EMAIL': maintainer_email,
            'COMMITTER': committer or None,
        }
        yield (queue_id, branch_url, mode, env, shlex.split(command))


def drop_queue_item(queue_id):
    cur = conn.cursor()
    cur.execute("DELETE FROM queue WHERE id = %s", (queue_id,))
    conn.commit()


def add_to_queue(vcs_url, mode, env, command, priority=None):
    assert env['PACKAGE']
    cur = conn.cursor()
    cur.execute(
        "insert INTO package (name, branch_url, maintainer_email) "
        "VALUES (%s, %s, %s) ON CONFLICT DO NOTHING",
        (env['PACKAGE'], vcs_url, env['MAINTAINER_EMAIL']))
    try:
        cur.execute(
            "INSERT INTO queue (package, command, committer, mode, priority) "
            "VALUES (%s, %s, %s, %s, %s)", (
                env['PACKAGE'], ' '.join(command), env['COMMITTER'], mode,
                priority))
    except sqlite3.IntegrityError:
        # No need to add it to the queue multiple times
        return False
    conn.commit()
    return True


def set_proposal_status(url, status):
    cur = conn.cursor()
    cur.execute("""
INSERT INTO merge_proposal (url, status) VALUES (%s, %s)
ON CONFLICT (url) DO UPDATE SET status = %s
""", (url, status, status))
    conn.commit()


def queue_length():
    cur = conn.cursor()
    cur.execute('SELECT COUNT(*) FROM queue')
    return cur.fetchone()[0]


def iter_published_packages(suite):
    cur = conn.cursor()
    cur.execute("""
select distinct package, build_version from run where build_distribution = %s
""", (suite, ))
    return cur.fetchall()
