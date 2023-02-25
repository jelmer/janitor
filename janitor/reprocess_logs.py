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

import logging
from datetime import timedelta
from typing import Optional

import silver_platter  # noqa: E402, F401
from buildlog_consultant.common import \
    find_build_failure_description  # noqa: E402
from buildlog_consultant.sbuild import \
    worker_failure_from_sbuild_log  # noqa: E402

from janitor.schedule import do_schedule


def process_sbuild_log(logf):
    failure = worker_failure_from_sbuild_log(logf)
    if failure.error:
        if failure.stage and not failure.error.is_global:
            new_code = f"{failure.stage}-{failure.error.kind}"
        else:
            new_code = failure.error.kind
        try:
            new_failure_details = failure.error.json()
        except NotImplementedError:
            new_failure_details = None
    elif failure.stage:
        new_code = "build-failed-stage-%s" % failure.stage
        new_failure_details = None
    else:
        new_code = "build-failed"
        new_failure_details = None
    new_description = failure.description
    new_phase = failure.phase
    return (new_code, new_description, new_phase, new_failure_details)


def process_build_log(logf):
    lines = [line.decode('utf-8', 'replace') for line in logf]
    match, problem = find_build_failure_description(lines)
    if problem:
        new_code = problem.kind
        try:
            new_failure_details = problem.json()
        except NotImplementedError:
            new_failure_details = None
    else:
        new_code = "build-failed"
        new_failure_details = None
    if match:
        new_description = str(match.line)
    elif problem:
        new_description = str(problem)
    else:
        new_description = "Build failed"
    return (new_code, new_description, "build", new_failure_details)


def process_dist_log(logf):
    lines = [line.decode('utf-8', 'replace') for line in logf]
    problem = find_build_failure_description(lines)[1]
    if problem is None:
        new_code = 'dist-command-failed'
        new_description = "Dist command failed"
        new_failure_details = None
    else:
        if problem.is_global:
            new_code = problem.kind
        else:
            new_code = 'dist-' + problem.kind
        new_description = str(problem)
        try:
            new_failure_details = problem.json()
        except NotImplementedError:
            new_failure_details = None
    new_phase = None
    return (new_code, new_description, new_phase, new_failure_details)


async def reprocess_run_logs(
        db, logfile_manager, *, codebase: str,
        campaign: str, log_id: str, command: str,
        change_set: Optional[str], duration: timedelta,
        result_code: str, description: str, failure_details,
        process_fns, dry_run: bool = False,
        reschedule: bool = False, log_timeout: Optional[int] = None):
    """Reprocess run logs.
    """
    if result_code in ('dist-no-tarball', ):
        return
    for prefix, logname, fn in process_fns:
        if not result_code.startswith(prefix):
            continue
        try:
            logf = await logfile_manager.get_log(
                codebase, log_id, logname, timeout=log_timeout
            )
        except FileNotFoundError:
            return
        else:
            (new_code, new_description, new_phase,
             new_failure_details) = fn(logf)
            break
    else:
        return

    if new_code != result_code or description != new_description or failure_details != new_failure_details:
        logging.info(
            "%s/%s: Updated %r, %r ⇒ %r, %r %r",
            codebase,
            log_id,
            result_code,
            description,
            new_code,
            new_description,
            new_phase
        )
        if not dry_run:
            async with db.acquire() as conn:
                await conn.execute(
                    "UPDATE run SET result_code = $1, description = $2, failure_details = $3 WHERE id = $4",
                    new_code,
                    new_description,
                    new_failure_details,
                    log_id,
                )
                if reschedule and new_code != result_code:
                    await do_schedule(
                        conn,
                        campaign=campaign,
                        change_set=change_set,
                        codebase=codebase,
                        estimated_duration=duration,
                        requestor="reprocess-build-results",
                        bucket="reschedule",
                    )
        return (new_code, new_description, new_failure_details)
    return
