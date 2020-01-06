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

import asyncio
from io import BytesIO


class DiffoscopeError(Exception):
    """An error occurred while running diffoscope."""


async def run_diffoscope(old_changes, new_changes, content_type):
    args = ['diffoscope']
    args.extend({
        'application/json': ['--json=-'],
        'text/plain': ['--text=-'],
        'text/html': ['--html=-'],
        'text/markdown': ['--markdown=-'],
    }[content_type])

    args.extend([old_changes, new_changes])
    stdout = BytesIO()
    p = await asyncio.create_subprocess_exec(
        *args, stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE)
    stdout, stderr = await p.communicate(b'')
    if p.returncode not in (0, 1):
        raise DiffoscopeError(stderr)
    return stdout
