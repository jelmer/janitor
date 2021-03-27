#!/usr/bin/python
# Copyright (C) 2018-2021 Jelmer Vernooij <jelmer@jelmer.uk>
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

from typing import Optional, Dict, List, Tuple, Any


import asyncpg
from breezy import urlutils
from debian.changelog import Version


async def version_available(
    conn: asyncpg.Connection,
    package: str,
    suite: str,
    version: Optional[Tuple[str, Version]] = None,
) -> List[Tuple[str, str, Version]]:
    query = """\
SELECT
  package,
  suite,
  debian_build.version
FROM
  run
LEFT JOIN debian_build ON run.id = debian_build.run_id
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
            "version_match1": "debian_build.version %s $3" % (version[0],),
            "version_match2": "archive_version %s $3" % (version[0],),
        }
        args.append(str(version[1]))
    else:
        query = query % {"version_match1": "True", "version_match2": "True"}
    return await conn.fetch(query, *args)
