#!/usr/bin/python
# Copyright (C) 2022 Jelmer Vernooij <jelmer@jelmer.uk>
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

from jinja2 import Environment, select_autoescape

from janitor.site import template_loader

env = Environment(loader=template_loader, autoescape=select_autoescape(["html", "xml"]))


def test_display_branch_url():
    template = env.get_template("run_util.html")
    assert (
        str(
            template.module.display_branch_url(  # type: ignore
                None, "https://github.com/jelmer/example.git"
            )
        )
        == """\

    
        <a href="https://github.com/jelmer/example.git">https://github.com/jelmer/example.git</a>
    
"""
    )
    assert (
        str(
            template.module.display_branch_url(  # type: ignore
                "https://github.com/jelmer/example.git",
                "https://github.com/jelmer/example",
            )
        )
        == """\

    
        <a href="https://github.com/jelmer/example.git">https://github.com/jelmer/example</a>
    
"""
    )


def test_display_publish_blockers():
    template = env.get_template("run_util.html")
    assert (
        str(
            template.module.display_publish_blockers(  # type: ignore
                {}
            )
        )
        == """\

    <ul>
        
    </ul>
"""
    )
    assert (
        str(
            template.module.display_publish_blockers(  # type: ignore
                {"inactive": {"result": True, "details": {}}}
            )
        )
        == """\

    <ul>
        
            <li>☑
                codebase is not inactive</li>
        
    </ul>
"""
    )
