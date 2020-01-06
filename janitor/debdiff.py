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
    i = 0
    while i < len(lines):
        line = lines[i]
        if i+1 < len(lines) and lines[i+1] == (len(line) * '-'):
            if title or paragraph:
                yield title, paragraph
            title = line
            paragraph = []
            i += 1
        elif not line.rstrip('\n'):
            if title or paragraph:
                yield title, paragraph
            title = None
            paragraph = []
        else:
            paragraph.append(line)
        i += 1
    if title or paragraph:
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
            wdiff = True
        elif title == 'Control files: lines which differ (wdiff format)':
            package = None
            wdiff = True
        else:
            package = None
            wdiff = False
        if wdiff:
            paragraph = [
                filter_boring_wdiff(line, old_version, new_version)
                for line in paragraph]
            paragraph = [line for line in paragraph if line is not None]
            if any([line.strip() for line in paragraph]):
                ret.append((title, paragraph))
            else:
                if package:
                    ret.append((
                        None,
                        ['No differences were encountered between the control '
                         'files of package %s' % package]))
                else:
                    ret.append((
                        None,
                        ['No differences were encountered in the control files'
                         ]))
        else:
            ret.append((title, paragraph))

    lines = []
    for title, paragraph in ret:
        if title is not None:
            lines.append(title)
            lines.append(len(title) * '-')
        lines.extend(paragraph)
        lines.append('')
    return '\n'.join(lines)


class DebdiffError(Exception):
    """Error occurred while running debdiff."""


async def run_debdiff(old_changes, new_changes):
    args = ['debdiff', old_changes, new_changes]
    stdout = BytesIO()
    p = await asyncio.create_subprocess_exec(
        *args, stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE)
    stdout, stderr = await p.communicate(b'')
    if p.returncode not in (0, 1):
        raise DebdiffError(stderr)
    return stdout


def debdiff_is_empty(debdiff):
    return any(
        [title is not None for (title, paragraph) in iter_sections(debdiff)])


def section_is_wdiff(title):
    m = re.match(
            r'Control files of package (.*): lines which differ '
            r'\(wdiff format\)',
            title)
    if m:
        return (True, m.group(1))
    if title == 'Control files: lines which differ (wdiff format)':
        return (True, None)
    return (False, None)


def markdownify_debdiff(debdiff):
    def fix_wdiff_md(line):
        # GitLab markdown will render links but then not show the
        # delta highlighting. This fools it into not autolinking:
        return line.replace('://', '&#8203;://')
    ret = []
    for title, lines in iter_sections(debdiff):
        if title:
            ret.append("### %s" % title)
            wdiff, package = section_is_wdiff(title)
            if wdiff:
                ret.extend(
                    ["* %s" % fix_wdiff_md(line)
                     for line in lines if line.strip()])
            else:
                for line in lines:
                    ret.append('    ' + line)
        else:
            ret.append("")
            for line in lines:
                if line.strip():
                    line = re.sub(
                        '^(No differences were encountered between the '
                        'control files of package) (.*)$',
                        r'\1 \*\2\*', line)
                    ret.append(line)
                else:
                    ret.append("")
            if ret[-1] == "":
                ret.pop(-1)
    return "\n".join(ret)


def htmlize_debdiff(debdiff):
    def highlight_wdiff(line):
        line = re.sub(
            r'\[-(.*)-\]',
            r'<span style="color:red;font-weight:bold">\1</span>', line)
        line = re.sub(
            r'\{\+(.*)\+\}',
            r'<span style="color:green;font-weight:bold">\1</span>', line)
        return line
    ret = []
    for title, lines in iter_sections(debdiff):
        if title:
            ret.append("<h4>%s</h4>" % title)
            if re.match(
                    r'Control files of package .*: lines which differ '
                    r'\(wdiff format\)',
                    title):
                wdiff = True
            elif title == 'Control files: lines which differ (wdiff format)':
                wdiff = True
            else:
                wdiff = False
            if wdiff:
                ret.append("<ul>")
                ret.extend(
                    ["<li>%s</li>" % highlight_wdiff(line)
                     for line in lines if line.strip()])
                ret.append("</ul>")
            else:
                ret.append("<pre>")
                ret.extend(lines)
                ret.append("</pre>")
        else:
            ret.append("<p>")
            for line in lines:
                if line.strip():
                    line = re.sub(
                        '^(No differences were encountered between the '
                        'control files of package) (.*)$',
                        '\\1 <b>\\2</b>', line)
                    ret.append(line)
                else:
                    ret.append("</p>")
                    ret.append("<p>")
            if ret[-1] == "<p>":
                ret.pop(-1)
    return "\n".join(ret)
