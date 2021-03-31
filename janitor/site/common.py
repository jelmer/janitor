#!/usr/bin/python3

from aiohttp import ClientConnectorError
from aiohttp import web
import asyncpg
from functools import partial
from typing import Optional, List, Tuple, AsyncIterable
import urllib.parse

from janitor import state
from janitor.site import (
    get_archive_diff,
    BuildDiffUnavailable,
    get_vcs_type,
    DebdiffRetrievalError,
    tracker_url,
)


async def get_previous_runs(
        conn: asyncpg.Connection, package: str, suite: str):
    return await conn.fetch(
        """
SELECT
  id,
  start_time,
  finish_time,
  finish_time - start_time AS duration,
  description,
  package,
  result_code
FROM
  run
WHERE
  package = $1 AND suite = $2
ORDER BY start_time DESC
""",
        package,
        suite,
    )


async def get_candidate(conn: asyncpg.Connection, package, suite):
    return await conn.fetchrow(
        "SELECT context, value, success_chance FROM candidate "
        "WHERE package = $1 AND suite = $2",
        package,
        suite,
    )


async def iter_candidates(
    conn: asyncpg.Connection,
    packages: Optional[List[str]] = None,
    suite: Optional[str] = None):
    query = """
SELECT
  candidate.package AS package,
  candidate.suite AS suite,
  candidate.context AS context,
  candidate.value AS value,
  candidate.success_chance AS success_chance
FROM candidate
INNER JOIN package on package.name = candidate.package
WHERE NOT package.removed
"""
    args = []
    if suite is not None and packages is not None:
        query += " AND package.name = ANY($1::text[]) AND suite = $2"
        args.extend([packages, suite])
    elif suite is not None:
        query += " AND suite = $1"
        args.append(suite)
    elif packages is not None:
        query += " AND package.name = ANY($1::text[])"
        args.append(packages)
    return await conn.fetch(query, *args)


async def get_last_unabsorbed_run(
        conn: asyncpg.Connection, package: str, suite: str):
    args = []
    query = """
SELECT
  id,
  command,
  start_time,
  finish_time,
  description,
  package,
  debian_build.version AS build_version,
  debian_build.distribution AS build_distribution,
  debian_build.lintian_result AS lintian_result,
  debian_build.binary_packages AS binary_packages,
  result_code,
  branch_name,
  main_branch_revision,
  revision,
  context,
  result,
  suite,
  instigated_context,
  branch_url,
  logfilenames,
  review_status,
  review_comment,
  worker,
  array(SELECT row(role, remote_name, base_revision,
   revision) FROM new_result_branch WHERE run_id = id) AS result_branches,
  result_tags
FROM
  last_unabsorbed_runs
LEFT JOIN debian_build ON last_unabsorbed_runs.id = debian_build.run_id
WHERE package = $1 AND suite = $2
ORDER BY package, suite DESC, start_time DESC
LIMIT 1
"""
    args = [package, suite]
    return await conn.fetchrow(query, *args)


async def get_run(conn: asyncpg.Connection, run_id):
    query = """
SELECT
    id, command, start_time, finish_time, description, package,
    debian_build.version AS build_version,
    debian_build.distribution AS build_distribution,
    debian_build.lintian_result AS lintian_result,
    debian_build.binary_packages AS binary_packages,
    result_code,
    branch_name, main_branch_revision, revision, context, result, suite,
    instigated_context, branch_url, logfilenames, review_status,
    review_comment, worker,
    array(SELECT row(role, remote_name, base_revision,
     revision) FROM new_result_branch WHERE run_id = id) AS result_branches,
    result_tags
FROM
    run
LEFT JOIN
    debian_build ON debian_build.run_id = run.id
WHERE id = $1
"""
    return await conn.fetchrow(query, run_id)


