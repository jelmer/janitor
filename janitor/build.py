#!/usr/bin/python
# Copyright (C) 2018 Jelmer Vernooij <jelmer@jelmer.uk>
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

__all__ = [
    'changes_filename',
    'get_build_architecture',
    'add_dummy_changelog_entry',
    'build',
]

import os
import subprocess

from debian.changelog import Changelog

from breezy.plugins.debian.util import (
    changes_filename,
    get_build_architecture,
    )

from silver_platter.debian import BuildFailedError


from .trace import note


def add_dummy_changelog_entry(directory, suffix, suite, message):
    """Add a dummy changelog entry to a package.

    Args:
      directory: Directory to run in
      suffix: Suffix for the version
      suite: Debian suite
      message: Changelog message
    """
    subprocess.check_call(
        ["dch", "-l" + suffix, "--no-auto-nmu", "--distribution", suite,
            "--force-distribution", message], cwd=directory,
        stderr=subprocess.DEVNULL)


def get_latest_changelog_version(local_tree):
    with open(os.path.join(
            local_tree.basedir, 'debian', 'changelog'), 'r') as f:
        cl = Changelog(f, max_blocks=1)
        return cl.package, cl.version


def build(local_tree, outf, build_command='build', result_dir=None,
          distribution=None):
    args = ['brz', 'builddeb', '--builder=%s' % build_command]
    if result_dir:
        args.append('--result-dir=%s' % result_dir)
    outf.write('Running %r\n' % (build_command, ))
    outf.flush()
    env = dict(os.environ.items())
    if distribution is not None:
        env['DISTRIBUTION'] = distribution
    note('Building debian packages, running %r.', build_command)
    try:
        subprocess.check_call(
            args, cwd=local_tree.basedir, stdout=outf, stderr=outf,
            env=env)
    except subprocess.CalledProcessError:
        raise BuildFailedError()


def parse_sbuild_log(f):
    paragraphs = {}
    title = None
    sep = '+' + ('-' * 78) + '+'
    l = f.readline()
    while l:
        if l.strip() == sep:
            l1 = f.readline()
            l2 = f.readline()
            if (l1[0] == '|' and
                l1.strip()[-1] == '|' and l2.strip() == sep):
                title = l1[1:-1].strip()
                paragraphs[title] = []
            else:
                paragraphs[title].extend([l, l1, l2])
        else:
            paragraphs[title].append(l)
        l = f.readline()
    return paragraphs


def find_failed_stage(ls):
    for l in ls:
        if not l.startswith('Fail-Stage: '):
            continue
        (key, value) = l.split(': ', 1)
        return value.strip()
