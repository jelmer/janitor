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
import re


def iter_sections(text):
    lines = list(text.splitlines(False))
    title = None
    paragraph = []
    for i, line in enumerate(lines):
        if i+1 < len(lines) and lines[i+1] == (len(line) * '-'):
            yield title, paragraph
            title = line
            paragraph = []
        elif paragraph or not line.startswith('---'):
            paragraph.append(line)
    if paragraph:
        yield title, paragraph


def filter_boring_wdiff(line, old_version, new_version):
    if not line:
        return line
    field, changes = line.split(':', 1)
    if field == 'Installed-Size':
        return None
    if field == 'Version':
        return None
    line = re.sub(
        r'\[-%s(.*)-\] \{\+%s\1\+\}' % (
            re.escape(old_version), re.escape(new_version)),
        '', line)
    if not re.findall(r'\[-.*-\] \{\+.*\+\}', line):
        return None
    return line


def filter_boring(debdiff, old_version, new_version):
    ret = []
    for title, paragraph in iter_sections(debdiff):
        if not title:
            ret.append((title, paragraph))
            continue
        m = re.match(
                r'Control files of package (.*): lines which differ '
                r'\(wdiff format\)',
                title)
        if m:
            package = m.group(1)
            paragraph = [
                filter_boring_wdiff(line, old_version, new_version)
                for line in paragraph]
            paragraph = [line for line in paragraph if line is not None]
        else:
            package = None
        if any([line.strip() for line in paragraph]):
            ret.append((title, paragraph))
        else:
            ret.append((
                None,
                ['No differences were encountered between the control files '
                 'of package %s\n' % package]))

    lines = []
    for title, paragraph in ret:
        if title is not None:
            lines.append(title)
            lines.append(len(title) * '-')
        lines.extend(paragraph)
    return '\n'.join(lines)


async def run_debdiff(old_changes, new_changes):
    args = ['debdiff', old_changes, new_changes]
    stdout = BytesIO()
    p = await asyncio.create_subprocess_exec(
        *args, stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE)
    stdout, stderr = await p.communicate()
    return stdout
