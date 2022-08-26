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

from aiohttp import web

from aiohttp_openmetrics import setup_metrics, Counter

from janitor_client import JanitorClient

import logging
import sys

from mastodon import Mastodon


toots_posted = Counter("toots_posted", "Number of toots posted")


class MastodonNotifier(object):
    def __init__(self, mastodon):
        self.mastodon = mastodon

    def toot(self, msg):
        toots_posted.inc()
        self.mastodon.toot(msg)

    async def notify_merged(self, url, package, merged_by=None):
        self.toot(
            "Merge proposal %s (%s) merged%s."
            % (url, package, ((" by %s" % merged_by) if merged_by else ""))
        )

    async def notify_pushed(self, url, package, campaign, result):
        msg = "Pushed %s changes to %s (%s)" % (campaign, url, package)
        if campaign == "lintian-fixes":
            tags = set()
            for entry in result["applied"]:
                tags.update(entry["fixed_lintian_tags"])
            if tags:
                msg += ", fixing: %s." % (", ".join(tags))
        self.toot(msg)


async def main(args, mastodon):
    if args.gcp_logging:
        import google.cloud.logging
        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    else:
        logging.basicConfig(level=logging.INFO)

    app = web.Application()
    setup_metrics(app)
    app.router.add_get(
        '/health', lambda req: web.Response(text='ok', status=200))
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(
        runner, args.prometheus_listen_address, args.prometheus_port)
    await site.start()

    notifier = MastodonNotifier(mastodon)
    async with JanitorClient(args.janitor_url) as janitor_client:
        async for msg in janitor_client._iter_notifications():
            if msg[0] == "merge-proposal" and msg[1]["status"] == "merged":
                await notifier.notify_merged(
                    msg[1]["url"], msg[1].get("package"),
                    msg[1].get("merged_by")
                )
            if (
                msg[0] == "publish"
                and msg[1]["mode"] == "push"
                and msg[1]["result_code"] == "success"
            ):
                url = (msg[1]["main_branch_browse_url"]
                       or msg[1]["main_branch_url"])
                await notifier.notify_pushed(
                    url, msg[1]["package"], msg[1]["campaign"],
                    msg[1]["result"]
                )


if __name__ == "__main__":
    import argparse
    import asyncio

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--publisher-url", help="Publisher URL",
        default="http://localhost:9912/"
    )
    parser.add_argument(
        "--janitor-url",
        help="URL to janitor instance",
        default="https://janitor.debian.net/",
    )
    parser.add_argument(
        "--register", help="Register the app", action="store_true")
    parser.add_argument(
        "--login", type=str, help="Login to the specified user (e-mail)."
    )
    parser.add_argument(
        "--api-base-url",
        type=str,
        default="https://mastodon.cloud",
        help="Mastodon API Base URL.",
    )
    parser.add_argument(
        "--prometheus-listen-address",
        type=str,
        default="localhost",
        help="Host to provide prometheus metrics on.",
    )
    parser.add_argument(
        "--prometheus-port", type=int, default=9919,
        help="Port for prometheus metrics"
    )
    parser.add_argument(
        "--user-secret-path", type=str, default="mastodon-notify-user.secret",
        help="Path to user secret.")
    parser.add_argument(
        "--app-secret-path", type=str, default="mastodon-notify-app.secret",
        help="Path to app secret.")
    parser.add_argument(
        "--gcp-logging", action='store_true', help='Use Google cloud logging.')

    args = parser.parse_args()
    if args.register:
        Mastodon.create_app(
            "debian-janitor-notify",
            api_base_url=args.api_base_url,
            to_file=args.app_secret_path
        )
        sys.exit(0)

    if args.login:
        mastodon = Mastodon(
            client_id=args.app_secret_path, api_base_url=args.api_base_url)

        import getpass

        password = getpass.getpass()

        mastodon.log_in(args.login, password, to_file=args.user_secret_path)

        sys.exit(0)

    mastodon = Mastodon(
        access_token=args.user_secret_path, api_base_url=args.api_base_url
    )

    asyncio.run(main(args, mastodon))
