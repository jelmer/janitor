from . import html_template
from ..config import get_suite_config


@html_template("debianize-start.html", headers={"Cache-Control": "max-age=60"})
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
            "suite_config": get_suite_config(request.app.config, 'debianize'),
        }


@html_template(
    "debianize-package.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_debianize_pkg(request):
    from .common import generate_pkg_context

    # TODO(jelmer): Handle Accept: text/diff
    pkg = request.match_info["pkg"]
    run_id = request.match_info.get("run_id")
    return await generate_pkg_context(
        request.app.database,
        request.app.config,
        "debianize",
        request.app.policy,
        request.app.http_client_session,
        request.app.differ_url,
        request.app.vcs_store_url,
        pkg,
        run_id,
    )


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

