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

import os
import sys

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402


print("Queue")
print("=====")
print("")

for i, (queue_id, branch_url, mode, env, command) in enumerate(
        state.iter_queue(), 1):
    if command[0] == 'new-upstream':
        if len(command) > 0 and command[1] == 'snapshot':
            description = 'New upstream snapshot'
        else:
            description = 'New upstream'
            if env.get('CONTEXT'):
                description += ', expecting to merge %s' % env['CONTEXT']
    elif command[0] == 'lintian-brush':
        description = 'Lintian fixes'
            if env.get('CONTEXT'):
                description += ', expecting to fix: %s' % env['CONTEXT']
    else:
        raise AssertionError('invalid command %s' % command)
    print("%d. `%s <pkg/%s>`_ (%s)" % (
        i, env["PACKAGE"], env["PACKAGE"], description))
