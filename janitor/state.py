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

import os
import sqlite3

con = sqlite3.connect(
    os.path.join(os.path.dirname(__file__), '..', 'state.db'))


def store_run(run_id, name, vcs_url, maintainer_email, start_time, finish_time,
              command, description, merge_proposal_url):
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
    """
    cur = con.cursor()
    cur.execute(
        "REPLACE INTO package (name, branch_url, maintainer_email) "
        "VALUES (?, ?, ?)",
        (name, vcs_url, maintainer_email))
    cur.execute('SELECT id FROM package WHERE name = ?', (name, ))
    package_id = cur.fetchrow()[0]
    if merge_proposal_url:
        cur.execute(
            "REPLACE INTO merge_proposal (url, package_id) VALUES (?, ?)",
            (merge_proposal_url, package_id))
        cur.execute('SELECT id FROM merge_proposal WHERE url = ?', (url, ))
        merge_proposal_id = cur.fetchrow()[0]
    else:
        merge_proposal_id = None
    cur.execute(
        "INSERT INTO run (id, command, description, start_time, finish_time, "
        "package_id, merge_proposal_id) "
        "VALUES (?, ?, ?, ?, ?, ?, ?)", (
            run_id, ' '.join(command), description, start_time, finish_time,
            package_id, merge_proposal_id, ))
    con.commit()


def iter_packages():
    cur = con.cursor()
    cur.execute("""
SELECT
  id,
  name,
  maintainer_email,
  branch_url
FROM
  package
ORDER BY name ASC
""")
    return cur.fetchall()


def iter_runs(package=None):
    cur = con.cursor()
    query = """
SELECT
    run.id, command, start_time, finish_time, description, package.name,
    merge_proposal.url
FROM
    run
left JOIN package ON package.id = run.package_id
LEFT JOIN merge_proposal ON merge_proposal.id = run.merge_proposal_id
"""
    args = ()
    if package is not None:
        query += " WHERE package.name = ? "
        args += (package,)
    query += "ORDER BY start_time DESC"
    cur.execute(query, args)
    row = cur.fetchone()
    while row:
        yield (row[0], (row[2], row[3]), row[1], row[4], row[5], row[6])
        row = cur.fetchone()


def get_maintainer_email(vcs_url):
    cur = con.cursor()
    cur.execute(
        """
SELECT
    maintainer_email
FROM
    package
LEFT JOIN merge_proposal ON merge_proposal.package_id = package.id
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
LEFT JOIN package ON merge_proposal.package_id = package.id
WHERE
    package.name = ?
""",
        (package, ))
    return cur.fetchall()
