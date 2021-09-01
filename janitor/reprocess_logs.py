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

from __future__ import absolute_import

import logging

import silver_platter  # noqa: E402, F401
from buildlog_consultant.common import find_build_failure_description  # noqa: E402
from buildlog_consultant.sbuild import worker_failure_from_sbuild_log  # noqa: E402
from janitor.schedule import do_schedule  # noqa: E402


async def reprocess_run_logs(
        db, logfile_manager, package, suite, log_id, command, duration,
        result_code, description, failure_details, dry_run=False,
        reschedule=False, log_timeout=None):
    if result_code in ('dist-no-tarball', ):
        return
    if result_code.startswith('dist-'):
        logname = 'dist.log'
    else:
        logname = 'build.log'
    try:
        logf = await logfile_manager.get_log(
            package, log_id, logname, timeout=log_timeout
        )
    except FileNotFoundError:
        return

    if logname == 'build.log':
        failure = worker_failure_from_sbuild_log(logf)
        if failure.error:
            if failure.stage and not failure.error.is_global:
                new_code = "%s-%s" % (failure.stage, failure.error.kind)
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
    elif logname == 'dist.log':
        lines = [line.decode('utf-8', 'replace') for line in logf]
        problem = find_build_failure_description(lines)[1]
        if problem is None:
            new_code = 'dist-command-failed'
            new_description = description
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

    if new_code != result_code or description != new_description or failure_details != new_failure_details:
        logging.info(
            "%s/%s: Updated %r, %r => %r, %r %r",
            package,
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
                        package,
                        suite,
                        estimated_duration=duration,
                        requestor="reprocess-build-results",
                        bucket="reschedule",
                    )
