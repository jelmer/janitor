#!/usr/bin/python3

from aiohttp_apispec import (
    docs,
    )


import aiozipkin
import asyncpg
from typing import List, Dict
from .common import generate_pkg_context, iter_candidates
from lintian_brush import (
    available_lintian_fixers,
)
from lintian_brush.lintian_overrides import load_renamed_tags

from .common import html_template


SUITE = "lintian-fixes"

renamed_tags = load_renamed_tags()


async def generate_pkg_file(
    db, config, policy, client, differ_url, vcs_manager, package, span, run_id=None
):
    kwargs = await generate_pkg_context(
        db,
        config,
        SUITE,
        policy,
        client,
        differ_url,
        vcs_manager,
        package,
        span,
        run_id=run_id,
    )
    run = kwargs["run"]
    if run and run['result']:
        applied = run['result'].get("applied", [])
    else:
        applied = []
    fixed_tags = set()
    if isinstance(applied, dict):
        applied = [applied]
    for applied in applied:
        for tag in applied.get("fixed_lintian_tags", []):
            fixed_tags.add(tag)
    kwargs["fixed_tags"] = fixed_tags
    kwargs["candidate_tags"] = (
        set(kwargs["candidate_context"].split(" "))
        if kwargs["candidate_context"]
        else set()
    )
    return kwargs


async def iter_lintian_tags(conn):
    return await conn.fetch(
        """
select tag, count(tag) from (
    select
      json_array_elements(
        json_array_elements(
          result->'applied')->'fixed_lintian_tags') #>> '{}' as tag
    from
      last_runs
    where
      suite = 'lintian-fixes'
   ) as bypackage group by 1 order by 2
 desc
"""
    )


async def iter_last_successes_by_lintian_tag(conn: asyncpg.Connection, tags: List[str]):
    return await conn.fetch(
        """
select distinct on (package) * from (
select
  package,
  command,
  result_code,
  context,
  start_time,
  id,
  (json_array_elements(
     json_array_elements(
       result->'applied')->'fixed_lintian_tags') #>> '{}') as tag
from
  run
where
  suite = 'lintian-fixes' and
  result_code = 'success'
) as package where tag = ANY($1::text[]) order by package, start_time desc
""",
        tags,
    )


async def generate_candidates(db):
    supported_tags = set()
    for fixer in available_lintian_fixers():
        supported_tags.update(fixer.lintian_tags)
    async with db.acquire() as conn:
        candidates = [
            (row['package'], row['context'].split(" "), row['value'])
            for row in await iter_candidates(conn, suite=SUITE)
        ]
        candidates.sort()
    return {
        "supported_tags": supported_tags,
        "candidates": candidates,
    }


