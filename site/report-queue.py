#!/usr/bin/python3
# Copyright (C) 2019 Jelmer Vernooij <jelmer@jelmer.uk>
#
# This program is free software; you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation; either version 2 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program; if not, write to the Free Software
# Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA

import argparse
import asyncio
import os
import sys

from jinja2 import Environment, FileSystemLoader, select_autoescape

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402

parser = argparse.ArgumentParser('report-queue')
parser.add_argument(
    '--command', type=str, help='Only display queue for specified command')
parser.add_argument(
    '--limit', type=int, help='Limit to this number of entries',
    default=100)
args = parser.parse_args()

loop = asyncio.get_event_loop()

data = []

for queue_id, branch_url, env, command in loop.run_until_complete(
        state.iter_queue(limit=args.limit)):
    if args.command is not None and command != args.command:
        continue
    expecting = None
    if command[0] == 'new-upstream':
        if '--snapshot' in command:
            description = 'New upstream snapshot'
        else:
            description = 'New upstream'
            if env.get('CONTEXT'):
                expecting = 'expecting to merge %s' % env['CONTEXT']
    elif command[0] == 'lintian-brush':
        description = 'Lintian fixes'
        if env.get('CONTEXT'):
            expecting = 'expecting to fix: ' + ', '.join([
                '<a href="https://lintian.debian.org/tags/%s.html">%s</a>' %
                (tag, tag) for tag in env['CONTEXT'].split(' ')])
    else:
        raise AssertionError('invalid command %s' % command)
    if args.command is not None:
        description = expecting
    elif expecting is not None:
        description += ", " + expecting
    data.append((env['PACKAGE'], description))


env = Environment(
    loader=FileSystemLoader('templates'),
    autoescape=select_autoescape(['html', 'xml'])
)
template = env.get_template('queue.html')
sys.stdout.write(template.render(queue=data))
