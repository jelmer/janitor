#!/usr/bin/python3

from . import env
from .common import iter_candidates, html_template


SUITE = "orphan"


@html_template(env, "orphan/candidates.html", headers={"Cache-Control": "max-age=3600"})
async def handle_orphan_candidates(request):
    candidates = []
    async with request.app.database.acquire() as conn:
        for row in await iter_candidates(conn, suite=SUITE):
            candidates.append((row['package'], row['context'], row['value']))
        candidates.sort(key=lambda x: x[2], reverse=True)
    return {"candidates": candidates}


@html_template(env, "orphan/start.html")
async def handle_orphan_start(request):
    return {}


def register_orphan_endpoints(router):
    router.add_get("/orphan/", handle_orphan_start, name="orphan-start")
    router.add_get(
        "/orphan/candidates", handle_orphan_candidates, name="orphan-candidates"
    )
