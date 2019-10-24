#!/usr/bin/python3

import os

from janitor.site import env

simple_render = {
    'index.html': 'index.html',
    'contact/index.html': 'contact.html',
    'credentials/index.html': 'credentials.html',
    'apt/index.html': 'apt.html',
    'cupboard/index.html': 'cupboard.html',
    }


async def render_simple(src):
    template = env.get_template(src)
    return await template.render_async()


async def render_lintian_fixes():
    template = env.get_template('lintian-fixes-start.html')
    import lintian_brush
    from silver_platter.debian.lintian import DEFAULT_ADDON_FIXERS
    return await template.render_async(
        {'lintian_brush': lintian_brush,
        'ADDON_FIXERS': DEFAULT_ADDON_FIXERS})


if __name__ == '__main__':
    import argparse
    import asyncio
    parser = argparse.ArgumentParser(prog='generate')
    parser.add_argument("directory")
    args = parser.parse_args()
    loop = asyncio.get_event_loop()
    for dest, src in simple_render.items():
        os.makedirs(
            os.path.join(args.directory, os.path.dirname(dest)), exist_ok=True)
        with open(os.path.join(args.directory, dest), 'w') as f:
            f.write(loop.run_until_complete(render_simple(src)))
    lintian_fixes_dir = os.path.join(args.directory, 'lintian-fixes')
    os.makedirs(lintian_fixes_dir, exist_ok=True)
    with open(os.path.join(lintian_fixes_dir, 'index.html'), 'w') as f:
        f.write(loop.run_until_complete(render_lintian_fixes()))
