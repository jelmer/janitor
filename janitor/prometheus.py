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

from aiohttp import web
import time

from prometheus_client import (
    Counter,
    Gauge,
    Histogram,
    generate_latest,
    CONTENT_TYPE_LATEST,
    )

request_counter = Counter(
    'requests_total', 'Total Request Count', ['method', 'route', 'status'])

request_latency_hist = Histogram(
    'request_latency_seconds', 'Request latency', ['route'])

requests_in_progress_gauge = Gauge(
    'requests_in_progress_total', 'Requests currently in progress',
    ['method', 'route'])


async def metrics(request):
    resp = web.Response(body=generate_latest())
    resp.content_type = CONTENT_TYPE_LATEST
    return resp


@web.middleware
async def metrics_middleware(request, handler):
    start_time = time.time()
    route = request.match_info.route.name
    requests_in_progress_gauge.labels(request.method, route).inc()
    try:
        response = await handler(request)
    except Exception as e:
        if not isinstance(e, web.HTTPError):
            import traceback
            traceback.print_exc()
        raise
    resp_time = time.time() - start_time
    request_latency_hist.labels(route).observe(resp_time)
    requests_in_progress_gauge.labels(request.method, route).dec()
    request_counter.labels(request.method, route, response.status).inc()
    return response


def setup_metrics(app):
    app.middlewares.insert(0, metrics_middleware)
    app.router.add_get("/metrics", metrics, name='metrics')


async def run_prometheus_server(listen_addr, port):
    """Run a web server with metrics only."""
    app = web.Application()
    setup_metrics(app)
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()
