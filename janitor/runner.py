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


from breezy.plugins.propose.propose import (
    hosters,
)
from breezy.trace import note, warning

from prometheus_client import (
    Counter,
    Gauge,
)

from janitor import state
from janitor.worker import process_package

open_proposal_count = Gauge(
    'open_proposal_count', 'Number of open proposals.',
    labelnames=('maintainer',))
packages_processed_count = Counter(
    'package_count', 'Number of packages processed.')


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
            worker_result.package, worker_result.log_id,
            worker_result.start_time,
            worker_result.finish_time,
            worker_result.proposal_url,
            worker_result.is_new)


def get_open_mps_per_maintainer():
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
    return open_mps_per_maintainer


def process_queue(
        todo, max_mps_per_maintainer,
        build_command, open_mps_per_maintainer,
        refresh=False, pre_check=None, post_check=None,
        dry_run=False, incoming=None, output_directory=None):

    possible_transports = []
    possible_hosters = []

    for (vcs_url, mode, env, command) in todo:
        maintainer_email = env['MAINTAINER_EMAIL']
        if max_mps_per_maintainer and \
                open_mps_per_maintainer.get(maintainer_email, 0) \
                >= max_mps_per_maintainer:
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
            output_directory=output_directory,
            dry_run=dry_run, refresh=refresh,
            incoming=incoming,
            build_command=build_command,
            pre_check_command=pre_check,
            post_check_command=post_check,
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
