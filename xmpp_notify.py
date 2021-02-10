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
import logging
import slixmpp

from prometheus_client import Counter

from janitor.trace import note
from janitor.pubsub import pubsub_reader
from janitor.prometheus import run_prometheus_server

import re
import sys
from urllib.parse import urljoin

xmpp_messages_sent = Counter("xmpp_messages_sent", "Number of messages sent to XMPP")


class JanitorNotifier(slixmpp.ClientXMPP):
    def __init__(self, jid, password, channel, publisher_url):
        slixmpp.ClientXMPP.__init__(self, jid, password)
        self.channel = channel
        self.nick = "janitor"
        self.publisher_url = publisher_url
        self.auto_authorize = True
        self._runner_status = None
        self.add_event_handler("session_start", self.on_start)
        self.add_event_handler("disconnected", self.on_lost)
        self.add_event_handler("message", self.on_message)
        self.add_event_handler("groupchat_message", self.on_muc_message)
        self.register_plugin("xep_0030")  # Service Discovery
        self.register_plugin("xep_0004")  # Data Forms
        self.register_plugin("xep_0060")  # PubSub
        self.register_plugin("xep_0199")  # XMPP Ping
        self.register_plugin("xep_0045")  # MUC

    async def on_start(self, event):
        self.send_presence(ptype="available", pstatus="Active")
        await self.get_roster()
        self.plugin["xep_0045"].join_muc(self.channel, self.nick, wait=True)
        await self.send_message(mto=args.channel, mtype="groupchat", mbody="Hello")

    async def on_lost(self, event):
        note("Connection lost, exiting.")
        sys.exit(1)

    async def send_message(self, *args, **kwargs):
        xmpp_messages_sent.inc()
        return super(JanitorNotifier, self).send_message(*args, **kwargs)

    async def set_runner_status(self, status):
        self._runner_status = status

    async def notify_merged(self, url, package, merged_by=None):
        await self.send_message(
            mto=self.channel,
            mtype="chat",
            mbody="Merge proposal %s (%s) merged%s."
            % (url, package, ((" by %s" % merged_by) if merged_by else "")),
        )

    async def notify_pushed(self, url, package, suite, result):
        msg = "Pushed %s changes to %s (%s)" % (suite, url, package)
        if suite == "lintian-fixes":
            tags = set()
            for entry in result["applied"]:
                tags.update(entry["fixed_lintian_tags"])
            if tags:
                msg += ", fixing: %s." % (", ".join(tags))
        await self.send_message(mto=self.channel, mtype="groupchat", mbody=msg)

    async def handle_message(self, message, reply):
        m = re.match("reschedule (.*)", message)
        if m:
            await reply("Rescheduling %s" % m.group(1))
            return
        if message == "status":
            if self._runner_status:
                status_strs = [
                    "%s (%s) since %s"
                    % (item["package"], item["suite"], item["start_time"])
                    for item in self._runner_status["processing"]
                ]
                await reply("Currently processing: " + ", ".join(status_strs) + ".")
            else:
                await reply("Current runner status unknown.")
        if message == "scan":
            url = urljoin(self.publisher_url, "scan")
            async with ClientSession() as session, session.post(url) as resp:
                if resp.status in (200, 202):
                    await reply("Merge proposal scan started.")
                else:
                    await reply("Merge proposal scan failed: %d." % resp.status)

    async def on_muc_message(self, msg):
        if msg["type"] == "groupchat":
            message = msg["body"]
            if not message.startswith(self.nick + ": "):
                return

            async def reply(m):
                await self.send_message(
                    mto=msg["from"].bare, mtype="groupchat", mbody=m
                )

            await self.handle_message(message[len(self.nick + ": ") :], reply)

    async def on_message(self, msg):
        if msg["type"] in ("chat", "normal"):
            message = msg["body"]

            async def reply(m):
                await self.send_message(mto=msg["from"].bare, mtype="chat", mbody=m)

            await self.handle_message(message, reply)


async def main(args):
    notifier = JanitorNotifier(
        args.jid, args.password, channel=args.channel, publisher_url=args.publisher_url
    )
    await run_prometheus_server(args.prometheus_listen_address, args.prometheus_port)
    notifier.connect()
    async with ClientSession() as session:
        async for msg in pubsub_reader(session, args.notifications_url):
            if msg[0] == "merge-proposal" and msg[1]["status"] == "merged":
                await notifier.notify_merged(
                    msg[1]["url"], msg[1].get("package"), msg[1].get("merged_by")
                )
            if msg[0] == "queue":
                await notifier.set_runner_status(msg[1])
            if (
                msg[0] == "publish"
                and msg[1]["mode"] == "push"
                and msg[1]["result_code"] == "success"
            ):
                url = msg[1]["main_branch_browse_url"] or msg[1]["main_branch_url"]
                await notifier.notify_pushed(
                    url, msg[1]["package"], msg[1]["suite"], msg[1]["result"]
                )


if __name__ == "__main__":
    import argparse
    import asyncio

    parser = argparse.ArgumentParser()
    parser.add_argument("--jid", help="Jabber ID", default="janitor@jelmer.uk")
    parser.add_argument("--password", help="Password", default=None)
    parser.add_argument(
        "--publisher-url", help="Publisher URL", default="http://localhost:9912/"
    )
    parser.add_argument(
        "--notifications-url",
        help="URL to retrieve notifications from",
        default="wss://janitor.debian.net/ws/notifications",
    )
    parser.add_argument(
        "--prometheus-listen-address",
        type=str,
        default="localhost",
        help="Host to provide prometheus metrics on.",
    )
    parser.add_argument(
        "--prometheus-port", type=int, default=9918, help="Port for prometheus metrics"
    )
    parser.add_argument("--channel", type=str, help="Channel", default=None)
    parser.add_argument(
        "-d",
        "--debug",
        help="set logging to DEBUG",
        action="store_const",
        dest="loglevel",
        const=logging.DEBUG,
        default=logging.INFO,
    )
    args = parser.parse_args()

    logging.basicConfig(level=args.loglevel, format="%(levelname)-8s %(message)s")

    asyncio.run(main(args))
