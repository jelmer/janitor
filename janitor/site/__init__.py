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

from debian.deb822 import Changes
from jinja2 import Environment, PackageLoader, select_autoescape
import os

from janitor.vcs import SUPPORTED_VCSES


def format_duration(duration):
    return '%dm%02ds' % (duration.seconds / 60, duration.seconds % 60)


def format_timestamp(ts):
    return str(ts)


env = Environment(
    loader=PackageLoader('janitor.site', 'templates'),
    autoescape=select_autoescape(['html', 'xml']),
    enable_async=True,
)

env.globals.update(format_duration=format_duration)
env.globals.update(format_timestamp=format_timestamp)


def get_local_vcs_repo(package):
    import breezy.git  # noqa: F401
    import breezy.bzr  # noqa: F401
    from breezy.repository import Repository
    for vcs in SUPPORTED_VCSES:
        path = os.path.join(
            os.path.dirname(__file__), '..', '..', 'vcs', vcs, package)
        if not os.path.exists(path):
            continue
        return Repository.open(path)
    return None


def get_run_diff(run):
    from breezy.diff import show_diff_trees
    from io import BytesIO

    f = BytesIO()
    repo = get_local_vcs_repo(run.package)
    if repo is None:
        return None
    old_tree = repo.revision_tree(run.main_branch_revision)
    new_tree = repo.revision_tree(run.revision)
    show_diff_trees(old_tree, new_tree, to_file=f)
    return f.getvalue()


def highlight_diff(diff):
    from pygments import highlight
    from pygments.lexers.diff import DiffLexer
    from pygments.formatters import HtmlFormatter
    return highlight(diff, DiffLexer(stripnl=False), HtmlFormatter())


def get_changes_path(run, changes_name):
    path = os.path.join(
            os.path.dirname(__file__), '..', '..',
            "public_html", run.build_distribution, changes_name)
    if not os.path.exists(path):
        return None
    return path


def changes_get_binaries(changes_path):
    with open(changes_path, "r") as cf:
        changes = Changes(cf)
        return changes['Binary'].split(' ')
