#!/usr/bin/python3
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

"""Import VCS branches."""

from datetime import datetime, timedelta

import sys

from prometheus_client import (
    Gauge,
    REGISTRY,
    push_to_gateway,
    )


from . import state
from .vcs import (
    open_branch_ext,
    BranchOpenFailure,
    mirror_branches,
    )
from .trace import note


last_success_gauge = Gauge(
    'job_last_success_unixtime',
    'Last time a batch job successfully finished')


def main(argv=None):
    import argparse
    parser = argparse.ArgumentParser(prog='janitor.vcs_mirror')
    parser.add_argument(
        '--prometheus', type=str,
        help='Prometheus push gateway to export to.')
    parser.add_argument(
        '--vcs-result-dir', type=str,
        help='Directory to store VCS repositories in.')

    args = parser.parse_args()

    # TODO(jelmer): Special handling for salsa
    for package, suite, branch_url in state.iter_unscanned_branches(
            last_scanned_minimum=timedelta(days=7)):
        note('Processing %s', package)
        try:
            branch = open_branch_ext(branch_url)
        except BranchOpenFailure as e:
            state.update_branch_status(
                branch_url, last_scanned=datetime.now(), status=e.code,
                revision=None)
        else:
            mirror_branches(
                args.vcs_result_dir, package, [(suite, branch)],
                public_master_branch=branch)
            state.update_branch_status(
                branch_url, last_scanned=datetime.now(), status='success',
                revision=branch.last_revision())

    last_success_gauge.set_to_current_time()
    if args.prometheus:
        push_to_gateway(
            args.prometheus, job='janitor.vcs_mirror',
            registry=REGISTRY)


if __name__ == '__main__':
    sys.exit(main(sys.argv))
