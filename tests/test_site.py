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
from jinja2 import Environment

from janitor.site import format_duration, template_loader


def test_some():
    assert "10s" == format_duration(timedelta(seconds=10))
    assert "1m10s" == format_duration(timedelta(seconds=70))
    assert "1h0m" == format_duration(timedelta(hours=1))
    assert "1d1h" == format_duration(timedelta(days=1, hours=1))
    assert "2w1d" == format_duration(timedelta(weeks=2, days=1))


def test_render_merge_proposal():
    env = Environment(loader=template_loader)
    template = env.get_template('cupboard/merge-proposal.html')
    template.render(proposal={
        'url': 'https://github.com/jelmer/example/pulls/1',
        'package': 'zz',
    })
