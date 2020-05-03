#!/usr/bin/python3
# Copyright (C) 2020 Jelmer Vernooij <jelmer@jelmer.uk>
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
import subprocess
import sys

if os.path.exists('package.xml'):
    sys.exit(subprocess.call(['pear', 'package']))
elif os.path.exists('pyproject.toml'):
    sys.exit(subprocess.call(['/home/janitor/.local/bin/poetry', 'build', '-f', 'sdist']))
elif os.path.exists('dist.ini') and not os.path.exists('Makefile.PL'):
    with open('dist.ini', 'rb') as f:
        for l in f:
            if not l.startswith(b';;'):
                continue
            try:
                (key, value) = l[2:].split(b'=', 1)
            except ValueError:
                continue
            if (key.strip() == b'class' and
                    value.strip().startswith(b"'Dist::Inkt")):
                sys.exit(subprocess.call(['distinkt-dist']))
sys.exit(2)
