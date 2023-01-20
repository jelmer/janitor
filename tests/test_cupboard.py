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

from datetime import datetime

from jinja2 import Environment

from janitor.config import Campaign, Config
from janitor.site import template_loader, format_timestamp, format_duration, classify_result_code


def test_render_merge_proposal():
    env = Environment(loader=template_loader)
    template = env.get_template('cupboard/merge-proposal.html')
    template.render(proposal={
        'url': 'https://github.com/jelmer/example/pulls/1',
        'package': 'zz',
    })


def test_render_run():
    env = Environment(loader=template_loader)
    template = env.get_template('cupboard/run.html')
    campaign = Campaign()
    campaign.name = "some-fixes"
    template.render(
        run={
            "start_time": datetime.utcnow(),
            "finish_time": datetime.utcnow(),
        }, success_probability=0.2,
        classify_result_code=classify_result_code,
        show_debdiff=lambda: "",
        campaign=campaign,
        config=Config(),
        publish_blockers=dict,
        format_timestamp=format_timestamp, format_duration=format_duration)


def test_render_history():
    env = Environment(loader=template_loader)
    env.get_template('cupboard/history.html')


def test_render_queue():
    env = Environment(loader=template_loader)
    env.get_template('cupboard/queue.html')


def test_render_changeset():
    env = Environment(loader=template_loader)
    env.get_template('cupboard/changeset.html')


def test_render_broken_mps():
    env = Environment(loader=template_loader)
    env.get_template('cupboard/broken-merge-proposals.html')


def test_render_changeset_list():
    env = Environment(loader=template_loader)
    env.get_template('cupboard/changeset-list.html')


def test_render_rejected():
    env = Environment(loader=template_loader)
    env.get_template('cupboard/rejected.html')


def test_workers():
    env = Environment(loader=template_loader)
    env.get_template('cupboard/workers.html')


def test_start():
    env = Environment(loader=template_loader)
    env.get_template('cupboard/start.html')
