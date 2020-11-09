#!/usr/bin/python3

from . import state


SUITE = 'orphan'


async def generate_candidates(db):
    candidates = []
    async with db.acquire() as conn:
        for (package, suite, context, value,
             success_chance) in await state.iter_candidates(conn, suite=SUITE):
            candidates.append((package.name, context, value))
        candidates.sort(key=lambda x: x[2], reverse=True)
    return {'candidates': candidates}
