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

from contextlib import contextmanager
import logging
import os
import sys

from ognibuild.session import SessionSetupFailure
from ognibuild.dist import (
    create_dist_schroot,
    DistNoTarball,
)
from ognibuild import (
    DetailedFailure,
    UnidentifiedError,
)
from ognibuild.buildsystem import (
    NoBuildToolsFound,
)

from silver_platter.changer import ChangerError
from breezy.plugins.debian.upstream.branch import (
    DistCommandFailed,
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


def create_dist(
        log_directory, tree, package, version, target_dir, schroot=None,
        packaging_tree=None, packaging_debian_path=None):
    os.environ['SETUPTOOLS_SCM_PRETEND_VERSION'] = version

    with open(os.path.join(log_directory, 'dist.log'), 'wb') as distf, redirect_output(distf):
        try:
            return create_dist_schroot(
                tree,
                subdir=package,
                target_dir=target_dir,
                chroot=schroot,
                packaging_tree=packaging_tree,
                packaging_subpath=packaging_debian_path,
            )
        except NotImplementedError:
            return None
        except SessionSetupFailure as e:
            raise ChangerError('session-setup-failure', str(e))
        except NoBuildToolsFound:
            logger.info("No build tools found, falling back to simple export.")
            return None
        except DetailedFailure as e:
            if e.error.is_global:
                error_code = e.error.kind
            else:
                error_code = "dist-" + e.error.kind
            error_description = str(e.error)
            raise ChangerError(
                summary=error_description, category=error_code, original=e
            )
        except DistNoTarball as e:
            raise ChangerError('dist-no-tarball', str(e))
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


if __name__ == '__main__':
    pass
