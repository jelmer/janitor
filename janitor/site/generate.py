#!/usr/bin/python3

import os
import argparse

from janitor.site import env

simple_render = {
    'index.html': 'index.html',
    'contact/index.html': 'contact.html',
    'credentials/index.html': 'credentials.html',
    'apt/index.html': 'apt.html',
    'cupboard/index.html': 'cupboard.html',
    }


async def render_simple(src, dest, directory):
    template = env.get_template(src)
    os.makedirs(
        os.path.join(directory, os.path.dirname(dest)), exist_ok=True)
    with open(os.path.join(directory, dest), 'w') as f:
        f.write(await template.render_async())


async def render_lintian_fixes(directory):
    template = env.get_template('lintian-fixes-start.html')
    lintian_fixes_dir = os.path.join(directory, 'lintian-fixes')
    os.makedirs(lintian_fixes_dir, exist_ok=True)
    with open(os.path.join(lintian_fixes_dir, 'index.html'), 'w') as f:
        import lintian_brush
        f.write(await template.render_async({'lintian_brush': lintian_brush}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(prog='generate')
    parser.add_argument("directory")
    args = parser.parse_args()
    for dest, src in simple_render.items():
        loop.run_until_complete(render_simple(src, dest, args.directory))
    loop.run_until_complete(render_lintian_fixes(args.directory))
