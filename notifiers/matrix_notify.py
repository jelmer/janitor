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

import logging

from aiohttp import web

from janitor_client import JanitorClient
from aiohttp_openmetrics import setup_metrics

import asyncio
from nio import AsyncClient


class JanitorNotifier(object):
    def __init__(self, matrix_client, matrix_room, **kwargs):
        self._matrix_client = matrix_client
        self._matrix_room = matrix_room
        super(JanitorNotifier, self).__init__(**kwargs)
        self._runner_status = None

    async def message(self, message):
        await self._matrix_client.room_send(
            room_id=self._matrix_room,
            message_type="m.room.message",
            content={
                "msgtype": "m.text",
                "body": message
            }
        )

    async def set_runner_status(self, status):
        self._runner_status = status

    async def notify_merged(self, url, package, campaign, merged_by=None):
        await self.message(
            "Merge proposal %s (%s/%s) merged%s."
            % (url, package, campaign,
               ((" by %s" % merged_by) if merged_by else "")),
        )

    async def notify_pushed(self, url, package, campaign, result):
        msg = "Pushed %s changes to %s (%s)" % (campaign, url, package)
        if campaign == "lintian-fixes":
            tags = set()
            for entry in result["applied"]:
                tags.update(entry["fixed_lintian_tags"])
            if tags:
                msg += ", fixing: %s." % (", ".join(tags))
        await self.message(msg)


async def main(args):
    if args.gcp_logging:
        import google.cloud.logging
        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    else:
        logging.basicConfig(level=logging.INFO)

    matrix_client = AsyncClient(args.homeserver_url, args.user)
    await matrix_client.login(args.password)
    notifier = JanitorNotifier(
        matrix_client=matrix_client, matrix_room=args.room)
    app = web.Application()
    setup_metrics(app)
    app.router.add_get(
        '/health', lambda req: web.Response(text='ok', status=200))
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(
        runner, args.prometheus_listen_address, args.prometheus_port)
    await site.start()

    async with JanitorClient(args.janitor_url) as janitor_client:
        async for msg in janitor_client._iter_notifications():
            if msg[0] == "merge-proposal" and msg[1]["status"] == "merged":
                await notifier.notify_merged(
                    msg[1]["url"], msg[1].get("package"),
                    msg[1].get("campaign"),
                    msg[1].get("merged_by")
                )
            if msg[0] == "queue":
                await notifier.set_runner_status(msg[1])
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
    import os

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--publisher-url", help="Publisher URL",
        default="http://localhost:9912/"
    )
    parser.add_argument(
        "--password", help="Matrix password", type=str,
        default=os.environ.get('MATRIX_PASSWORD'))
    parser.add_argument(
        "--homeserver-url", type=str,
        help="Matrix homeserver URL")
    parser.add_argument(
        "--user", type=str,
        help="Matrix user string")
    parser.add_argument(
        "--janitor-url",
        help="Janitor instance URL",
        default="https://janitor.debian.net/",
    )
    parser.add_argument(
        "--room", type=str,
        help="Matrix room to send notifications to")
    parser.add_argument(
        "--prometheus-listen-address",
        type=str,
        default="localhost",
        help="Host to provide prometheus metrics on.",
    )
    parser.add_argument(
        "--prometheus-port", type=int, default=9918,
        help="Port for prometheus metrics"
    )
    parser.add_argument(
        "--gcp-logging", action='store_true', help='Use Google cloud logging.')
    args = parser.parse_args()

    asyncio.run(main(args))
