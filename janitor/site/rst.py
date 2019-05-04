#!/usr/bin/python
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


def include_console_log(f, log_path, include_lines=None, highlight_lines=None):
    f.write('.. literalinclude:: %s\n' % os.path.basename(log_path))
    f.write('  :language: console\n')
    f.write('  :linenos:\n')
    if include_lines:
        f.write('  :lines: ')
        if include_lines[0] is not None:
            f.write('%d' % include_lines[0])
        f.write('-')
        if include_lines[1] is not None:
            f.write('%d' % include_lines[1])
        f.write('\n')
    if highlight_lines:
        f.write('   :emphasize-lines: ')
        for line in highlight_lines[:-1]:
            f.write('%d,' % line)
        f.write('%d\n' % highlight_lines[-1])
    f.write('\n')


def include_console_log_tail(f, log_path, tail):
    with open(log_path, 'r') as logf:
        linecount = logf.read().count('\n')
    if linecount > tail:
        include_console_log(f, log_path, include_lines=(linecount-tail, None))
    else:
        include_console_log(f, log_path)


def format_rst_table(f, header, data):
    def separator(lengths):
        for i, length in enumerate(lengths):
            if i > 0:
                f.write(' ')
            f.write('=' * length)
        f.write('\n')
    lengths = [
        max([len(str(x[i])) for x in [header] + data])
        for i in range(len(header))]
    separator(lengths)
    for i, (column, length) in enumerate(zip(header, lengths)):
        if i > 0:
            f.write(' ')
        f.write(column + (' ' * (length - len(column))))
    f.write('\n')
    separator(lengths)
    for row in data:
        for i, (column, length) in enumerate(zip(row, lengths)):
            if i > 0:
                f.write(' ')
            f.write(str(column) + (' ' * (length - len(str(column)))))
        f.write('\n')
    separator(lengths)
