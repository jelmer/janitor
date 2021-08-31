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

from contextlib import contextmanager, ExitStack
import errno
import logging
import os
import sys

from ognibuild.session import SessionSetupFailure
from ognibuild.dist import (
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

from breezy.plugins.debian.upstream.branch import (
    DistCommandFailed,
    )

from buildlog_consultant.common import (
    NoSpaceOnDevice,
)

logger = logging.getLogger(__name__)


@contextmanager
def redirect_output(to_file):
    sys.stdout.flush()
    sys.stderr.flush()
    old_stdout = os.dup(sys.stdout.fileno())
    old_stderr = os.dup(sys.stderr.fileno())
    os.dup2(to_file.fileno(), sys.stdout.fileno())  # type: ignore
    os.dup2(to_file.fileno(), sys.stderr.fileno())  # type: ignore
    try:
        yield
    finally:
        sys.stdout.flush()
        sys.stderr.flush()
        os.dup2(old_stdout, sys.stdout.fileno())
        os.dup2(old_stderr, sys.stderr.fileno())


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
    args = parser.parse_args()

    from ognibuild.session.schroot import SchrootSession

    import breezy.bzr
    import breezy.git
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
                raise DetailedFailure(1, ["mkdtemp"], NoSpaceOnDevice())
            raise

        if args.packaging:
            packaging_tree, packaging_debian_path = WorkingTree.open_containing(args.packaging)
            from ognibuild.debian import satisfy_build_deps

            satisfy_build_deps(session, packaging_tree, packaging_debian_path)
        else:
            packaging_tree = None
            packaging_debian_path = None

        try:
            if version:
                os.environ['SETUPTOOLS_SCM_PRETEND_VERSION'] = version

            if args.log_directory:
                distf = es.enter_context(open(os.path.join(args.log_directory, 'dist.log'), 'wb'))
                es.enter_context(redirect_output(distf))

            target_dir = os.path.abspath(os.path.join(args.directory, args.target_dir))

            try:
                dist(session, export_directory, reldir, target_dir)
            except NotImplementedError:
                sys.exit(2)
            except NoBuildToolsFound:
                logger.info("No build tools found, falling back to simple export.")
                sys.exit(2)
            except UnidentifiedError as e:
                lines = [line for line in e.lines if line]
                if e.secondary:
                    raise DistCommandFailed(e.secondary.line)
                elif len(lines) == 1:
                    raise DistCommandFailed(lines[0])
                else:
                    raise DistCommandFailed(
                        "%r failed with unidentified error "
                        "(return code %d)" % (e.argv, e.retcode)
                    )
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

    sys.exit(0)
