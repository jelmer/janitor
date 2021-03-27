#!/usr/bin/python3

from ..debian import state as debian_state
from .common import iter_candidates


SUITE = "orphan"


async def generate_candidates(db):
    candidates = []
    async with db.acquire() as conn:
        for (
            package,
            unused_suite,
            context,
            value,
            success_chance,
        ) in await iter_candidates(conn, suite=SUITE):
            candidates.append((package.name, context, value))
        candidates.sort(key=lambda x: x[2], reverse=True)
    return {"candidates": candidates}
