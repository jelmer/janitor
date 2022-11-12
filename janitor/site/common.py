#!/usr/bin/python3

from aiohttp import ClientConnectorError, ClientResponseError
from aiohttp import web
import aiohttp_jinja2
import asyncpg
from typing import Optional, List, Dict, Any

from breezy.revision import NULL_REVISION

from janitor import state, splitout_env
from janitor.config import get_campaign_config
from janitor.queue import Queue
from janitor.site import (
    get_archive_diff,
    BuildDiffUnavailable,
    DebdiffRetrievalError,
    update_vars_from_request,
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
  result_code,
  failure_transient
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
  main_branch_revision,
  revision,
  context,
  result,
  suite,
  instigated_context,
  vcs_type,
  branch_url,
  logfilenames,
  review_status,
  last_unabsorbed_runs.worker AS worker_name,
  worker.link AS worker_link,
  array(SELECT row(role, remote_name, base_revision,
   revision) FROM new_result_branch WHERE run_id = id) AS result_branches,
  result_tags,
  change_set,
  failure_stage,
  failure_transient
FROM
  last_unabsorbed_runs
LEFT JOIN worker ON worker.name = last_unabsorbed_runs.worker
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
    main_branch_revision, revision, context, result, suite,
    instigated_context, vcs_type, branch_url, logfilenames, review_status,
    run.worker AS worker_name,
    worker.link AS worker_link,
    array(SELECT row(role, remote_name, base_revision,
     revision) FROM new_result_branch WHERE run_id = id) AS result_branches,
    result_tags,
    resume_from,
    change_set,
    failure_stage,
    failure_transient
FROM
    run
LEFT JOIN worker ON worker.name = run.worker
LEFT JOIN
    debian_build ON debian_build.run_id = run.id
WHERE id = $1
"""
    return await conn.fetchrow(query, run_id)


async def get_unchanged_run(
        conn: asyncpg.Connection, package: str, main_branch_revision: bytes):
    query = """
SELECT
    id, command, start_time, finish_time, description, package,
    debian_build.version AS build_version,
    debian_build.distribution AS build_distribution, result_code, value,
    main_branch_revision, revision, context, result, suite,
    instigated_context, vcs_type, branch_url, logfilenames, review_status,
    worker,
    array(SELECT row(role, remote_name, base_revision, revision) FROM
     new_result_branch WHERE run_id = id) AS result_branches,
    result_tags, target_branch_url, change_set AS change_set,
    failure_transient AS failure_transient, failure_stage
FROM
    last_runs
LEFT JOIN
    debian_build ON debian_build.run_id = last_runs.id
WHERE
    suite in ('control', 'unchanged') AND revision = $1 AND
    package = $2 AND
    result_code = 'success' AND
    change_set IS NULL
ORDER BY finish_time DESC
"""
    return await conn.fetchrow(
        query, main_branch_revision.decode('utf-8'), package)


async def generate_pkg_context(
    db, config, suite, client, differ_url, vcs_managers, package, span, run_id=None
):
    async with db.acquire() as conn:
        with span.new_child('sql:package'):
            package = await conn.fetchrow("""\
SELECT package.*, named_publish_policy.per_branch_policy AS publish_policy
FROM package
LEFT JOIN candidate ON package.name = candidate.package AND candidate.suite = $2
LEFT JOIN named_publish_policy ON named_publish_policy.name = candidate.publish_policy
WHERE package.name = $1""", package, suite)
        if package is None:
            raise web.HTTPNotFound(text='no such package: %s' % package)
        if run_id is not None:
            with span.new_child('sql:run'):
                run = await get_run(conn, run_id)
            if not run:
                raise web.HTTPNotFound(text='no such run: %s' % run_id)
            merge_proposals = []
        else:
            with span.new_child('sql:unchanged-run'):
                run = await get_last_unabsorbed_run(conn, package['name'], suite)
            with span.new_child('sql:merge-proposals'):
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
                with span.new_child('sql:unchanged-run'):
                    unchanged_run = await get_unchanged_run(
                        conn, run['package'],
                        run['main_branch_revision'].encode('utf-8'))
            else:
                unchanged_run = None

        with span.new_child('sql:candidate'):
            candidate = await get_candidate(conn, package['name'], suite)
        if candidate is not None:
            (candidate_context, candidate_value, candidate_success_chance) = candidate
        else:
            candidate_context = None
            candidate_value = None
            candidate_success_chance = None
        with span.new_child('sql:previous-runs'):
            previous_runs = await get_previous_runs(conn, package['name'], suite)
        with span.new_child('sql:queue-position'):
            queue = Queue(conn)
            (queue_position, queue_wait_time) = await queue.get_position(
                suite, package['name'])
        if run_id:
            with span.new_child('sql:reviews'):
                reviews = await conn.fetch(
                    'SELECT * FROM review WHERE run_id = $1 '
                    'ORDER BY reviewed_at ASC', run_id)
        else:
            reviews = None

    async def show_diff(role):
        try:
            (remote_name, base_revid, revid) = state.get_result_branch(run['result_branches'], role)
        except KeyError:
            return "no result branch with role %s" % role
        if base_revid == revid:
            return ""
        if run['vcs_type'] is None:
            return "not in a VCS"
        if revid is None:
            return "Branch deleted"
        try:
            with span.new_child('vcs-diff'):
                diff = await vcs_managers[run['vcs_type']].get_diff(
                    run['package'],
                    base_revid.encode('utf-8')
                    if base_revid is not None else NULL_REVISION,
                    revid.encode('utf-8'))
                return diff.decode("utf-8", "replace")
        except ClientResponseError as e:
            return "Unable to retrieve diff; error %d" % e.status
        except ClientConnectorError as e:
            return "Unable to retrieve diff; error %s" % e

    async def show_debdiff():
        if not run['build_version']:
            return ""
        if not unchanged_run or not unchanged_run['build_version']:
            return ""
        try:
            with span.new_child('archive-diff'):
                debdiff, content_type = await get_archive_diff(
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

    kwargs = {}
    kwargs.update([(k, v) for (k, v) in package.items() if k != 'name'])

    if run:
        kwargs.update(run)
        env, plain_command = splitout_env(run['command'])
        kwargs['env'] = env
        kwargs['plain_command'] = plain_command
    else:
        env = {}

    campaign = get_campaign_config(config, suite)

    kwargs.update({
        "package": package['name'],
        "reviews": reviews,
        "unchanged_run": unchanged_run,
        "merge_proposals": merge_proposals,
        "run_id": run_id,
        "suite": suite,
        "campaign": campaign,
        "show_diff": show_diff,
        "show_debdiff": show_debdiff,
        "previous_runs": previous_runs,
        "run": run,
        "candidate_context": candidate_context,
        "candidate_success_chance": candidate_success_chance,
        "candidate_value": candidate_value,
        "queue_position": queue_position,
        "queue_wait_time": queue_wait_time,
        "changelog_policy": env.get('DEB_UPDATE_CHANGELOG', 'auto'),
        "config": config,
    })
    return kwargs


async def generate_candidates(db, suite):
    candidates = []
    async with db.acquire() as conn:
        for row in await iter_candidates(conn, suite=suite):
            candidates.append((row['package'], row['value']))
        candidates.sort(key=lambda x: x[1], reverse=True)
    return {"candidates": candidates, "suite": suite}


def html_template(template_name, headers={}):
    def decorator(fn):
        async def handle(request):
            vs = await fn(request)
            if isinstance(vs, web.Response):
                return vs
            text = await render_template_for_request(
                template_name, request, vs)
            return web.Response(
                content_type="text/html", text=text, headers=headers)

        return handle

    return decorator


async def render_template_for_request(templatename, request, vs: Dict[str, Any]):
    update_vars_from_request(vs, request)
    return await aiohttp_jinja2.render_string_async(templatename, request, vs)
