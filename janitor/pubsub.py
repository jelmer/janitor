#!/usr/bin/python3
# Copyright (C) 2018 Jelmer Vernooij <jelmer@jelmer.uk>
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

import aiohttp
from aiohttp import web
import asyncio
import json


class Subscription(object):

    def __init__(self, topic):
        self.topic = topic
        self.queue = asyncio.Queue()
        if topic.last:
            self.queue.put_nowait(topic.last)

    def __enter__(self):
        self.topic.subscriptions.add(self.queue)
        return self.queue

    def __exit__(self, type, value, traceback):
        self.topic.subscriptions.remove(self.queue)


class Topic(object):

    def __init__(self, repeat_last=False):
        self.subscriptions = set()
        self.last = None
        self.repeat_last = repeat_last

    def publish(self, message):
        if self.repeat_last:
            self.last = message
        for queue in self.subscriptions:
            queue.put_nowait(message)


async def pubsub_handler(topic, request):
    ws = web.WebSocketResponse()
    await ws.prepare(request)

    with Subscription(topic) as queue:
        while True:
            msg = await queue.get()
            await ws.send_str(json.dumps(msg))

    return ws


async def pubsub_reader(session, url):
    ws = await session.ws_connect(url)
    while True:
        msg = await ws.receive()

        if msg.type == aiohttp.WSMsgType.text:
            yield msg.json()
        elif msg.type == aiohttp.WSMsgType.closed:
            break
        elif msg.type == aiohttp.WSMsgType.error:
            break
