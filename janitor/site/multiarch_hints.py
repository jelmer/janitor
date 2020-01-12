#!/usr/bin/python3

from .common import generate_pkg_context
from janitor.site import (
    env,
    )


SUITE = 'multiarch-fixes'


async def generate_pkg_file(db, policy, client, archiver_url, publisher_url,
                            package, run_id=None):
    kwargs = await generate_pkg_context(
        db, SUITE, policy, client, archiver_url, publisher_url, package,
        run_id=run_id)
    template = env.get_template('multiarch-fixes-package.html')
    return await template.render_async(**kwargs)


async def render_start():
    template = env.get_template('multiarch-fixes-start.html')
    return await template.render_async()
