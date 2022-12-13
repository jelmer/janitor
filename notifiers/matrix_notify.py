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
from mautrix.client import Client as MatrixClient


async def main(args):
    if args.gcp_logging:
        import google.cloud.logging
        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    else:
        logging.basicConfig(level=logging.INFO)

    matrix_client = MatrixClient(base_url=args.homeserver_url)
    logging.info('%s', await matrix_client.login(args.user, password=args.password))
    await matrix_client.join_room(args.room)

    async def message(msg, formatted_msg):
        try:
            await matrix_client.room_send(
                room_id=args.room,
                message_type="m.room.message",
                content={
                    "msgtype": "m.text",
                    "formatted_body": formatted_msg,
                    "format": "org.matrix.custom.html",
                    "body": msg
                }
            )
        except LocalProtocolError as e:
            logging.warning(
                'Error sending matrix message: %r',
                e)

    app = web.Application()
    setup_metrics(app)
    app.router.add_get(
        '/health', lambda req: web.Response(text='ok', status=200))
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(
        runner, args.prometheus_listen_address, args.prometheus_port)
    await site.start()

    await matrix_client.sync()

    async with JanitorClient(args.janitor_url) as janitor_client:
        async for msg in janitor_client._iter_notifications():
            k = msg[0]
            d = msg[1]
            if k == "merge-proposal" and d["status"] == "merged":
                out = "Merge proposal %s (%s/%s) merged%s." % (
                    d["url"], d.get("package"), d.get("campaign"),
                    ((" by %s" % d["merged_by"]) if d.get("merged_by") else ""))
                outhtml = f"<a href=\"{d['url']}\">Merge proposal</a> for <a href=\"{args.janitor_url}/{d['campaign']}\">{d['campaign']}</a>/{d.get('package')} merged%s." % ((" by %s" % d["merged_by"]) if d.get("merged_by") else "")
                outhtml += f" #{d['campaign']}"
                await message(out, outhtml)
            if (
                k == "publish"
                and d["mode"] == "push"
                and d["result_code"] == "success"
            ):
                url = (d["main_branch_browse_url"]
                       or d["main_branch_url"])
                out = "Pushed %s changes to %s (%s)" % (
                    d.get("campaign"), url, d["package"])
                outhtml = f"Pushed <a href=\"{args.janitor_url}/{d['campaign']}\">{d['campaign']}</a> changes to <a href=\"{url}\">{d['package']}</a>"
                if d.get("campaign") == "lintian-fixes":
                    tags = set()
                    for entry in d["result"]["applied"]:
                        tags.update(entry["fixed_lintian_tags"])
                    if tags:
                        out += ", fixing: %s." % (", ".join(tags))
                        outhtml += ", fixing: %s." % (", ".join([f"<a href=\"https://lintian.debian.org/tags/{tag}.html\">{tag}</a>" for tag in tags]))
                outhtml += f" #{d['campaign']}"
                await message(out, outhtml)


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
        "--homeserver-url", type=str, default=os.environ.get('HOMESERVER_URL'),
        help="Matrix homeserver URL")
    parser.add_argument(
        "--user", type=str, default=os.environ.get('MATRIX_USER'),
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
