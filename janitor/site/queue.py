#!/usr/bin/python3
# Copyright (C) 2019 Jelmer Vernooij <jelmer@jelmer.uk>
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

from datetime import datetime, timedelta

from typing import AsyncIterator, Tuple, Optional, Dict, Any, Iterator

from janitor import state


def lintian_tag_link(tag: str) -> str:
    return '<a href="https://lintian.debian.org/tags/%s.html">%s</a>' % (
        tag, tag)


class RunnerProcessingUnavailable(Exception):
    """Raised when unable to get processing data for runner."""


def get_processing(answer: Dict[str, Any]) -> Iterator[Dict[str, Any]]:
    for entry in answer['processing']:
        entry = dict(entry.items())
        if entry.get('estimated_duration'):
            entry['estimated_duration'] = timedelta(
                seconds=entry['estimated_duration'])
        if entry.get('start_time'):
            entry['start_time'] = datetime.fromisoformat(entry['start_time'])
            entry['current_duration'] = datetime.now() - entry['start_time']
        yield entry


async def iter_queue_with_last_run(
        db: state.Database,
        limit: Optional[int] = None
        ) -> AsyncIterator[
                Tuple[state.QueueItem, Optional[str], Optional[str]]]:
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
      upstream.upstream_branch_url,
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
  LEFT OUTER JOIN upstream ON package.name = upstream.name
  ORDER BY
  queue.priority ASC,
  queue.id ASC
"""
    if limit:
        query += " LIMIT %d" % limit
    async with db.acquire() as conn:
        for row in await conn.fetch(query):
            yield (
                state.QueueItem.from_row(row[:-2]),
                row[-2], row[-1])


async def get_queue(
        db: state.Database,
        only_command: Optional[str] = None,
        limit: Optional[int] = None) -> AsyncIterator[
            Tuple[int, str, Optional[str], str, str,
                  Optional[timedelta], Optional[str], Optional[str]]]:
    async for entry, log_id, result_code in (
            iter_queue_with_last_run(db, limit=limit)):
        if only_command is not None and entry.command != only_command:
            continue
        expecting = None
        if entry.command[0] == 'new-upstream':
            if '--snapshot' in entry.command:
                description = 'New upstream snapshot'
            else:
                description = 'New upstream'
                if entry.context:
                    expecting = (
                        'expecting to merge <a href=\'https://qa.debian.org'
                        '/cgi-bin/watch?pkg=%s\'>%s</a>' % (
                            entry.package, entry.context))
        elif entry.command[0] == 'lintian-brush':
            description = 'Lintian fixes'
            if entry.context:
                expecting = (
                    'expecting to fix: ' +
                    ', '.join(
                        map(lintian_tag_link, entry.context.split(' '))))
        elif entry.command[0] == 'just-build':
            description = 'Build without changes'
        elif entry.command[0] == 'apply-multiarch-hints':
            description = 'Apply multi-arch hints'
        elif entry.command[0] == 'orphan':
            description = 'Mark as orphaned'
        elif entry.command[0] == 'import-upload':
            description = 'Import archive changes missing from VCS'
        else:
            raise AssertionError('invalid command %s' % entry.command)
        if only_command is not None:
            description = expecting or ''
        elif expecting is not None:
            description += ", " + expecting
        if entry.refresh:
            description += " (from scratch)"
        yield (
            entry.id, entry.package, entry.requestor,
            entry.suite, description, entry.estimated_duration,
            log_id, result_code)


async def write_queue(client, db: state.Database,
                      only_command=None, limit=None,
                      is_admin=False,
                      queue_status=None):
    if queue_status:
        processing = get_processing(queue_status)
        active_queue_ids = set(
            [p['queue_id'] for p in queue_status['processing']])
    else:
        processing = iter([])
        active_queue_ids = set()
    return {
        'queue': get_queue(db, only_command, limit),
        'active_queue_ids': active_queue_ids,
        'processing': processing,
        }
