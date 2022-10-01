#!/usr/bin/python3
# Copyright (C) 2018-2022 Jelmer Vernooij <jelmer@jelmer.uk>
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

import logging
import os
import sys
import traceback

from ognibuild.build import run_build
from ognibuild.test import run_test
from ognibuild.buildlog import InstallFixer
from ognibuild.session import SessionSetupFailure
from ognibuild.session.plain import PlainSession
from ognibuild.session.schroot import SchrootSession
from ognibuild.resolver import auto_resolver
from ognibuild import UnidentifiedError, DetailedFailure
from ognibuild.buildsystem import (
    NoBuildToolsFound,
    detect_buildsystems,
)


# TODO(jelmer): Get rid of this circular import
from ..worker import WorkerFailure


def build(local_tree, subpath, output_directory, chroot=None, dep_server_url=None):
    if chroot:
        session = SchrootSession(chroot)
        logging.info('Using schroot %s', chroot)
    else:
        session = PlainSession()
    try:
        with session:
            resolver = auto_resolver(session, dep_server_url=dep_server_url)
            fixers = [InstallFixer(resolver)]
            external_dir, internal_dir = session.setup_from_vcs(local_tree)
            bss = list(detect_buildsystems(os.path.join(external_dir, subpath)))
            session.chdir(os.path.join(internal_dir, subpath))
            try:
                try:
                    run_build(session, buildsystems=bss, resolver=resolver, fixers=fixers)
                except NotImplementedError as e:
                    traceback.print_exc()
                    raise WorkerFailure('build-action-unknown', str(e))
                try:
                    run_test(session, buildsystems=bss, resolver=resolver, fixers=fixers)
                except NotImplementedError as e:
                    traceback.print_exc()
                    raise WorkerFailure('test-action-unknown', str(e))
            except NoBuildToolsFound as e:
                raise WorkerFailure('no-build-tools-found', str(e))
            except DetailedFailure as f:
                raise WorkerFailure(f.error.kind, str(f.error), details={'command': f.argv})
            except UnidentifiedError as e:
                lines = [line for line in e.lines if line]
                if e.secondary:
                    raise WorkerFailure('build-failed', e.secondary.line)
                elif len(lines) == 1:
                    raise WorkerFailure('build-failed', lines[0])
                else:
                    raise WorkerFailure(
                        'build-failed',
                        "%r failed with unidentified error "
                        "(return code %d)" % (e.argv, e.retcode)
                    )
    except SessionSetupFailure as e:
        if e.errlines:
            sys.stderr.buffer.writelines(e.errlines)
        raise WorkerFailure('session-setup-failure', str(e))

    return {}


def build_from_config(local_tree, subpath, output_directory, config, env):
    chroot = config.get("chroot")
    dep_server_url = config.get("dep_server_url")
    return build(
        local_tree, subpath, output_directory, chroot=chroot,
        dep_server_url=dep_server_url)


def main():
    import argparse
    import json
    parser = argparse.ArgumentParser()
    parser.add_argument('--config', type=str, help="Path to configuration (JSON)")
    parser.add_argument('output-directory', type=str, help="Output directory")
    args = parser.parse_args()

    import breezy.bzr  # noqa: F401
    import breezy.git  # noqa: F401
    from breezy.workingtree import WorkingTree

    wt, subpath = WorkingTree.open_containing('.')

    if args.config:
        with open(args.config, 'r') as f:
            config = json.load(f)
    else:
        config = {}

    try:
        result = build_from_config(
            wt, subpath, args.output_directory, config=config,
            env=os.environ)
    except WorkerFailure as e:
        json.dump(e.json())
        return 1

    json.dump(result, sys.stdout, indent=4)
    return 0


if __name__ == '__main__':
    sys.exit(main())
