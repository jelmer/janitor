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
from datetime import datetime
import os
import shlex
import sqlite3

con = sqlite3.connect(
    os.path.join(os.path.dirname(__file__), '..', 'state.db'))


def store_run(run_id, name, vcs_url, maintainer_email, start_time, finish_time,
              command, description, merge_proposal_url, build_version,
              build_distribution):
    """Store a run.

    :param run_id: Run id
    :param name: Package name
    :param vcs_url: Upstream branch URL
    :param maintainer_email: Maintainer email
    :param start_time: Start time
    :param finish_time: Finish time
    :param command: Command
    :param description: A human-readable description
    :param merge_proposal_url: Optional merge proposal URL
    :param build_version: Version that was built
    :param build_distribution: Build distribution
    """
    cur = con.cursor()
    cur.execute(
        "insert or ignore INTO package (name, branch_url, maintainer_email) "
        "VALUES (?, ?, ?)",
        (name, vcs_url, maintainer_email))
    if merge_proposal_url:
        cur.execute(
            "INSERT OR IGNORE INTO merge_proposal (url, package, status) "
            "VALUES (?, ?, 'open')",
            (merge_proposal_url, name))
    else:
        merge_proposal_url = None
    cur.execute(
        "INSERT INTO run (id, command, description, start_time, finish_time, "
        "package, merge_proposal_url, build_version, build_distribution) "
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)", (
            run_id, ' '.join(command), description,
            start_time.isoformat(), finish_time.isoformat(),
            name, merge_proposal_url,
            str(build_version) if build_version else None, build_distribution))
    con.commit()


def iter_packages():
    cur = con.cursor()
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
        package_name, merge_proposal_url, build_version, build_distribution)
    """
    cur = con.cursor()
    query = """
SELECT
    run.id, command, start_time, finish_time, description, package.name,
    run.merge_proposal_url, build_version, build_distribution
FROM
    run
LEFT JOIN package ON package.name = run.package
"""
    args = ()
    if package is not None:
        query += " WHERE package.name = ? "
        args += (package,)
    query += "ORDER BY start_time DESC"
    cur.execute(query, args)
    row = cur.fetchone()
    while row:
        yield (row[0],
               (datetime.fromisoformat(row[2]),
                datetime.fromisoformat(row[3])),
               row[1], row[4], row[5], row[6],
               Version(row[7]) if row[7] else None, row[8])
        row = cur.fetchone()


def get_maintainer_email(vcs_url):
    cur = con.cursor()
    cur.execute(
        """
SELECT
    maintainer_email
FROM
    package
LEFT JOIN merge_proposal ON merge_proposal.package = package.name
WHERE
    merge_proposal.url = ?""",
        (vcs_url, ))
    row = cur.fetchone()
    if row:
        return row[0]
    return None


def iter_proposals(package):
    cur = con.cursor()
    cur.execute(
        """
SELECT
    url
FROM
    merge_proposal
LEFT JOIN package ON merge_proposal.package = package.name
WHERE
    package.name = ?
""",
        (package, ))
    return cur.fetchall()


def iter_all_proposals():
    cur = con.cursor()
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
    cur = con.cursor()
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
    cur = con.cursor()
    cur.execute("DELETE FROM queue WHERE id = ?", (queue_id,))
    con.commit()


def add_to_queue(vcs_url, mode, env, command, priority=None):
    assert env['PACKAGE']
    cur = con.cursor()
    cur.execute(
        "insert or ignore INTO package (name, branch_url, maintainer_email) "
        "VALUES (?, ?, ?)",
        (env['PACKAGE'], vcs_url, env['MAINTAINER_EMAIL']))
    try:
        cur.execute(
            "INSERT INTO queue (package, command, committer, mode, priority) "
            "VALUES (?, ?, ?, ?, ?)", (
                env['PACKAGE'], ' '.join(command), env['COMMITTER'], mode,
                priority))
    except sqlite3.IntegrityError:
        # No need to add it to the queue multiple times
        return False
    con.commit()
    return True


def set_proposal_status(url, status):
    cur = con.cursor()
    cur.execute("""
INSERT OR IGNORE INTO merge_proposal (url, status) VALUES (?, ?)
""", (url, status))
    cur.execute("""
UPDATE
    merge_proposal
SET
    status = ?
WHERE url = ?
""", (status, url))
    con.commit()


def queue_length():
    cur = con.cursor()
    cur.execute('SELECT COUNT(*) FROM queue')
    return cur.fetchone()[0]


def iter_published_packages(suite):
    cur = con.cursor()
    cur.execute("""
select distinct package, build_version from run where build_distribution = ?
""", (suite, ))
    return cur.fetchall()
