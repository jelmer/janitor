#!/usr/bin/python3

from .common import iter_candidates


SUITE = "orphan"


async def generate_candidates(db):
    candidates = []
    async with db.acquire() as conn:
        for row in await iter_candidates(conn, suite=SUITE):
            candidates.append((row['package'], row['context'], row['value']))
        candidates.sort(key=lambda x: x[2], reverse=True)
    return {"candidates": candidates}
