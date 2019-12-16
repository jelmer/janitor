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

from janitor import state
from janitor.site import env


def lintian_tag_link(tag):
    return '<a href="https://lintian.debian.org/tags/%s.html">%s</a>' % (
        tag, tag)


class RunnerProcessingUnavailable(Exception):
    """Raised when unable to get processing data for runner."""


def get_processing(answer):
    for entry in answer['processing']:
        entry = dict(entry.items())
        if entry.get('estimated_duration'):
            entry['estimated_duration'] = timedelta(
                seconds=entry['estimated_duration'])
        if entry.get('start_time'):
            entry['start_time'] = datetime.fromisoformat(entry['start_time'])
            entry['current_duration'] = datetime.now() - entry['start_time']
        yield entry


async def get_queue(conn, only_command=None, limit=None):
    async for entry, log_id, result_code in (
            state.iter_queue_with_last_run(conn, limit=limit)):
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
        else:
            raise AssertionError('invalid command %s' % entry.command)
        if only_command is not None:
            description = expecting
        elif expecting is not None:
            description += ", " + expecting
        if entry.refresh:
            description += " (from scratch)"
        yield (
            entry.package, entry.requestor,
            entry.suite, description, entry.estimated_duration,
            log_id, result_code)


async def write_queue(client, conn, only_command=None, limit=None,
                      queue_status=None):
    template = env.get_template('queue.html')
    if queue_status:
        processing = get_processing(queue_status)
    else:
        processing = []
    return await template.render_async(
        queue=get_queue(conn, only_command, limit),
        processing=processing)
