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

from typing import AsyncIterator, Any

import aiohttp
from aiohttp.client import ClientResponseError, ClientConnectorError
import asyncio

import logging

from typing import Optional

from aiohttp_openmetrics import Gauge

from yarl import URL


subscription_active = Gauge(
    "subscription_active", "Whether url is reachable", labelnames=("url",)
)


async def pubsub_reader(
    session: aiohttp.ClientSession, url: URL, reconnect_interval: Optional[int] = 10
) -> AsyncIterator[Any]:
    subscription_active.labels(url=url).set(0)
    while True:
        try:
            ws = await session.ws_connect(url)
        except (ClientResponseError, ClientConnectorError) as e:
            logging.warning("Unable to connect: %s" % e)
        else:
            subscription_active.labels(url=url).set(1)
            logging.info("Subscribed to %s", url)
            while True:
                msg = await ws.receive()

                if msg.type == aiohttp.WSMsgType.text:
                    yield msg.json()
                elif msg.type == aiohttp.WSMsgType.closed:
                    break
                elif msg.type == aiohttp.WSMsgType.error:
                    logging.warning("Error on websocket: %s", ws.exception())
                    break
                else:
                    logging.warning("Ignoring ws message type %r", msg.type)
        subscription_active.labels(url=url).set(0)
        if reconnect_interval is None:
            return
        logging.info("Waiting %d seconds before reconnecting...", reconnect_interval)
        await asyncio.sleep(reconnect_interval)


class JanitorClient(object):
    """Interface to the public API of the janitor."""

    def __init__(self, url):
        self.url = url
        self.session = None

    async def __aenter__(self):
        self.session = aiohttp.ClientSession()
        return await self.session.__aenter__()

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.session.__aexit__(exc_type, exc_val, exc_tb)
        return False

    async def _iter_notifications(self):
        notifications_url = URL(self.url).with_scheme('wss').join(['ws', 'notifications'])
        async for msg in pubsub_reader(self.session, notifications_url):
            yield msg
