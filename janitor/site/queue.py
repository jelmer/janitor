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

from aiohttp import ClientSession, ContentTypeError, ClientConnectorError
import argparse
import asyncio
import sys
import urllib
from datetime import datetime, timedelta

from janitor import state
from janitor.site import env


def lintian_tag_link(tag):
    return '<a href="https://lintian.debian.org/tags/%s.html">%s</a>' % (
        tag, tag)


class RunnerProcessingUnavailable(Exception):
    """Raised when unable to get processing data for runner."""


async def get_processing(runner_url):
    url = urllib.parse.urljoin(runner_url, 'status')
    async with ClientSession() as client:
        try:
            async with client.get(url) as resp:
                if resp.status != 200:
                    raise RunnerProcessingUnavailable(await resp.text())
                answer = await resp.json()
        except ContentTypeError as e:
            raise RunnerProcessingUnavailable(
                'publisher returned error %d' % e.code)
        except ClientConnectorError:
            raise RunnerProcessingUnavailable(
                'unable to contact publisher')
        else:
            for entry in answer['processing']:
                if entry.get('estimated_duration'):
                    entry['estimated_duration'] = timedelta(
                        seconds=entry['estimated_duration'])
                if entry.get('current_duration'):
                    entry['current_duration'] = timedelta(
                        seconds=entry['current_duration'])
                if entry.get('start_time'):
                    entry['start_time'] = datetime.fromisoformat(
                        entry['start_time'])
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
                if entry.env.get('CONTEXT'):
                    expecting = (
                        'expecting to merge <a href=\'https://qa.debian.org'
                        '/cgi-bin/watch?pkg=%s\'>%s</a>' % (
                            entry.package, entry.env['CONTEXT']))
        elif entry.command[0] == 'lintian-brush':
            description = 'Lintian fixes'
            if entry.env.get('CONTEXT'):
                expecting = (
                    'expecting to fix: ' +
                    ', '.join(
                        map(lintian_tag_link,
                            entry.env['CONTEXT'].split(' '))))
        elif entry.command[0] == 'just-build':
            description = 'Build without changes'
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


async def write_queue(only_command=None, limit=None, runner_url=None):
    template = env.get_template('queue.html')
    if runner_url:
        async def processing_():
            try:
                async for x in get_processing(runner_url):
                    yield x
            except RunnerProcessingUnavailable:
                pass
        processing = processing_()
    else:
        processing = []
    async with state.get_connection() as conn:
        return await template.render_async(
            queue=get_queue(conn, only_command, limit),
            processing=processing)


if __name__ == '__main__':
    parser = argparse.ArgumentParser('report-queue')
    parser.add_argument(
        '--command', type=str, help='Only display queue for specified command')
    parser.add_argument(
        '--limit', type=int, help='Limit to this number of entries',
        default=100)
    args = parser.parse_args()

    loop = asyncio.get_event_loop()
    sys.stdout.write(loop.run_until_complete(write_queue()))
