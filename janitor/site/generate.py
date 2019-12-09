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

