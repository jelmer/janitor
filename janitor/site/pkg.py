#!/usr/bin/python3

from aiohttp import ClientConnectorError
from datetime import datetime
from functools import partial
from io import BytesIO
from typing import Optional, Tuple
import urllib.parse

import asyncpg

from janitor import state
from janitor.debian import state as debian_state
from buildlog_consultant.sbuild import (
    parse_sbuild_log,
    find_failed_stage,
    find_build_failure_description,
    find_install_deps_failure_description,
    SBUILD_FOCUS_SECTION,
    strip_build_tail,
)
from janitor.logs import LogRetrievalError
from janitor.site import (
    get_archive_diff,
    BuildDiffUnavailable,
    get_vcs_type,
    DebdiffRetrievalError,
    tracker_url,
)

FAIL_BUILD_LOG_LEN = 15

BUILD_LOG_NAME = "build.log"
WORKER_LOG_NAME = "worker.log"
DIST_LOG_NAME = "dist.log"


def find_build_log_failure(logf, length):
    offsets = {}
    linecount = 0
    paragraphs = {}
    for title, offset, lines in parse_sbuild_log(logf):
        if title is not None:
            title = title.lower()
        paragraphs[title] = lines
        linecount = max(offset[1], linecount)
        offsets[title] = offset
    highlight_lines = []
    include_lines = None
    failed_stage = find_failed_stage(paragraphs.get("summary", []))
    focus_section = SBUILD_FOCUS_SECTION.get(failed_stage)
    if focus_section not in paragraphs:
        focus_section = None
    if failed_stage == "install-deps":
        (focus_section, match, error) = find_install_deps_failure_description(
            paragraphs
        )
        if offset is not None:
            abs_offset = offsets[focus_section][0] + match.lineno
            include_lines = (
                max(1, abs_offset - length // 2),
                abs_offset + min(length // 2, len(lines)),
            )
            highlight_lines = [abs_offset]
            return (linecount, include_lines, highlight_lines)

    if focus_section:
        include_lines = (
            max(1, offsets[focus_section][1] - length),
            offsets[focus_section][1],
        )
    elif length < linecount:
        include_lines = (linecount - length, None)
    else:
        include_lines = (1, linecount)
    if focus_section == "build":
        lines = paragraphs.get(focus_section, [])
        lines, files = strip_build_tail(lines)
        include_lines = (
            max(1, offsets[focus_section][0] + len(lines) - length),
            offsets[focus_section][0] + len(lines),
        )
        match, unused_err = find_build_failure_description(lines)
        if match is not None:
            highlight_lines = [offsets[focus_section][0] + match.lineno]

    return (linecount, include_lines, highlight_lines)


def find_dist_log_failure(logf, length):
    lines = [line.decode('utf-8', 'replace') for line in logf.readlines()]
    match, unused_err = find_build_failure_description(lines)
    if match is not None:
        highlight_lines = [match.lineno]
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
) -> Tuple[str, Optional[str], str, str, str, datetime]:
    return await conn.fetch(
        "select mode, merge_proposal_url, description, result_code, "
        "requestor, timestamp from publish where revision = $1 "
        "ORDER BY timestamp DESC",
        revision.decode("utf-8"),
    )


async def generate_run_file(
    db, client, config, differ_url, logfile_manager, run, vcs_store_url, is_admin
):
    (start_time, finish_time) = run.times
    kwargs = {}
    kwargs["run"] = run
    kwargs["run_id"] = run.id
    kwargs["command"] = run.command
    kwargs["description"] = run.description
    kwargs["package"] = run.package
    kwargs["start_time"] = run.times[0]
    kwargs["finish_time"] = run.times[1]
    kwargs["build_version"] = run.build_version
    kwargs["build_distribution"] = run.build_distribution
    kwargs["result_code"] = run.result_code
    kwargs["result"] = run.result
    kwargs["revision"] = run.revision
    kwargs["branch_url"] = run.branch_url
    kwargs["tracker_url"] = partial(tracker_url, config)
    async with db.acquire() as conn:
        if run.main_branch_revision:
            kwargs["unchanged_run"] = await state.get_unchanged_run(
                conn, run.package, run.main_branch_revision
            )
        (queue_position, queue_wait_time) = await state.get_queue_position(
            conn, run.suite, run.package
        )
        package = await debian_state.get_package(conn, run.package)
        if run.revision and run.result_code in ("success", "nothing-new-to-do"):
            publish_history = await get_publish_history(conn, run.revision)
        else:
            publish_history = []
    kwargs["queue_wait_time"] = queue_wait_time
    kwargs["queue_position"] = queue_position
    kwargs["vcs_type"] = package.vcs_type
    kwargs["vcs_url"] = package.vcs_url
    kwargs["vcs_browse"] = package.vcs_browse
    kwargs["vcswatch_version"] = package.vcswatch_version
    kwargs["is_admin"] = is_admin
    kwargs["publish_history"] = publish_history

    async def show_diff(role):
        try:
            (remote_name, base_revid, revid) = run.get_result_branch(role)
        except KeyError:
            return "No branch with role %s" % role
        if base_revid == revid:
            return ""
        url = urllib.parse.urljoin(vcs_store_url, "diff/%s/%s" % (run.id, role))
        try:
            async with client.get(url) as resp:
                if resp.status == 200:
                    return (await resp.read()).decode("utf-8", "replace")
                else:
                    return "Unable to retrieve diff; error %d" % resp.status
        except ClientConnectorError as e:
            return "Unable to retrieve diff; error %s" % e

    kwargs["show_diff"] = show_diff

    async def show_debdiff():
        if not run.has_artifacts():
            return ""
        unchanged_run = kwargs.get("unchanged_run")
        if not unchanged_run or not unchanged_run.has_artifacts():
            return ""
        try:
            debdiff, unused_content_type = await get_archive_diff(
                client,
                differ_url,
                run,
                unchanged_run,
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
    kwargs["suite"] = run.suite

    def read_file(f):
        return [line.decode("utf-8", "replace") for line in f.readlines()]

    kwargs["read_file"] = read_file

    async def vcs_type():
        return await get_vcs_type(client, vcs_store_url, run.package)

    kwargs["vcs_type"] = vcs_type
    kwargs["in_line_boundaries"] = in_line_boundaries

    cached_logs = {}

    async def _cache_log(name):
        try:
            cached_logs[name] = (
                await logfile_manager.get_log(run.package, run.id, name)
            ).read()
        except FileNotFoundError:
            cached_logs[name] = None
        except LogRetrievalError:
            cached_logs[name] = None

    def has_log(name):
        return name in run.logfilenames

    async def get_log(name):
        if name not in cached_logs:
            await _cache_log(name)
        if cached_logs[name] is None:
            return BytesIO(b"Log file missing or inaccessible.")
        return BytesIO(cached_logs[name])

    if has_log(BUILD_LOG_NAME):
        kwargs["build_log_name"] = BUILD_LOG_NAME

    if has_log(WORKER_LOG_NAME):
        kwargs["worker_log_name"] = WORKER_LOG_NAME

    if has_log(DIST_LOG_NAME):
        kwargs["dist_log_name"] = DIST_LOG_NAME

    kwargs["get_log"] = get_log
    if run.result_code.startswith('worker-'):
        kwargs["primary_log"] = "worker"
    elif has_log(BUILD_LOG_NAME):
        kwargs["earlier_build_log_names"] = []
        i = 1
        while has_log(BUILD_LOG_NAME + ".%d" % i):
            log_name = "%s.%d" % (BUILD_LOG_NAME, i)
            kwargs["earlier_build_log_names"].append((i, log_name))
            i += 1

        logf = await get_log(BUILD_LOG_NAME)
        line_count, include_lines, highlight_lines = find_build_log_failure(
            logf, FAIL_BUILD_LOG_LEN
        )
        kwargs["build_log_line_count"] = line_count
        kwargs["build_log_include_lines"] = include_lines
        kwargs["build_log_highlight_lines"] = highlight_lines
        kwargs["primary_log"] = "build"
    elif has_log(DIST_LOG_NAME) and run.result_code.startswith('dist-'):
        kwargs["primary_log"] = "dist"
        logf = await get_log(DIST_LOG_NAME)
        line_count, include_lines, highlight_lines = find_dist_log_failure(
            logf, FAIL_BUILD_LOG_LEN
        )
        kwargs["dist_log_line_count"] = line_count
        kwargs["dist_log_include_lines"] = include_lines
        kwargs["dist_log_highlight_lines"] = highlight_lines
    else:
        kwargs["primary_log"] = "worker"

    return kwargs


async def generate_pkg_file(db, config, package, merge_proposals, runs, available_suites):
    kwargs = {}
    kwargs["package"] = package.name
    kwargs["vcswatch_status"] = package.vcswatch_status
    kwargs["maintainer_email"] = package.maintainer_email
    kwargs["vcs_type"] = package.vcs_type
    kwargs["vcs_url"] = package.vcs_url
    kwargs["vcs_browse"] = package.vcs_browse
    kwargs["merge_proposals"] = merge_proposals
    kwargs["runs"] = [run async for run in runs]
    kwargs["removed"] = package.removed
    kwargs["tracker_url"] = partial(tracker_url, config)
    kwargs["available_suites"] = available_suites
    async with db.acquire() as conn:
        kwargs["candidates"] = {
            suite: (context, value, success_chance)
            for (
                package,
                suite,
                context,
                value,
                success_chance,
            ) in await debian_state.iter_candidates(conn, packages=[package.name])
        }
    return kwargs


async def generate_pkg_list(packages):
    return {"packages": [name for (name, maintainer) in packages]}


async def generate_maintainer_list(packages):
    by_maintainer = {}
    for name, maintainer in packages:
        by_maintainer.setdefault(maintainer, []).append(name)
    return {"by_maintainer": by_maintainer}


async def generate_ready_list(
    db, suite: Optional[str], review_status: Optional[str] = None
):
    async with db.acquire() as conn:
        runs = [
            row
            async for row in state.iter_publish_ready(
                conn,
                suites=([suite] if suite else None),
                review_status=review_status,
                publishable_only=True,
            )
        ]
    return {"runs": runs, "suite": suite}
