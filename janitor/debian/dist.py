#!/usr/bin/python3
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

from contextlib import ExitStack
import errno
import logging
import os
import sys
import traceback

from ognibuild.session import SessionSetupFailure
from ognibuild.dist import (
    DIST_LOG_FILENAME,
    dist,
    DistNoTarball,
)
from ognibuild import (
    DetailedFailure,
    UnidentifiedError,
)
from ognibuild.buildsystem import (
    NoBuildToolsFound,
)
from ognibuild.logs import (
    DirectoryLogManager,
    NoLogManager,
)


logger = logging.getLogger(__name__)


def report_failure(kind, description, original):
    logging.fatal('%s: %s', kind, description)
    if 'DIST_RESULT' in os.environ:
        import json
        with open(os.environ['DIST_RESULT'], 'w') as f:
            json.dump({
                'result_code': kind,
                'description': description}, f)


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument(
        '--log-directory', type=str, default=None,
        help='Write logs to files in specified directory rather than standard out')
    parser.add_argument(
        '--schroot', type=str, default=os.environ.get('SCHROOT'),
        help='Schroot to use')
    parser.add_argument(
        '--target-dir', type=str, default='..',
        help='Directory to write to')
    parser.add_argument(
        '--packaging', type=str, default=None,
        help='Location of packaging')
    parser.add_argument(
        '--directory', '-d',
        type=str, default='.',
        help='Path to tree to create dist tarball for')
    parser.add_argument(
        '--require-declared',
        action='store_true',
        help='Fail if declared dependencies can not be installed')
    args = parser.parse_args()

    from ognibuild.session.schroot import SchrootSession

    import breezy.bzr  # noqa: F401
    import breezy.git  # noqa: F401
    from breezy.errors import NotBranchError
    from breezy.workingtree import WorkingTree

    logging.basicConfig(level=logging.INFO, format='%(message)s')

    package = os.environ.get('PACKAGE')
    version = os.environ.get('VERSION')

    with ExitStack() as es:
        subdir = package or "package"

        session = es.enter_context(SchrootSession(args.schroot))
        try:
            try:
                tree = WorkingTree.open(args.directory)
            except NotBranchError:
                export_directory, reldir = session.setup_from_directory(
                    args.directory, subdir)
            else:
                export_directory, reldir = session.setup_from_vcs(
                    tree, include_controldir=True, subdir=subdir
                )
        except OSError as e:
            if e.errno == errno.ENOSPC:
                report_failure(
                    'no-space-on-device', 'No space on device running mkdtemp',
                    e)
                sys.exit(1)
            raise

        if args.packaging:
            (packaging_tree,
             packaging_debian_path) = WorkingTree.open_containing(
                args.packaging)
            from ognibuild.debian import satisfy_build_deps

            try:
                satisfy_build_deps(
                    session, packaging_tree, packaging_debian_path)
            except DetailedFailure as e:
                logging.warning(
                    'Ignoring error installing declared build dependencies '
                    '(%s): %s', e.error.kind, str(e.error))
                if args.require_declared:
                    sys.exit(1)
            except UnidentifiedError as e:
                lines = [line for line in e.lines if line]
                if e.secondary:
                    logging.warning(
                        'Ignoring error installing '
                        'declared build dependencies (%r): %s',
                        e.argv, e.secondary.line)
                    report_failure('dist-command-failed', e.secondary.line, e)
                elif len(lines) == 1:
                    logging.warning(
                        'Ignoring error installing declared '
                        'build dependencies (%r): %s',
                        e.argv, lines[0])
                else:
                    logging.warning(
                        'Ignoring error installing declared '
                        'build dependencies (%r): %r',
                        e.argv, lines)
                if args.require_declared:
                    sys.exit(1)
        else:
            packaging_tree = None
            packaging_debian_path = None

        try:
            target_dir = os.path.abspath(
                os.path.join(args.directory, args.target_dir))

            if args.log_directory:
                log_manager = DirectoryLogManager(
                    os.path.join(args.log_directory, DIST_LOG_FILENAME),
                    mode='redirect')
            else:
                log_manager = NoLogManager()

            try:
                dist(session, export_directory, reldir, target_dir,
                     version=version, log_manager=log_manager)
            except NotImplementedError:
                sys.exit(2)
            except NoBuildToolsFound:
                logger.info(
                    "No build tools found, falling back to simple export.")
                sys.exit(2)
        except UnidentifiedError as e:
            lines = [line for line in e.lines if line]
            if e.secondary:
                report_failure('dist-command-failed', e.secondary.line, e)
            elif len(lines) == 1:
                report_failure('dist-command-failed', lines[0], e)
            else:
                report_failure(
                    'dist-command-failed',
                    "%r failed with unidentified error "
                    "(return code %d)" % (e.argv, e.retcode), e)
            sys.exit(1)
        except SessionSetupFailure as e:
            report_failure('session-setup-failure', str(e), e)
            sys.exit(1)
        except DetailedFailure as e:
            if e.error.is_global:
                error_code = e.error.kind
            else:
                error_code = "dist-" + e.error.kind
            error_description = str(e.error)
            report_failure(error_code, error_description, e)
            sys.exit(1)
        except DistNoTarball as e:
            report_failure('dist-no-tarball', str(e), e)
            sys.exit(1)
        except BaseException as e:
            traceback.print_exc()
            report_failure('dist-exception', str(e), e)
            sys.exit(1)

    sys.exit(0)
