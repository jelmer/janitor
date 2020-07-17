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

import subprocess

from typing import Optional, List


class Session(object):

    def __init__(self, chroot):
        self.chroot = chroot
        self._location = None

    def _get_location(self):
        return subprocess.check_output(
            ['schroot', '--location', '-c', 'session:' + self.session_id
             ]).strip().decode()

    def _end_session(self):
        subprocess.check_output(
            ['schroot', '-c', 'session:' + self.session_id, '-e'])

    def __enter__(self):
        self.session_id = subprocess.check_output(
            ['schroot', '-c', self.chroot, '-b']).strip().decode()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self._end_session()
        return False

    @property
    def location(self):
        if self._location is None:
            self._location = self._get_location()
        return self._location

    def _run_argv(self, argv: List[str], cwd: Optional[str] = None,
                  user: Optional[str] = None):
        base_argv = ['schroot', '-r', '-c', 'session:' + self.session_id]
        if cwd is not None:
            base_argv.extend(['-d', cwd])
        if user is not None:
            base_argv.extend(['-u', user])
        return base_argv + ['--'] + argv

    def check_call(
            self,
            argv: List[str], cwd: Optional[str] = None,
            user: Optional[str] = None):
        subprocess.check_call(self._run_argv(argv, cwd, user))

    def call(
            self, argv: List[str], cwd: Optional[str] = None,
            user: Optional[str] = None):
        return subprocess.call(self._run_argv(argv, cwd, user))