async def generate_pkg_context(
    db, config, suite, policy, client, differ_url, vcs_store_url, package, run_id=None
):
    async with db.acquire() as conn:
        package = await conn.fetchrow("""\
SELECT name, maintainer_email, uploader_emails, removed, vcs_url, vcs_browse, vcswatch_version, update_changelog AS changelog_policy, publish AS publish_policy
FROM package
LEFT JOIN policy ON package.name = policy.package AND suite = $2
WHERE name = $1""", package, suite)
        if package is None:
            raise web.HTTPNotFound(text='no such package: %s' % package)
        if run_id is not None:
            run = await get_run(conn, run_id)
            if not run:
                raise web.HTTPNotFound(text='no such run: %s' % run_id)
            merge_proposals = []
        else:
            run = await get_last_unabsorbed_run(conn, package['name'], suite)
            merge_proposals = await conn.fetch("""\
SELECT
    DISTINCT ON (merge_proposal.url)
    merge_proposal.url AS url, merge_proposal.status AS status
FROM
    merge_proposal
LEFT JOIN run
ON merge_proposal.revision = run.revision AND run.result_code = 'success'
WHERE run.package = $1 AND run.suite = $2
""", package['name'], suite)
        if run is None:
            # No runs recorded
            run_id = None
            unchanged_run = None
        else:
            run_id = run['id']
            if run['main_branch_revision']:
                unchanged_run = await state.get_unchanged_run(
                    conn, run['package'], run['main_branch_revision']
                )
            else:
                unchanged_run = None

        candidate = await get_candidate(conn, package['name'], suite)
        if candidate is not None:
            (candidate_context, candidate_value, candidate_success_chance) = candidate
        else:
            candidate_context = None
            candidate_value = None
            candidate_success_chance = None
        previous_runs = await get_previous_runs(conn, package['name'], suite)
        (queue_position, queue_wait_time) = await state.get_queue_position(
            conn, suite, package['name']
        )

    async def show_diff(role):
        try:
            (remote_name, base_revid, revid) = state.get_result_branch(run['result_branches'], role)
        except KeyError:
            return "no result branch with role %s" % role
        if base_revid == revid:
            return ""
        url = urllib.parse.urljoin(vcs_store_url, "diff/%s/%s" % (run['id'], role))
        try:
            async with client.get(url) as resp:
                if resp.status == 200:
                    return (await resp.read()).decode("utf-8", "replace")
                else:
                    return "Unable to retrieve diff; error %d" % resp.status
        except ClientConnectorError as e:
            return "Unable to retrieve diff; error %s" % e

    async def show_debdiff():
        if not run['build_version']:
            return ""
        if not unchanged_run or not unchanged_run.build_version:
            return ""
        try:
            debdiff, content_type = await get_archive_diff(
                client,
                differ_url,
                run['id'],
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

    async def vcs_type():
        return await get_vcs_type(client, vcs_store_url, run['package'])

    kwargs = {}
    if run:
        kwargs.update(run)
    kwargs.update({
        "package": package['name'],
        "unchanged_run": unchanged_run,
        "merge_proposals": merge_proposals,
        "maintainer_email": package['maintainer_email'],
        "uploader_emails": package['uploader_emails'],
        "removed": package['removed'],
        "vcs_url": package['vcs_url'],
        "vcs_type": vcs_type,
        "vcs_browse": package['vcs_browse'],
        "vcswatch_version": package['vcswatch_version'],
        "run_id": run_id,
        "suite": suite,
        "show_diff": show_diff,
        "show_debdiff": show_debdiff,
        "previous_runs": previous_runs,
        "run": run,
        "candidate_context": candidate_context,
        "candidate_success_chance": candidate_success_chance,
        "candidate_value": candidate_value,
        "queue_position": queue_position,
        "queue_wait_time": queue_wait_time,
        "publish_policy": package['publish_policy'],
        "changelog_policy": package['changelog_policy'],
        "tracker_url": partial(tracker_url, config),
    })
    return kwargs


async def generate_candidates(db, suite):
    candidates = []
    async with db.acquire() as conn:
        for row in await iter_candidates(conn, suite=suite):
            candidates.append((row['package'], row['value']))
        candidates.sort(key=lambda x: x[1], reverse=True)
    return {"candidates": candidates, "suite": suite}
