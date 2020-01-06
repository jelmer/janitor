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
import os
import json

from breezy.patches import iter_hunks


class DiffoscopeError(Exception):
    """An error occurred while running diffoscope."""


def filter_boring_udiff(udiff, old_version, new_version, display_version):
    old_version = old_version.encode('utf-8')
    new_version = new_version.encode('utf-8')
    display_version = display_version.encode('utf-8')
    lines = iter([l.encode('utf-8') for l in udiff.splitlines(True)])
    hunks = []
    for hunk in iter_hunks(lines, allow_dirty=False):
        for line in hunk.lines:
            line.contents = line.contents.replace(old_version, display_version)
            line.contents = line.contents.replace(new_version, display_version)
        hunks.append(hunk)
    return ''.join([hunk.as_bytes().decode('utf-8', 'replace')])


def filter_boring_detail(detail, old_version, new_version, display_version):
    if detail['unified_diff'] is not None:
        detail['unified_diff'] = filter_boring_udiff(
            detail['unified_diff'], old_version, new_version, display_version)
    detail['source1'] = detail['source1'].replace(old_version, display_version)
    detail['source2'] = detail['source2'].replace(new_version, display_version)
    if detail.get('details'):
        i = 0
        for subdetail in list(detail['details']):
            if not filter_boring_detail(
                    subdetail, old_version, new_version, display_version):
                continue
            i += 1
    return True


def filter_boring(diff, old_version, new_version):
    display_version = new_version.rsplit('~', 1)[0]
    # Changes file differences
    BORING_FIELDS = ['Date', 'Distribution', 'Version']
    i = 0
    for detail in list(diff['details']):
        if (detail['source1'] in BORING_FIELDS and
                detail['source2'] in BORING_FIELDS):
            del diff['details'][i]
            continue
        if (detail['source1'].endswith('.buildinfo') and
                detail['source2'].endswith('.buildinfo')):
            del diff['details'][i]
            continue
        if not filter_boring_detail(
                detail, old_version, new_version, display_version):
            continue
        i += 1


def filter_irrelevant(diff):
    diff['source1'] = os.path.basename(diff['source1'])
    diff['source2'] = os.path.basename(diff['source2'])


async def format_diffoscope(diffoscope_diff, content_type):
    args = ['diffoscope']
    args.extend({
        'application/json': ['--json=-'],
        'text/plain': ['--text=-'],
        'text/html': ['--html=-'],
        'text/markdown': ['--markdown=-'],
    }[content_type])
    args.extend(['-'])
    stdout = BytesIO()
    p = await asyncio.create_subprocess_exec(
        *args, stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE)
    stdout, stderr = await p.communicate(
        json.dumps(diffoscope_diff).encode('utf-8'))
    if p.returncode not in (0, 1):
        raise DiffoscopeError(stderr.decode(errors='replace'))
    return stdout


async def run_diffoscope(old_changes, new_changes):
    args = ['diffoscope', '--json=-', '--exclude-directory-metadata=yes']
    args.extend([old_changes, new_changes])
    stdout = BytesIO()
    p = await asyncio.create_subprocess_exec(
        *args, stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE)
    stdout, stderr = await p.communicate(b'')
    if p.returncode not in (0, 1):
        raise DiffoscopeError(stderr.decode(errors='replace'))
    return json.loads(stdout.decode('utf-8'))
