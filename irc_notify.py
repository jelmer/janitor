#!/usr/bin/python3

# Copyright (C) 2019-2020 Jelmer Vernooij <jelmer@jelmer.uk>
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

from aiohttp.client import ClientSession
import pydle

from prometheus_client import Counter

from janitor.pubsub import pubsub_reader
from janitor.prometheus import run_prometheus_server

import re
from urllib.parse import urljoin

irc_messages_sent = Counter(
    'irc_messages_sent', 'Number of messages sent to IRC')


class JanitorNotifier(pydle.Client):

    def __init__(self, channel, **kwargs):
        self.publisher_url = kwargs.pop('publisher_url')
        super(JanitorNotifier, self).__init__(**kwargs)
        self._channel = channel
        self._runner_status = None

    def message(self, *args, **kwargs):
        irc_messages_sent.inc()
        return super(JanitorNotifier, self).message(*args, **kwargs)

    async def on_connect(self):
        await self.join(self._channel)

    async def set_runner_status(self, status):
        self._runner_status = status

    async def notify_merged(self, url, package, merged_by=None):
        await self.message(
            self._channel, 'Merge proposal %s (%s) merged%s.' %
            (url, package, ((' by %s' % merged_by) if merged_by else '')))

    async def notify_pushed(self, url, package, suite, result):
        msg = 'Pushed %s changes to %s (%s)' % (suite, url, package)
        if suite == 'lintian-fixes':
            tags = set()
            for entry in result['applied']:
                tags.update(entry['fixed_lintian_tags'])
            if tags:
                msg += ', fixing: %s.' % (', '.join(tags))
        await self.message(self._channel, msg)

    async def on_message(self, target, source, message):
        if not message.startswith(self.nickname + ': '):
            return
        message = message[len(self.nickname + ': '):]
        m = re.match('reschedule (.*)', message)
        if m:
            await self.message(target, 'Rescheduling %s' % m.group(1))
            return
        if message == 'status':
            if self._runner_status:
                status_strs = [
                    '%s (%s) since %s' % (
                        item['package'], item['suite'], item['start_time'])
                    for item in self._runner_status['processing']]
                await self.message(
                    target,
                    'Currently processing: ' + ', '.join(status_strs) + '.')
            else:
                await self.message(target, 'Current runner status unknown.')
        if message == 'scan':
            url = urljoin(self.publisher_url, 'scan')
            async with ClientSession() as session, session.post(url) as resp:
                if resp.status in (200, 202):
                    await self.message(target, 'Merge proposal scan started.')
                else:
                    await self.message(
                        target,
                        'Merge proposal scan failed: %d.' % resp.status)


async def main(args):
    notifier = JanitorNotifier(
        args.channel, nickname=args.nick, realname=args.fullname,
        publisher_url=args.publisher_url)
    loop = asyncio.get_event_loop()
    await run_prometheus_server(
        args.prometheus_listen_address, args.prometheus_port)
    asyncio.ensure_future(
        notifier.connect(args.server, tls=True, tls_verify=False), loop=loop)
    async with ClientSession() as session:
        async for msg in pubsub_reader(session, args.notifications_url):
            if msg[0] == 'merge-proposal' and msg[1]['status'] == 'merged':
                await notifier.notify_merged(
                    msg[1]['url'], msg[1].get('package'),
                    msg[1].get('merged_by'))
            if msg[0] == 'queue':
                await notifier.set_runner_status(msg[1])
            if (msg[0] == 'publish' and
                    msg[1]['mode'] == 'push' and
                    msg[1]['result_code'] == 'success'):
                url = (msg[1]['main_branch_browse_url'] or
                       msg[1]['main_branch_url'])
                await notifier.notify_pushed(
                    url, msg[1]['package'],
                    msg[1]['suite'], msg[1]['result'])


if __name__ == '__main__':
    import argparse
    import asyncio
    parser = argparse.ArgumentParser()
    parser.add_argument('--server', help='IRC server', default='irc.oftc.net')
    parser.add_argument('--nick', help='IRC nick', default='janitor-notify')
    parser.add_argument(
        '--channel', help='IRC channel', default='#debian-janitor')
    parser.add_argument(
        '--publisher-url', help='Publisher URL',
        default='http://localhost:9912/')
    parser.add_argument(
        '--notifications-url', help='URL to retrieve notifications from',
        default='wss://janitor.debian.net/ws/notifications')
    parser.add_argument(
        '--fullname', help='IRC fullname',
        default='Debian Janitor Notifier (https://janitor.debian.net/contact/')
    parser.add_argument(
        '--prometheus-listen-address', type=str,
        default='localhost', help='Host to provide prometheus metrics on.')
    parser.add_argument(
        '--prometheus-port', type=int,
        default=9918, help='Port for prometheus metrics')
    args = parser.parse_args()

    asyncio.run(main(args))
