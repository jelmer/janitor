#!/usr/bin/python3


async def generate_review(db):
    async with db.acquire() as conn:
        pass
    kwargs = {}
    template = env.get_template('review.html')
    return await template.render_async(**kwargs)
