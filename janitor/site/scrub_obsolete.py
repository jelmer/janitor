import aiozipkin
from . import html_template


@html_template(
    "scrub-obsolete-package.html", headers={"Cache-Control": "max-age=600"}
)
async def handle_scrub_obsolete_pkg(request):
    from .common import generate_pkg_context

    # TODO(jelmer): Handle Accept: text/diff
    pkg = request.match_info["pkg"]
    run_id = request.match_info.get("run_id")
    return await generate_pkg_context(
        request.app.database,
        request.app.config,
        "scrub-obsolete",
        request.app.policy,
        request.app.http_client_session,
        request.app.differ_url,
        request.app.vcs_store_url,
        pkg,
        aiozipkin.request_span(request),
        run_id,
    )


def register_scrub_obsolete_endpoints(router):
    router.add_get(
        "/scrub-obsolete/pkg/{pkg}/",
        handle_scrub_obsolete_pkg,
        name="scrub-obsolete-package",
    )
    router.add_get(
        "/scrub-obsolete/pkg/{pkg}/{run_id}",
        handle_scrub_obsolete_pkg,
        name="scrub-obsolete-package-run",
    )
