#!/usr/bin/python3

from aiohttp import web, ClientConnectorError

from ..config import get_campaign_config
from .common import html_template, render_template_for_request
from . import env


async def handle_apt_repo(target_release, request):
    suite = target_release + "-backports"
    from .apt_repo import get_published_packages

    async with request.app.database.acquire() as conn:
        vs = {
            "packages": await get_published_packages(conn, suite),
            "suite": suite,
            "target_release",
            "campaign_config": get_campaign_config(request.app['config'], suite),
        }
        text = await render_template_for_request(env, suite + ".html", request, vs)
        return web.Response(
            content_type="text/html",
            text=text,
            headers={"Cache-Control": "max-age=60"},
        )


def register_backports_endpoints(router, target_release):
    router.add_get(
        "/%s-backports/" % target_release,
        lambda r: handle_apt_repo(target_release, r),
        name="%s-backports-start" % target_release
    )