async def generate_developer_table_page(db, developer):
    async with db.acquire() as conn:
        packages = [
            row['name']
            for row in await conn.fetch(
                "SELECT name FROM package WHERE "
                "maintainer_email = $1 OR $1 = any(uploader_emails) AND NOT removed",
                developer)
        ]
        open_proposals = {}
        for row in await conn.fetch("""
SELECT
    DISTINCT ON (merge_proposal.url)
    merge_proposal.package AS package, merge_proposal.url AS url
FROM
    merge_proposal
LEFT JOIN run
ON merge_proposal.revision = run.revision AND run.result_code = 'success'
WHERE status = 'open' AND run.suite = $1
""", SUITE):
            open_proposals[row['package']] = row['url']
        candidates = {}
        for row in await iter_candidates(conn, packages=packages, suite=SUITE):
            candidates[row['package']] = row['context'].split(" ")
        runs = {}
        query = """
SELECT DISTINCT ON (package)
  id,
  command,
  start_time,
  finish_time,
  description,
  package,
  result_code,
  branch_name,
  main_branch_revision,
  revision,
  context,
  result,
  instigated_context,
  branch_url,
  array(SELECT row(role, remote_name, base_revision,
   revision) FROM new_result_branch WHERE run_id = id) AS result_branches,
  result_tags,
  suite
FROM
  last_unabsorbed_runs
WHERE suite = $1 AND package = ANY($2::text[])
ORDER BY package, suite, start_time DESC
"""
        for run in await conn.fetch(query, SUITE, packages):
            runs[run['package']] = run
        queue_data = {
            row['package']: (row['position'], row['wait_time'])
            for row in await conn.fetch(
                "SELECT package, position, wait_time FROM queue_positions "
                "WHERE package = ANY($1::text[]) AND suite = $2",
                packages, SUITE)
        }

    by_package = {}
    for package in packages:
        run = runs.get(package)
        fixed = set()
        unfixed = set()
        if run and run['result']:
            applied = run['result'].get("applied")
            if isinstance(applied, dict):
                applied = [applied]
            for applied in applied:
                for tag in applied.get("fixed_lintian_tags", []):
                    fixed.add(tag)
        if run and run['instigated_context']:
            for tag in run['instigated_context'].split(" "):
                unfixed.add(tag)
        unfixed -= fixed
        open_proposal = open_proposals.get(package)
        package_candidates = set(candidates.get(package, []))
        if open_proposal:
            status = "proposal"
        elif run and run['result'] and run['result_code'] in ("success", "nothing-new-to-do"):
            status = "unabsorbed"
        elif run and run['result_code'] != "nothing-to-do":
            status = "error"
        elif package_candidates:
            status = "candidates"
        else:
            status = "nothing-to-do"

        by_package[package] = (
            run,
            package_candidates,
            fixed,
            unfixed,
            open_proposal,
            status,
            queue_data.get(package, (None, None)),
        )

    return {
        "packages": packages,
        "by_package": by_package,
        "suite": SUITE,
        "developer": developer,
    }


async def iter_lintian_brush_fixer_failures(conn: asyncpg.Connection, fixer):
    query = """
select id, finish_time, package, result->'failed'->$1 FROM last_runs
where
  suite = 'lintian-fixes' and (result->'failed')::jsonb?$1
order by finish_time desc
"""
    return await conn.fetch(query, fixer)


async def iter_failed_lintian_fixers(db):
    query = """
select json_object_keys((result->'failed')::json), count(*) from last_runs
where
  suite = 'lintian-fixes' and
  json_typeof((result->'failed')::json) = 'object' group by 1 order by 2 desc
"""
    async with db.acquire() as conn:
        for row in await conn.fetch(query):
            yield row


async def iter_lintian_fixes_regressions(conn):
    query = """
SELECT l.package, l.id, u.id, l.result_code FROM last_runs l
   INNER JOIN last_runs u ON l.main_branch_revision = u.main_branch_revision
   WHERE
    l.suite = 'lintian-fixes' AND
    u.suite = 'unchanged' AND
    l.result_code NOT IN ('success', 'nothing-to-do', 'nothing-new-to-do') AND
    u.result_code = 'success'
"""
    return await conn.fetch(query)


async def iter_lintian_fixes_counts(conn):
    per_tag = {}
    for (tag, absorbed, unabsorbed, total) in await conn.fetch(
        """
SELECT
   absorbed.tag,
   COALESCE(absorbed.cnt, 0),
   COALESCE(unabsorbed.cnt, 0),
   COALESCE(absorbed.cnt, 0)+COALESCE(unabsorbed.cnt, 0)
FROM (
    SELECT UNNEST(fixed_lintian_tags) AS tag, COUNT(*) AS cnt
    FROM absorbed_lintian_fixes group by 1 order by 2 desc
    ) AS absorbed
LEFT JOIN (
    SELECT UNNEST(fixed_lintian_tags) AS tag, COUNT(*) AS cnt
    FROM last_unabsorbed_lintian_fixes group by 1 order by 2 desc
    ) AS unabsorbed
ON absorbed.tag = unabsorbed.tag
"""
    ):
        canonical_name = renamed_tags.get(tag, tag)
        per_tag.setdefault(canonical_name, (0, 0, 0))
        per_tag[canonical_name] = (
            per_tag[canonical_name][0] + absorbed,
            per_tag[canonical_name][1] + unabsorbed,
            per_tag[canonical_name][2] + total,
        )
    entries = [
        (tag, absorbed, unabsorbed, total)
        for (tag, (absorbed, unabsorbed, total)) in per_tag.items()
    ]
    entries.sort(key=lambda v: v[3], reverse=True)
    return entries


