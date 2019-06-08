#!/usr/bin/python3

import os

from jinja2 import Environment, FileSystemLoader, select_autoescape
env = Environment(
    loader=FileSystemLoader('templates'),
    autoescape=select_autoescape(['html', 'xml'])
)

template = env.get_template('index.html')
with open('html/index.html', 'w') as f:
    f.write(template.render())

os.makedirs('html/contact', exist_ok=True)

template = env.get_template('contact.html')
with open('html/contact/index.html', 'w') as f:
    f.write(template.render())
