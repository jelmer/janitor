#!/usr/bin/python3

from aiohttp_apispec import docs
import aiozipkin
import asyncpg
from . import env
from .common import generate_pkg_context, iter_candidates, html_template

SUITE = "multiarch-fixes"


async def generate_pkg_file(
    db, config, policy, client, differ_url, vcs_manager, package, span, run_id=None
):
    return await generate_pkg_context(
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


async def iter_hint_links(conn):
    return await conn.fetch(
        """
select hint, count(hint) from (
    select
        json_array_elements(
          result->'applied-hints')->'link' #>> '{}' as hint
    from
      last_runs
    where
      suite = 'multiarch-fixes'
   ) as bypackage group by 1 order by 2
 desc
"""
    )


async def generate_hint_list(conn: asyncpg.Connection):
    hint_links = await iter_hint_links(conn)
    hints = [(link.split("#")[-1], count) for link, count in hint_links]
    return {"hints": hints}


async def iter_last_successes_by_hint(conn: asyncpg.Connection, hint: str):
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
  json_array_elements(
     result->'applied-hints')->'link' #>> '{}' as hint
from
  run
where
  suite = 'multiarch-fixes' and
  result_code = 'success'
) as package where hint like $1 order by package, start_time desc
""",
        "%#" + hint,
    )


async def generate_hint_page(db, hint):
    async with db.acquire() as conn:
        packages = list(await iter_last_successes_by_hint(conn, hint))
    return {"hint": hint, "packages": packages}


async def generate_candidates(db):
    candidates = []
    async with db.acquire() as conn:
        for row in await iter_candidates(conn, suite=SUITE):
            hints = {}
            for h in row['context'].split(" "):
                hints.setdefault(h, 0)
                hints[h] += 1
            candidates.append((row['package'], list(hints.items()), row['value']))
        candidates.sort(key=lambda x: x[2], reverse=True)
    return {"candidates": candidates}


@html_template(
    env, "multiarch-fixes/start.html", headers={"Cache-Control": "max-age=3600"}
)
async def handle_multiarch_fixes(request):
    return {"SUITE": SUITE}


@html_template(
    env, "multiarch-fixes/hint-list.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_multiarch_fixes_hint_list(request):
    async with request.app.database.acquire() as conn:
        return await generate_hint_list(conn)


@html_template(
    env, "multiarch-fixes/hint.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_multiarch_fixes_hint_page(request):
    return await generate_hint_page(
        request.app.database, request.match_info["hint"]
    )


@html_template(
    env, "multiarch-fixes/stats.html", headers={"Cache-Control": "max-age=3600"}
)
async def handle_multiarch_fixes_stats(request):
    async with request.app.database.acquire() as conn:
        hints_per_run = {
            (c or 0): nr
            for (c, nr) in await conn.fetch(
                """\
select json_array_length(result->'applied-hints'), count(*) from run
where result_code = 'success' and suite = $1 group by 1
""", SUITE
            )
        }
        per_kind = {
            h: nr
            for (h, nr) in await conn.fetch(
                """\
select split_part(link::text, '#', 2), count(*) from
multiarch_hints group by 1
"""
            )
        }

        absorbed_per_kind = {
            h: nr
            for (h, nr) in await conn.fetch(
                """\
select split_part(link::text, '#', 2), count(*) from
absorbed_multiarch_hints group by 1
"""
            )
        }
    return {
        "hints_per_run": hints_per_run,
        "per_kind": per_kind,
        "absorbed_per_kind": absorbed_per_kind,
    }


@html_template(
    env, "multiarch-fixes/candidates.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_multiarch_fixes_candidates(request):
    return await generate_candidates(request.app.database)


@html_template(
    env, "multiarch-fixes/package.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_multiarch_fixes_pkg(request):
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
WHERE run.suite = $1
""", SUITE):
            merge_proposal[package] = url
        query = """
SELECT DISTINCT ON (package)
  result_code,
  start_time,
  package,
  result
FROM
  last_unabsorbed_runs
WHERE suite = $1
ORDER BY package, suite, start_time DESC
"""
        for record in await conn.fetch(query, SUITE):
            if record['result_code'] not in ("success", "nothing-to-do"):
                continue
            data = {
                "timestamp": record['start_time'].isoformat(),
            }
            data["applied-hints"] = record['result'].get("applied-hints")
            if record['package'] in merge_proposal:
                data["merge-proposal"] = merge_proposal[record['package']]
            report[record['package']] = data
    return web.json_response(
        report, headers={"Cache-Control": "max-age=600"}, status=200
    )

def register_multiarch_hints_endpoints(router):
    router.add_get(
        "/multiarch-fixes/", handle_multiarch_fixes, name="multiarch-fixes-start"
    )
    router.add_get(
        "/multiarch-fixes/by-hint/",
        handle_multiarch_fixes_hint_list,
        name="multiarch-fixes-hint-list",
    )
    router.add_get(
        "/multiarch-fixes/stats",
        handle_multiarch_fixes_stats,
        name="multiarch-fixes-stats",
    )
    router.add_get(
        "/multiarch-fixes/by-hint/{hint}",
        handle_multiarch_fixes_hint_page,
        name="multiarch-fixes-hint",
    )
    router.add_get(
        "/multiarch-fixes/candidates",
        handle_multiarch_fixes_candidates,
        name="multiarch-fixes-candidates",
    )
    router.add_get(
        "/multiarch-fixes/pkg/{pkg}/",
        handle_multiarch_fixes_pkg,
        name="multiarch-fixes-package",
    )
    router.add_get(
        "/multiarch-fixes/pkg/{pkg}/{run_id}",
        handle_multiarch_fixes_pkg,
        name="multiarch-fixes-package-run",
    )
    router.add_get(
        "/multiarch-fixes/api/report",
        handle_report, name="multiarch-fixes-report")
