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
import json
import os

from janitor import SUITES


def format_duration(duration):
    return '%dm%02ds' % (duration.seconds / 60, duration.seconds % 60)


def format_timestamp(ts):
    return ts.isoformat(timespec='minutes')


env = Environment(
    loader=PackageLoader('janitor.site', 'templates'),
    autoescape=select_autoescape(['html', 'xml']),
    enable_async=True,
)

env.globals.update(format_duration=format_duration)
env.globals.update(format_timestamp=format_timestamp)
env.globals.update(suites=SUITES)
env.globals.update(json_dumps=json.dumps)


def get_build_architecture():
    # TODO(jelmer): don't hardcode this
    return "amd64"


def get_run_diff(vcs_manager, run):
    from breezy.diff import show_diff_trees
    from breezy.errors import NoSuchRevision, NotBranchError
    from io import BytesIO

    f = BytesIO()
    try:
        repo = vcs_manager.get_repository(run.package)
    except NotBranchError:
        repo = None
    if repo is None:
        return b'Local VCS repository for %s temporarily inaccessible' % (
            run.package.encode('ascii'))
    try:
        old_tree = repo.revision_tree(run.main_branch_revision)
    except NoSuchRevision:
        return b'Old revision %s temporarily missing' % (
            run.main_branch_revision)
    try:
        new_tree = repo.revision_tree(run.revision)
    except NoSuchRevision:
        return b'New revision %s temporarily missing' % (
            run.revision)
    show_diff_trees(old_tree, new_tree, to_file=f)
    return f.getvalue()


def highlight_diff(diff):
    from pygments import highlight
    from pygments.lexers.diff import DiffLexer
    from pygments.formatters import HtmlFormatter
    return highlight(diff, DiffLexer(stripnl=False), HtmlFormatter())


def open_changes_file(run, changes_name):
    path = os.path.join(
            os.path.dirname(__file__), '..', '..',
            "public_html", run.build_distribution, changes_name)
    return open(path, 'rb')


def changes_get_binaries(cf):
    changes = Changes(cf)
    return changes['Binary'].split(' ')
