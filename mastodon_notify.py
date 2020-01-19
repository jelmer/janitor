#!/usr/bin/python3

from aiohttp.client import ClientSession
from janitor.pubsub import pubsub_reader

import sys

from mastodon import Mastodon


class MastodonNotifier(object):

    def __init__(self, mastodon):
        self.mastodon = mastodon

    async def notify_merged(self, url, package, merged_by=None):
        self.mastodon.toot(
            'Merge proposal %s (%s) merged%s.' %
            (url, package, ((' by %s' % merged_by) if merged_by else '')))

    async def notify_pushed(self, url, package, suite, result):
        msg = 'Pushed %s changes to %s (%s)' % (suite, url, package)
        if suite == 'lintian-fixes':
            tags = set()
            for entry in result['applied']:
                tags.update(entry['fixed_lintian_tags'])
            if tags:
                msg += ', fixing: %s.' % (', '.join(tags))
        self.mastodon.toot(msg)


async def main(args, mastodon):
    notifier = MastodonNotifier(mastodon)
    async with ClientSession() as session:
        async for msg in pubsub_reader(session, args.notifications_url):
            if msg[0] == 'merge-proposal' and msg[1]['status'] == 'merged':
                await notifier.notify_merged(
                    msg[1]['url'], msg[1].get('package'),
                    msg[1].get('merged_by'))
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
    parser.add_argument(
        '--publisher-url', help='Publisher URL',
        default='http://localhost:9912/')
    parser.add_argument(
        '--notifications-url', help='URL to retrieve notifications from',
        default='wss://janitor.debian.net/ws/notifications')
    parser.add_argument(
        '--register', help='Register the app',
        action='store_true')
    parser.add_argument(
        '--login', type=str,
        help='Login to the specified user (e-mail).')
    parser.add_argument(
        '--api-base-url', type=str,
        default='https://mastodon.cloud',
        help='Mastodon API Base URL.')
    args = parser.parse_args()
    if args.register:
        Mastodon.create_app(
            'debian-janitor-notify',
            api_base_url=args.api_base_url,
            to_file='mastodon-notify-app.secret')
        sys.exit(0)

    if args.login:
        mastodon = Mastodon(
            client_id='mastodon-notify-app.secret',
            api_base_url=args.api_base_url
        )

        import getpass
        password = getpass.getpass()

        mastodon.log_in(
            args.login, password, to_file='mastodon-notify-user.secret')

        sys.exit(0)

    mastodon = Mastodon(
        access_token='mastodon-notify-user.secret',
        api_base_url=args.api_base_url)

    asyncio.run(main(args, mastodon))
