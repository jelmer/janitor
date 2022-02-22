import aiozipkin
from typing import Dict, List
from . import html_template
from ..config import get_campaign_config
from lintian_brush.lintian_overrides import load_renamed_tags
renamed_tags = load_renamed_tags()


@html_template("debianize/start.html", headers={"Cache-Control": "max-age=60"})
async def handle_debianize_start(request):
    async with request.app.database.acquire() as conn:
        return {
            "packages": await conn.fetch("""
select distinct on (source) source, run.package AS package, version
from debian_build
INNER JOIN run on debian_build.run_id = run.id
where run.suite = 'debianize'
order by source, version desc
"""),
            "suite": 'debianize',
            "campaign_config": get_campaign_config(request.app['config'], 'debianize'),
        }


@html_template(
    "debianize/package.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_debianize_pkg(request):
    from .common import generate_pkg_context

    # TODO(jelmer): Handle Accept: text/diff
    pkg = request.match_info["pkg"]
    run_id = request.match_info.get("run_id")
    return await generate_pkg_context(
        request.app.database,
        request.app['config'],
        "debianize",
        request.app.policy,
        request.app.http_client_session,
        request.app.differ_url,
        request.app['vcs_manager'],
        pkg,
        aiozipkin.request_span(request),
        run_id,
    )


@html_template("debianize/tag.html", headers={"Cache-Control": "max-age=600"})
async def handle_debianize_tag_page(request):
    tag = request.match_info["tag"]
    oldnames = []
    for oldname, newname in renamed_tags.items():
        if newname == tag:
            oldnames.append(oldname)
    async with request.app.database.acquire() as conn:
        issues = await conn.fetch("""
select path, name, lintian_results.context, severity from run inner join lintian_results on lintian_results.run_id = run.id where suite = 'debianize' AND lintian_results.name = $1
""", tag)
    return {
        "tag": tag,
        "oldnames": oldnames,
        "issues": issues,
    }


@html_template(
    "debianize/tag-list.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_debianize_lintian_tag_list(request):
    async with request.app.database.acquire() as conn:
        tags = []
        oldnames = {}  # type: Dict[str, List[str]]
        for tag, cnt in await conn.fetch("""
select lintian_results.name, count(*) from run inner join lintian_results on
lintian_results.run_id = run.id where suite = 'debianize' group by
lintian_results.name order by 2 desc
"""):
            try:
                newname = renamed_tags[tag]
            except KeyError:
                tags.append((tag, cnt))
            else:
                oldnames.setdefault(newname, []).append(tag)
        return {"tags": tags, "oldnames": oldnames}


def register_debianize_endpoints(router):
    router.add_get(
        "/debianize/",
        handle_debianize_start,
        name="debianize-start")
    router.add_get(
        "/debianize/pkg/{pkg}/",
        handle_debianize_pkg,
        name="debianize-package",
    )
    router.add_get(
        "/debianize/pkg/{pkg}/{run_id}",
        handle_debianize_pkg,
        name="debianize-package-run",
    )
    router.add_get(
        "/cupboard/debianize/lintian/",
        handle_debianize_lintian_tag_list,
        name="debianize-lintian-tag-list",
    )
    router.add_get(
        "/cupboard/debianize/lintian/{tag}",
        handle_debianize_tag_page,
        name="debianize-tag",
    )
