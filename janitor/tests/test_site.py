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

from datetime import timedelta

from janitor.site import format_duration

import unittest


class FormatDurationTests(unittest.TestCase):

    def test_some(self):
        self.assertEqual('10s', format_duration(timedelta(seconds=10)))
        self.assertEqual('1m10s', format_duration(timedelta(seconds=70)))
        self.assertEqual('1h0m', format_duration(timedelta(hours=1)))
        self.assertEqual('1d1h', format_duration(timedelta(days=1, hours=1)))
        self.assertEqual('2w1d', format_duration(timedelta(weeks=2, days=1)))
