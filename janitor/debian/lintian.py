#!/usr/bin/python3
# Copyright (C) 2019-2020 Jelmer Vernooij <jelmer@jelmer.uk>
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

import subprocess
import json
import os
import logging


logger = logging.getLogger(__name__)


def run_lintian(self, output_directory, changes_names):
    logger.info('Running lintian')
    args = ['--exp-output=format=json']
    if self.lintian_suppress_tags:
        args.append('--suppress-tags=' + self.lintian_suppress_tags)
    if self.lintian_profile:
        args.append('--profile=%s' % self.lintian_profile)
    try:
        lintian_output = subprocess.check_output(
            ['lintian'] + args +
            [os.path.join(output_directory, changes_name)
             for changes_name in changes_names])
    except subprocess.CalledProcessError:
        logger.warning('lintian failed to run.')
        return None
    lines = []
    for line in lintian_output.splitlines(True):
        lines.append(line)
        if line == b"}\n":
            break
    try:
        result = json.loads(b''.join(lines))
    except json.decoder.JSONDecodeError:
        logging.warning(
            'Error parsing lintian output: %r (%r)', lintian_output,
            b''.join(lines))
        return None

    # Strip irrelevant directory information
    for group in result.get('groups', []):
        for inp in group.get('input-files', []):
            inp['path'] = os.path.basename(inp['path'])

    return result
