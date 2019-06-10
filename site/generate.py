#!/usr/bin/python3

import os

import argparse
from jinja2 import Environment, FileSystemLoader, select_autoescape

env = Environment(
    loader=FileSystemLoader('templates'),
    autoescape=select_autoescape(['html', 'xml'])
)

parser = argparse.ArgumentParser(prog='generate')
parser.add_argument("directory")
args = parser.parse_args()


simple_render = {
    'index.html': 'index.html',
    'contact/index.html': 'contact.html',
    'credentials/index.html': 'credentials.html',
    'apt/index.html': 'apt.html',
    'cupboard/index.html': 'cupboard.html',
    }

for dest, src in simple_render.items():
    template = env.get_template(src)
    os.makedirs(
        os.path.join(args.directory, os.path.dirname(dest)), exist_ok=True)
    with open(os.path.join(args.directory, dest), 'w') as f:
        f.write(template.render())


template = env.get_template('lintian-fixes.html')
lintian_fixes_dir = os.path.join(args.directory, 'lintian-fixes')
os.makedirs(lintian_fixes_dir, exist_ok=True)
with open(os.path.join(lintian_fixes_dir, 'index.html'), 'w') as f:
    import lintian_brush
    f.write(template.render({'lintian_brush': lintian_brush}))
