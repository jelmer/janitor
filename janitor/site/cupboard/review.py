#!/usr/bin/python3

import logging

import aiozipkin
from asyncio import TimeoutError
from aiohttp import ClientConnectorError, ClientResponseError

from breezy.revision import NULL_REVISION

from janitor import state
from .. import (
    env,
    get_archive_diff,
    BuildDiffUnavailable,
    DebdiffRetrievalError,
)
from ..common import (
    get_unchanged_run,
    render_template_for_request,
)
from ...review import iter_needs_review

MAX_DIFF_SIZE = 200 * 1024


async def generate_rejected(conn, config, campaign=None):
    if campaign is None:
        campaigns = [c.name for c in config.campaign]
    else:
        campaigns = [campaign]

    runs = await conn.fetch(
        "SELECT id, suite, package FROM last_unabsorbed_runs "
        "WHERE review_status = 'rejected' AND suite = ANY($1::text[]) "
        "ORDER BY finish_time DESC",
        campaigns)

    reviews = {}

    for row in await conn.fetch(
            'SELECT * FROM review WHERE id = ANY($1:text[])',
            [run.id for run in runs]):
        reviews.setdefault(row['run_id'], []).append(row)

    return {"runs": runs, "suite": campaign, "reviews": reviews}


async def generate_review(
    conn, request, client, differ_url, vcs_managers, suites=None,
    publishable_only=True
):
    if 'required_only' in request.query:
        required_only = (request.query['required_only'] == 'true')
    else:
        required_only = None

    limit = int(request.query.get('limit', '100'))

    span = aiozipkin.request_span(request)

    if request.get('user'):
        reviewer = request['user'].get('email')
    else:
        reviewer = None

    with span.new_child('sql:needs-review'):
        entries = await iter_needs_review(
            conn,
            limit=limit,
            campaigns=suites,
            publishable_only=publishable_only,
            required_only=required_only,
            reviewer=reviewer
        )
    if not entries:
        return await render_template_for_request(
            env, "cupboard/review-done.html", request, {
                'publishable_only': publishable_only})

    (
        run_id,
        command,
        package,
        suite,
        vcs_type,
        result_branches,
        main_branch_revision,
        value,
    ) = entries.pop(0)

    async def show_diff(role):
        try:
            (remote_name, base_revid, revid) = state.get_result_branch(result_branches, role)
        except KeyError:
            return ""
        external_url = "/api/run/%s/diff?role=%s" % (run_id, role)
        if vcs_type is None:
            return "no vcs known"
        if revid is None:
            return "Branch deleted"
        try:
            with span.new_child('vcs-diff'):
                diff = (await vcs_managers[vcs_type].get_diff(
                    package,
                    base_revid.encode('utf-8') if base_revid else NULL_REVISION,
                    revid.encode('utf-8'))
                ).decode("utf-8", "replace")
                if len(diff) > MAX_DIFF_SIZE:
                    return "Diff too large (%d). See it at %s" % (
                        len(diff),
                        external_url,
                    )
                else:
                    return diff
        except ClientResponseError as e:
            return "Unable to retrieve diff; error code %d" % e.status
        except NotImplementedError as e:
            return str(e)
        except ClientConnectorError as e:
            return "Unable to retrieve diff; error %s" % e
        except TimeoutError:
            return "Timeout while retrieving diff; see it at %s" % external_url

    async def get_revision_info(role):
        try:
            (remote_name, base_revid, revid) = state.get_result_branch(result_branches, role)
        except KeyError:
            return []

        if base_revid == revid:
            return []
        if vcs_type is None:
            logging.warning("No vcs known for run %s", run_id)
            return []
        if revid is None:
            return []
        old_revid = base_revid.encode('utf-8') if base_revid else NULL_REVISION
        new_revid = revid.encode('utf-8')
        try:
            return await vcs_managers[vcs_type].get_revision_info(package, old_revid, new_revid)
        except ClientResponseError as e:
            logging.warning("Unable to retrieve commit info; error code %d", e.status)
            return []
        except ClientConnectorError as e:
            logging.warning("Unable to retrieve diff; error %s", e)
            return []
        except TimeoutError:
            logging.warning("Timeout while retrieving commit info")
            return []

    async def show_debdiff():
        with span.new_child("sql:unchanged-run"):
            unchanged_run = await get_unchanged_run(
                conn, package, main_branch_revision.encode('utf-8')
            )
        if unchanged_run is None:
            return "<p>No control run</p>"
        try:
            with span.new_child('archive-diff'):
                text, unused_content_type = await get_archive_diff(
                    client,
                    differ_url,
                    run_id,
                    unchanged_run.id,
                    kind="debdiff",
                    filter_boring=True,
                    accept="text/html",
                )
                return text.decode("utf-8", "replace")
        except DebdiffRetrievalError as e:
            return "Unable to retrieve debdiff: %r" % e
        except BuildDiffUnavailable:
            return "<p>No build diff generated</p>"

    kwargs = {
        "show_diff": show_diff,
        "show_debdiff": show_debdiff,
        "get_revision_info": get_revision_info,
        "package_name": package,
        "run_id": run_id,
        "command": command,
        "branches": result_branches,
        "suite": suite,
        "suites": suites,
        "value": value,
        "publishable_only": publishable_only,
        "MAX_DIFF_SIZE": MAX_DIFF_SIZE,
        "todo": [
            {
                'package': entry['package'],
                'command': entry['command'],
                'id': entry['id'],
                'branches': [rb[0] for rb in entry['result_branches']],
                'value': entry['value']
            } for entry in entries
        ],
    }
    return await render_template_for_request(env, "cupboard/review.html", request, kwargs)


async def generate_review_stats(conn):
    return {
        'by_reviewer': await conn.fetch(
            "select distinct(reviewer), count(*) from review group by reviewer"),
        'by_review_status': await conn.fetch(
            "with total as (select count(*) as cnt from review) "
            "select review_status, count(*) as cnt, "
            "1.0 * count(*) / (select cnt from total) * 100.0 as pct "
            "from review group by 1"
        )}