@html_template(
    "lintian-fixes/start.html", headers={"Cache-Control": "max-age=3600"}
)
async def handle_lintian_fixes(request):
    import lintian_brush
    from lintian_brush.__main__ import DEFAULT_ADDON_FIXERS

    return {
        "SUITE": SUITE,
        "lintian_brush": lintian_brush,
        "ADDON_FIXERS": DEFAULT_ADDON_FIXERS,
    }


@html_template(
    "lintian-fixes/package.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_lintian_fixes_pkg(request):
    # TODO(jelmer): Handle Accept: text/diff
    pkg = request.match_info["pkg"]
    run_id = request.match_info.get("run_id")
    return await generate_pkg_file(
        request.app.database,
        request.app['config'],
        request.app.policy,
        request.app.http_client_session,
        request.app.differ_url,
        request.app['vcs_manager'],
        pkg,
        aiozipkin.request_span(request),
        run_id,
    )


@html_template(
    "lintian-fixes/tag-list.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_lintian_fixes_tag_list(request):
    async with request.app.database.acquire() as conn:
        tags = []
        oldnames = {}  # type: Dict[str, List[str]]
        for tag in await iter_lintian_tags(conn):
            try:
                newname = renamed_tags[tag]
            except KeyError:
                tags.append(tag)
            else:
                oldnames.setdefault(newname, []).append(tag)
        tags.sort()
        return {"tags": tags, "oldnames": oldnames}


@html_template("lintian-fixes/tag.html", headers={"Cache-Control": "max-age=600"})
async def handle_lintian_fixes_tag_page(request):
    tag = request.match_info["tag"]
    oldnames = []
    for oldname, newname in renamed_tags.items():
        if newname == tag:
            oldnames.append(oldname)
    async with request.app.database.acquire() as conn:
        packages = list(
            await iter_last_successes_by_lintian_tag(conn, [tag] + oldnames)
        )
    return {
        "tag": tag,
        "oldnames": oldnames,
        "packages": packages,
    }


@html_template(
    "lintian-fixes/candidates.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_lintian_fixes_candidates(request):
    return await generate_candidates(request.app.database)


@html_template(
    "lintian-fixes/developer-table.html", headers={"Cache-Control": "max-age=30"}
)
async def handle_lintian_fixes_developer_table_page(request):
    try:
        developer = request.match_info["developer"]
    except KeyError:
        developer = request.query.get("developer")
    if developer and "@" not in developer:
        developer = "%s@debian.org" % developer
    return await generate_developer_table_page(request.app.database, developer)


@html_template(
    "lintian-fixes/stats.html", headers={"Cache-Control": "max-age=3600"}
)
async def handle_lintian_fixes_stats(request):
    async with request.app.database.acquire() as conn:
        by_tag = await iter_lintian_fixes_counts(conn)
        tags_per_run = {
            c: nr
            for (c, nr) in await conn.fetch(
                """\
select coalesce(c, 0), count(*) from (
    select sum(array_length(fixed_lintian_tags, 1)) c
    from absorbed_lintian_fixes where suite = 'lintian-fixes' group by revision
) as p group by 1
"""
            )
        }
        lintian_brush_versions = {
            (c or "unknown"): nr
            for (c, nr) in await conn.fetch(
                """
select result#>>'{versions,lintian-brush}', count(*) from run
where result_code = 'success' and suite = 'lintian-fixes'
group by 1 order by 1 desc
"""
            )
        }

    return {
        "by_tag": by_tag,
        "tags_per_run": tags_per_run,
        "lintian_brush_versions": lintian_brush_versions,
    }


@html_template(
    "lintian-fixes/failed-list.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_failed_lintian_brush_fixers_list(request):
    return {"fixers": iter_failed_lintian_fixers(request.app.database)}


@html_template(
    "lintian-fixes/failed.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_failed_lintian_brush_fixers(request):
    fixer = request.match_info["fixer"]
    async with request.app.database.acquire() as conn:
        failures = await iter_lintian_brush_fixer_failures(conn, fixer)
        return {"failures": failures, "fixer": fixer}


@html_template(
    "lintian-fixes/regressions.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_lintian_brush_regressions(request):
    async with request.app.database.acquire() as conn:
        packages = await iter_lintian_fixes_regressions(conn)
    return {"packages": packages}


@docs()
async def handle_report(request):
    report = {}
    merge_proposal = {}
    async with request.app['db'].acquire() as conn:
        for package, url in await conn.fetch("""
SELECT
    DISTINCT ON (merge_proposal.url)
    merge_proposal.package, merge_proposal.url
FROM
    merge_proposal
LEFT JOIN run
ON merge_proposal.revision = run.revision AND run.result_code = 'success'
AND status = 'open'
WHERE run.suite = 'lintian-fixes'
"""):
            merge_proposal[package] = url
        query = """
SELECT DISTINCT ON (package)
  result_code,
  start_time,
  package,
  result
FROM
  last_unabsorbed_runs
WHERE suite = 'lintian-fixes'
ORDER BY package, start_time DESC
"""
        for record in await conn.fetch(query):
            if record['result_code'] not in ("success", "nothing-to-do"):
                continue
            data = {
                "timestamp": record['start_time'].isoformat(),
            }
            data["fixed-tags"] = []
            for entry in record['result']["applied"]:
                data["fixed-tags"].extend(entry["fixed_lintian_tags"])
            if record['package'] in merge_proposal:
                data["merge-proposal"] = merge_proposal[record['package']]
            report[record['package']] = data
    return web.json_response(
        report, headers={"Cache-Control": "max-age=600"}, status=200
    )


def register_lintian_fixes_endpoints(router):
    router.add_get(
        "/lintian-fixes/", handle_lintian_fixes, name="lintian-fixes-start"
    )
    router.add_get(
        "/lintian-fixes/pkg/{pkg}/",
        handle_lintian_fixes_pkg,
        name="lintian-fixes-package",
    )
    router.add_get(
        "/lintian-fixes/pkg/{pkg}/{run_id}",
        handle_lintian_fixes_pkg,
        name="lintian-fixes-package-run",
    )

    router.add_get(
        "/lintian-fixes/by-tag/",
        handle_lintian_fixes_tag_list,
        name="lintian-fixes-tag-list",
    )
    router.add_get(
        "/lintian-fixes/by-tag/{tag}",
        handle_lintian_fixes_tag_page,
        name="lintian-fixes-tag",
    )
    router.add_get(
        "/lintian-fixes/by-developer",
        handle_lintian_fixes_developer_table_page,
        name="lintian-fixes-developer-list",
    )
    router.add_get(
        "/lintian-fixes/by-developer/{developer}",
        handle_lintian_fixes_developer_table_page,
        name="lintian-fixes-developer",
    )
    router.add_get(
        "/lintian-fixes/candidates",
        handle_lintian_fixes_candidates,
        name="lintian-fixes-candidates",
    )
    router.add_get(
        "/lintian-fixes/stats", handle_lintian_fixes_stats, name="lintian-fixes-stats"
    )
    router.add_get(
        "/lintian-fixes/failed-fixers/",
        handle_failed_lintian_brush_fixers_list,
        name="failed-lintian-brush-fixer-list",
    )
    router.add_get(
        "/lintian-fixes/failed-fixers/{fixer}",
        handle_failed_lintian_brush_fixers,
        name="failed-lintian-brush-fixer",
    )
    router.add_get(
        "/lintian-fixes/regressions/",
        handle_lintian_brush_regressions,
        name="lintian-brush-regressions",
    )
    router.add_get(
        "/lintian-fixes/api/report",
        handle_report,
        name="lintian-brush-report")
