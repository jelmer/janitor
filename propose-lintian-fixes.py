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


import argparse
import os

from prometheus_client import (
    CollectorRegistry,
    Counter,
    Gauge,
    push_to_gateway,
)

import silver_platter   # noqa: F401
from silver_platter.debian.lintian import (
    available_lintian_fixers,
)

from breezy.trace import (
    note,
    warning,
)

from breezy.plugins.propose.propose import (
    hosters,
)

import sys
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402
from janitor.schedule import schedule_udd  # noqa: E402
from janitor.worker import process_package  # noqa: E402

parser = argparse.ArgumentParser(prog='propose-lintian-fixes')
parser.add_argument("packages", nargs='*')
parser.add_argument('--lintian-log',
                    help="Path to lintian log file.", type=str,
                    default=None)
parser.add_argument("--fixers",
                    help="Fixers to run.", type=str, action='append')
parser.add_argument("--policy",
                    help="Policy file to read.", type=str,
                    default='policy.conf')
parser.add_argument("--dry-run",
                    help="Create branches but don't push or propose anything.",
                    action="store_true", default=False)
parser.add_argument('--propose-addon-only',
                    help='Fixers that should be considered add-on-only.',
                    type=str, action='append',
                    default=DEFAULT_ADDON_FIXERS)
parser.add_argument('--pre-check',
                    help='Command to run to check whether to process package.',
                    type=str)
parser.add_argument('--post-check',
                    help='Command to run to check package before pushing.',
                    type=str)
parser.add_argument('--build-command',
                    help='Build package to verify it.', type=str,
                    default='sbuild -v')
parser.add_argument('--shuffle',
                    help='Shuffle order in which packages are processed.',
                    action='store_true')
parser.add_argument('--refresh',
                    help='Discard old branch and apply fixers from scratch.',
                    action='store_true')
parser.add_argument('--log-dir',
                    help='Directory to store logs in.',
                    type=str, default='public_html/pkg')
parser.add_argument('--prometheus', type=str,
                    help='Prometheus push gateway to export to.')
parser.add_argument('--incoming', type=str,
                    help='Path to copy built Debian packages into.')
parser.add_argument(
    '--max-mps-per-maintainer',
    default=0,
    type=int, help='Maximum number of open merge proposals per maintainer.')
args = parser.parse_args()


class JanitorResult(object):

    def __init__(self, pkg, log_id, start_time, finish_time, description,
                 proposal_url=None, is_new=None):
        self.package = pkg
        self.log_id = log_id
        self.start_time = start_time
        self.finish_time = finish_time
        self.description = description
        self.proposal_url = proposal_url
        self.is_new = is_new

    @classmethod
    def from_worker_result(cls, worker_result):
        return JanitorResult(
            worker_result.pkg, worker_result.log_id,
            worker_result.start_time,
            worker_result.finish_time,
            worker_result.proposal_url,
            worker_result.is_new)


registry = CollectorRegistry()
packages_processed_count = Counter(
    'package_count', 'Number of packages processed.', registry=registry)
open_proposal_count = Gauge(
    'open_proposal_count', 'Number of open proposals.',
    labelnames=('maintainer',), registry=registry)
fixer_count = Counter(
    'fixer_count', 'Number of selected fixers.', registry=registry)
last_success_gauge = Gauge(
    'job_last_success_unixtime',
    'Last time a batch job successfully finished',
    registry=registry)


fixer_scripts = {}
for fixer in available_lintian_fixers():
    for tag in fixer.lintian_tags:
        fixer_scripts[tag] = fixer

available_fixers = set(fixer_scripts)
if args.fixers:
    available_fixers = available_fixers.intersection(set(args.fixers))

fixer_count.inc(len(available_fixers))

if args.max_mps_per_maintainer or args.prometheus:
    # Don't put in the effort if we don't need the results.
    # Querying GitHub in particular is quite slow.
    open_proposals = []
    for name, hoster_cls in hosters.items():
        for instance in hoster_cls.iter_instances():
            note('Checking open merge proposals on %r...', instance)
            open_proposals.extend(instance.iter_my_proposals(status='open'))

    open_mps_per_maintainer = {}
    for proposal in open_proposals:
        maintainer_email = state.get_maintainer_email(proposal.url)
        if maintainer_email is None:
            warning('No maintainer email known for %s', proposal.url)
            continue
        open_mps_per_maintainer.setdefault(maintainer_email, 0)
        open_mps_per_maintainer[maintainer_email] += 1
        open_proposal_count.labels(maintainer=maintainer_email).inc()

possible_transports = []
possible_hosters = []

note('Querying UDD...')
todo = schedule_udd(
    args.policy, args.propose_addon_only, args.packages,
    available_fixers, args.shuffle)

for (vcs_url, mode, env, command) in todo:
    maintainer_email = env['MAINTAINER_EMAIL']
    if args.max_mps_per_maintainer and \
            open_mps_per_maintainer.get(maintainer_email, 0) \
            >= args.max_mps_per_maintainer:
        warning(
            'Skipping %s, maximum number of open merge proposals reached '
            'for maintainer %s', env['PACKAGE'], maintainer_email)
        continue
    if mode == "attempt-push" and "salsa.debian.org/debian/" in vcs_url:
        # Make sure we don't accidentally push to unsuspecting collab-maint
        # repositories, even if debian-janitor becomes a member of "debian"
        # in the future.
        mode = "propose"
    packages_processed_count.inc()
    worker_result = process_package(
        vcs_url, mode, env, command,
        output_directory=args.log_path,
        dry_run=args.dry_run, refresh=args.refresh,
        incoming=args.incoming,
        build_command=args.build_command,
        pre_check=args.pre_check,
        post_check=args.post_check,
        possible_transports=possible_transports,
        possible_hosters=possible_hosters)
    result = JanitorResult.from_worker_result(worker_result)
    if result.proposal_url:
        note('%s: %s: %s', result.package, result.description,
             result.proposal_url)
    else:
        note('%s: %s', result.package, result.description)
    state.store_run(
        result.log_id, env['PACKAGE'], vcs_url, env['MAINTAINER_EMAIL'],
        result.start_time, result.finish_time, command,
        result.description, result.proposal_url)
    if result.proposal_url and result.is_new:
        open_mps_per_maintainer.setdefault(maintainer_email, 0)
        open_mps_per_maintainer[maintainer_email] += 1
        open_proposal_count.labels(maintainer=maintainer_email).inc()

last_success_gauge.set_to_current_time()
if args.prometheus:
    push_to_gateway(args.prometheus, job='propose-lintian-fixes',
                    registry=registry)
