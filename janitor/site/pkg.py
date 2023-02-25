#!/usr/bin/python3
# Copyright (C) 2021 Jelmer Vernooij <jelmer@jelmer.uk>
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
from datetime import datetime
from io import BytesIO
from typing import Dict, List, Optional

import asyncpg
from aiohttp import ClientConnectorError, ClientResponseError, ClientTimeout
from breezy.forge import UnsupportedForge, get_forge_by_hostname
from breezy.revision import NULL_REVISION
from buildlog_consultant.sbuild import (SbuildLog,
                                        find_build_failure_description,
                                        worker_failure_from_sbuild_log)
from ognibuild.build import BUILD_LOG_FILENAME
from ognibuild.dist import DIST_LOG_FILENAME
from yarl import URL

from janitor import state
from janitor.logs import LogRetrievalError
from janitor.queue import Queue
from janitor.site import (BuildDiffUnavailable, DebdiffRetrievalError,
                          get_archive_diff)

from ..config import get_campaign_config
from ..vcs import VcsManager
from .common import get_unchanged_run

FAIL_BUILD_LOG_LEN = 15

WORKER_LOG_FILENAME = "worker.log"
CODEMOD_LOG_FILENAME = "codemod.log"


def find_build_log_failure(logf, length):
    sbuildlog = SbuildLog.parse(logf)
    linecount = sbuildlog.sections[-1].offsets[1]
    failure = worker_failure_from_sbuild_log(sbuildlog)

    if failure.match and failure.section:
        abs_offset = failure.section.offsets[0] + failure.match.lineno
        include_lines = (
            max(1, abs_offset - length // 2),
            abs_offset + min(length // 2, len(failure.section.lines)),
        )
        highlight_lines = [abs_offset]
        return (linecount, include_lines, highlight_lines)

    if failure.match:
        include_lines = (
            max(1, failure.match.lineno - length // 2),
            failure.match.lineno + min(length // 2, linecount),
        )
        highlight_lines = [failure.match.lineno]
        return (linecount, include_lines, highlight_lines)

    if failure.section:
        include_lines = (max(1, failure.section.offsets[1] - length), failure.section.offsets[1])
    elif length < linecount:
        include_lines = (linecount - length, None)
    else:
        include_lines = (1, linecount)

    return (linecount, include_lines, [])


def find_dist_log_failure(logf, length):
    lines = [line.decode('utf-8', 'replace') for line in logf.readlines()]
    match, unused_err = find_build_failure_description(lines)
    if match is not None:
        highlight_lines = getattr(match, 'linenos', None)
    else:
        highlight_lines = None

    include_lines = (max(1, len(lines) - length), len(lines),)

    return (len(lines), include_lines, highlight_lines)


def in_line_boundaries(i, boundaries):
    if boundaries is None:
        return True
    if boundaries[0] is not None and i < boundaries[0]:
        return False
    if boundaries[1] is not None and i > boundaries[1]:
        return False
    return True


async def get_publish_history(
    conn: asyncpg.Connection, revision: bytes
) -> asyncpg.Record:
    return await conn.fetch(
        "select mode, merge_proposal_url, description, result_code, "
        "requestor, timestamp from publish where revision = $1 "
        "ORDER BY timestamp DESC",
        revision
    )


async def generate_run_file(
        db, client, config,
        differ_url: Optional[str], publisher_url: Optional[str], logfile_manager, run,
        vcs_managers: Dict[str, VcsManager], is_admin, span
):
    from ..schedule import estimate_success_probability_and_duration
    kwargs = {}
    kwargs["run"] = run
    kwargs["run_id"] = run['id']
    kwargs.update(run)
    async with db.acquire() as conn:
        if run['main_branch_revision']:
            with span.new_child('sql:unchanged-run'):
                kwargs["unchanged_run"] = await get_unchanged_run(
                    conn, run['codebase'],
                    run['main_branch_revision'].encode('utf-8')
                )
        with span.new_child('sql:queue-position'):
            queue = Queue(conn)
            (queue_position, queue_wait_time) = await queue.get_position(
                run['suite'], run['codebase']
            )
        with span.new_child('sql:publish-history'):
            publish_history: List[asyncpg.Record]
            if run['revision'] and run['result_code'] in ("success", "nothing-new-to-do"):
                publish_history = await get_publish_history(conn, run['revision'])
            else:
                publish_history = []
        with span.new_child('sql:reviews'):
            kwargs['reviews'] = await conn.fetch(
                'SELECT verdict, comment, reviewer, reviewed_at '
                'FROM review WHERE run_id = $1',
                run['id'])
        with span.new_child('sql:success-probability'):
            kwargs["success_probability"], kwargs['estimated_duration'], kwargs["total_previous_runs"] = await estimate_success_probability_and_duration(
                conn, run['codebase'], run['suite'])
        with span.new_child('sql:followups'):
            kwargs['followups'] = await conn.fetch("""SELECT \
    candidate.codebase AS codebase,
    candidate.suite AS campaign
FROM followup
LEFT JOIN candidate ON candidate.id = followup.candidate
WHERE followup.origin = $1""", run['id'])

    kwargs["queue_wait_time"] = queue_wait_time
    kwargs["queue_position"] = queue_position
    kwargs["is_admin"] = is_admin
    kwargs["publish_history"] = publish_history

    async def publish_blockers():
        if publisher_url is None:
            return {}
        url = URL(publisher_url) / "blockers" / run['id']
        with span.new_child('publish-blockers'):
            try:
                async with client.get(url, raise_for_status=True,
                                      timeout=ClientTimeout(30)) as resp:
                    return await resp.json()
            except ClientResponseError as e:
                if e.status == 404:
                    return {}
                logging.warning(
                    "Unable to retrieve publish blockers for %s: %r",
                    run['id'], e)
                return {}
            except ClientConnectorError as e:
                logging.warning(
                    "Unable to retrieve publish blockers for %s: %r",
                    run['id'], e)
                return {}

    kwargs["publish_blockers"] = publish_blockers

    async def show_diff(role):
        try:
            (remote_name, base_revid, revid) = state.get_result_branch(
                run['result_branches'], role)
        except KeyError:
            return "No branch with role %s" % role
        if base_revid == revid:
            return ""
        if run['vcs_type'] is None:
            return "Run not in VCS"
        if revid is None:
            return "Branch deleted"
        try:
            with span.new_child('vcs-diff'):
                diff = await vcs_managers[run['vcs_type']].get_diff(
                    run['codebase'],
                    base_revid.encode('utf-8') if base_revid is not None else NULL_REVISION,
                    revid.encode('utf-8'))
        except ClientResponseError as e:
            return "Unable to retrieve diff; error %d" % e.status
        except ClientConnectorError as e:
            return "Unable to retrieve diff; error %s" % e
        except NotImplementedError:
            return "Unable to retrieve diff; unsupported vcs"
        return diff.decode("utf-8", "replace")

    kwargs["show_diff"] = show_diff

    async def show_debdiff():
        if run['result_code'] != 'success':
            return ""
        unchanged_run = kwargs.get("unchanged_run")
        if not unchanged_run or unchanged_run['result_code'] != 'success':
            return ""
        try:
            with span.new_child('archive-diff'):
                debdiff, unused_content_type = await get_archive_diff(
                    client,
                    differ_url,
                    run['id'],
                    unchanged_run['id'],
                    kind="debdiff",
                    filter_boring=True,
                    accept="text/html",
                )
                return debdiff.decode("utf-8", "replace")
        except BuildDiffUnavailable:
            return ""
        except DebdiffRetrievalError as e:
            return "Error retrieving debdiff: %s" % e

    kwargs["show_debdiff"] = show_debdiff
    kwargs["max"] = max
    kwargs["suite"] = run['suite']
    try:
        kwargs["campaign"] = get_campaign_config(config, run['suite'])
    except KeyError:
        kwargs["campaign"] = None
    kwargs["resume_from"] = run['resume_from']
    kwargs['codemod_result'] = run['result']

    def read_file(f):
        return [line.decode("utf-8", "replace") for line in f.readlines()]

    kwargs["read_file"] = read_file

    kwargs["in_line_boundaries"] = in_line_boundaries

    cache_logs = {}

    async def _get_log(name):
        try:
            return (await logfile_manager.get_log(run['codebase'], run['id'], name)).read()
        except FileNotFoundError:
            return None
        except LogRetrievalError as e:
            return str(e).encode('utf-8')

    def has_log(name):
        return name in run['logfilenames']

    async def get_log(name):
        if name not in run['logfilenames']:
            log = None
        else:
            if name not in cache_logs:
                cache_logs[name] = await _get_log(name)
            log = cache_logs[name]
        if log is None:
            return BytesIO(b"Log file missing.")
        return BytesIO(log)

    if has_log(BUILD_LOG_FILENAME):
        kwargs["build_log_name"] = BUILD_LOG_FILENAME

    if has_log(WORKER_LOG_FILENAME):
        kwargs["worker_log_name"] = WORKER_LOG_FILENAME

    if has_log(CODEMOD_LOG_FILENAME):
        kwargs["codemod_log_name"] = CODEMOD_LOG_FILENAME

    if has_log(DIST_LOG_FILENAME):
        kwargs["dist_log_name"] = DIST_LOG_FILENAME

    kwargs["get_log"] = get_log

    if run.get('failure_stage'):
        if run['failure_stage'] == 'codemod/dist':
            primary_log = "dist"
        if run['failure_stage'].split('/')[0] == 'codemod':
            primary_log = "codemod"
        elif run['failure_stage'].split('/')[0] == 'build':
            primary_log = "build"
        else:
            primary_log = "worker"
    else:
        # Legacy runs
        if run['result_code'].startswith('worker-') or run['result_code'].startswith('result-'):
            primary_log = "worker"
        elif has_log(BUILD_LOG_FILENAME):
            primary_log = "build"
        elif has_log(DIST_LOG_FILENAME) and run['result_code'].startswith('dist-'):
            primary_log = "dist"
        elif has_log(WORKER_LOG_FILENAME):
            primary_log = "worker"
        else:
            primary_log = None

    if primary_log == "dist":
        logf = await get_log(DIST_LOG_FILENAME)
        line_count, include_lines, highlight_lines = find_dist_log_failure(
            logf, FAIL_BUILD_LOG_LEN
        )
        kwargs["dist_log_line_count"] = line_count
        kwargs["dist_log_include_lines"] = include_lines
        kwargs["dist_log_highlight_lines"] = highlight_lines
    elif primary_log == "build":
        kwargs["earlier_build_log_names"] = []
        i = 1
        while has_log(BUILD_LOG_FILENAME + ".%d" % i):
            log_name = "%s.%d" % (BUILD_LOG_FILENAME, i)
            kwargs["earlier_build_log_names"].append((i, log_name))
            i += 1

        logf = await get_log(BUILD_LOG_FILENAME)
        line_count, include_lines, highlight_lines = find_build_log_failure(
            logf, FAIL_BUILD_LOG_LEN
        )
        kwargs["build_log_line_count"] = line_count
        kwargs["build_log_include_lines"] = include_lines
        kwargs["build_log_highlight_lines"] = highlight_lines

    kwargs["primary_log"] = primary_log

    return kwargs


async def generate_done_list(
        db, campaign: Optional[str], since: Optional[datetime] = None):

    async with db.acquire() as conn:
        oldest = await conn.fetchval(
            "SELECT MIN(absorbed_at) FROM absorbed_runs WHERE campaign = $1",
            campaign)

        if since:
            orig_runs = await conn.fetch(
                "SELECT * FROM absorbed_runs "
                "WHERE absorbed_at >= $1 AND campaign = $2 "
                "ORDER BY absorbed_at DESC NULLS LAST", since, campaign)
        else:
            orig_runs = await conn.fetch(
                "SELECT * FROM absorbed_runs WHERE campaign = $1 "
                "ORDER BY absorbed_at DESC NULLS LAST", campaign)

    mp_user_url_resolver = MergeProposalUserUrlResolver()

    runs = []
    for orig_run in orig_runs:
        run = dict(orig_run)
        if not run['merged_by']:
            run['merged_by_url'] = None
        else:
            run['merged_by_url'] = mp_user_url_resolver.resolve(
                run['merge_proposal_url'], run['merged_by'])
        runs.append(run)

    return {
        "oldest": oldest, "runs": runs, "campaign": campaign,
        "since": since}


class MergeProposalUserUrlResolver(object):

    def __init__(self):
        self._forges = {}

    def resolve(self, url, user):
        hostname = URL(url).host
        if hostname is None:
            return None
        if hostname not in self._forges:
            try:
                self._forges[hostname] = get_forge_by_hostname(hostname)
            except UnsupportedForge:
                self._forges[hostname] = None
        if self._forges[hostname]:
            return self._forges[hostname].get_user_url(user)
        else:
            return None


async def generate_ready_list(
    db, suite: Optional[str], publish_status: Optional[str] = None
):
    async with db.acquire() as conn:
        query = 'SELECT codebase, suite, id, command, result FROM publish_ready'

        conditions = [
            "EXISTS (SELECT * FROM unnest(unpublished_branches) upb "
            "WHERE mode in "
            "('propose', 'attempt-push', 'push-derived', 'push') "
            "AND NOT EXISTS "
            "(SELECT FROM merge_proposal WHERE revision = upb.revision) "
            ")"]

        args = []
        if suite:
            args.append(suite)
            conditions.append('suite = $%d' % len(args))
        if publish_status:
            args.append(publish_status)
            conditions.append('publish_status = $%d' % len(args))

        query += " WHERE " + " AND ".join(conditions)

        query += " ORDER BY codebase ASC"

        runs = await conn.fetch(query, *args)
    return {"runs": runs, "suite": suite, "campaign": suite}
