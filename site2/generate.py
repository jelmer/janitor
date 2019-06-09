#!/usr/bin/python3

import os

from jinja2 import Environment, FileSystemLoader, select_autoescape
env = Environment(
    loader=FileSystemLoader('templates'),
    autoescape=select_autoescape(['html', 'xml'])
)

simple_render = {
    'index.html': 'index.html',
    'contact/index.html': 'contact.html',
    'credentials/index.html': 'credentials.html',
    'apt/index.html': 'apt.html',
    }

for dest, src in simple_render.items():
    template = env.get_template(src)
    os.makedirs(os.path.join('html', os.path.dirname(dest)), exist_ok=True)
    with open(os.path.join('html', dest), 'w') as f:
        f.write(template.render())


template = env.get_template('lintian-fixes.html')
os.makedirs(os.path.join('html/lintian-fixes'), exist_ok=True)
with open(os.path.join('html', 'lintian-fixes', 'index.html'), 'w') as f:
    import lintian_brush
    f.write(template.render({'lintian_brush': lintian_brush}))
