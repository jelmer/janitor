#!/usr/bin/python3
# Copyright (C) 2019 Jelmer Vernooij <jelmer@jelmer.uk>
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

"""Publishing VCS changes."""

from contextlib import AsyncExitStack
from dataclasses import dataclass
from datetime import datetime, timedelta
import asyncio
import json
import logging
import os
import sys
import time
from typing import Dict, List, Optional, Any, Tuple, Set, AsyncIterable, Iterator
import uuid


import aioredis
import aiozipkin
from aiohttp.web_middlewares import normalize_path_middleware
from aiohttp import web
import asyncpg
import asyncpg.pool

from aiohttp_apispec import (
    docs,
    response_schema,
    setup_aiohttp_apispec,
)

from aiohttp_openmetrics import (
    Counter,
    Gauge,
    Histogram,
    REGISTRY,
    setup_metrics,
    push_to_gateway
)
from marshmallow import Schema, fields

from breezy import urlutils
import gpg

from breezy.forge import (
    Forge,
    forges,
    ForgeLoginRequired,
    UnsupportedForge,
    iter_forge_instances,
)
from silver_platter.utils import (
    open_branch,
    BranchMissing,
    BranchUnavailable,
    BranchRateLimited,
    full_branch_url,
)

from breezy.errors import PermissionDenied, UnexpectedHttpStatus
from breezy.forge import (
    get_proposal_by_url,
    MergeProposal,
)
from breezy.transport import Transport
import breezy.plugins.gitlab  # noqa: F401
import breezy.plugins.launchpad  # noqa: F401
import breezy.plugins.github  # noqa: F401

from . import (
    state,
)
from .compat import to_thread
from .config import read_config, get_campaign_config, Campaign
from .schedule import (
    do_schedule,
    TRANSIENT_ERROR_RESULT_CODES,
    PolicyUnavailable,
)
from .vcs import (
    VcsManager,
    get_vcs_managers_from_config,
    bzr_to_browse_url,
)


from ._launchpad import override_launchpad_consumer_name
override_launchpad_consumer_name()


EXISTING_RUN_RETRY_INTERVAL = 30

MODE_SKIP = "skip"
MODE_BUILD_ONLY = "build-only"
MODE_PUSH = "push"
MODE_PUSH_DERIVED = "push-derived"
MODE_PROPOSE = "propose"
MODE_ATTEMPT_PUSH = "attempt-push"
MODE_BTS = "bts"
SUPPORTED_MODES = [
    MODE_PUSH,
    MODE_SKIP,
    MODE_BUILD_ONLY,
    MODE_PUSH_DERIVED,
    MODE_PROPOSE,
    MODE_ATTEMPT_PUSH,
    MODE_BTS,
]


proposal_rate_limited_count = Counter(
    "proposal_rate_limited",
    "Number of attempts to create a proposal that was rate-limited",
    ["package", "campaign"],
)
open_proposal_count = Gauge(
    "open_proposal_count", "Number of open proposals.", labelnames=("maintainer",)
)
merge_proposal_count = Gauge(
    "merge_proposal_count",
    "Number of merge proposals by status.",
    labelnames=("status",),
)
last_publish_pending_success = Gauge(
    "last_publish_pending_success",
    "Last time pending changes were successfully published",
)
last_scan_existing_success = Gauge(
    "last_scan_existing_success",
    "Last time existing merge proposals were successfully scanned")
publish_latency = Histogram(
    "publish_latency", "Delay between build finish and publish."
)

exponential_backoff_count = Counter(
    "exponential_backoff_count",
    "Number of times publishing has been skipped due to exponential backoff")

push_limit_count = Counter(
    "push_limit_count",
    "Number of times pushes haven't happened due to the limit")

missing_branch_url_count = Counter(
    "missing_branch_url_count",
    "Number of runs that weren't published because they had a "
    "missing branch URL")

missing_publish_mode_count = Counter(
    "missing_publish_mode_count",
    "Number of runs not published due to missing publish mode",
    labelnames=("role", ))

unpublished_aux_branches_count = Counter(
    "unpublished_aux_branches_count",
    "Number of branches not published because auxiliary branches "
    "were not yet published",
    labelnames=("role", ))

command_changed_count = Counter(
    "command_changed_count",
    "Number of runs not published because the codemod command changed")


no_result_branches_count = Counter(
    "no_result_branches_count",
    "Runs not published since there were no result branches")


missing_main_result_branch_count = Counter(
    "missing_main_result_branch_count",
    "Runs not published because of missing main result branch")

forge_rate_limited_count = Counter(
    "forge_rate_limited_count",
    "Runs were not published because the relevant forge was rate-limiting",
    labelnames=("forge", ))


logger = logging.getLogger('janitor.publish')


routes = web.RouteTableDef()


class RateLimited(Exception):
    """A rate limit was reached."""


class MaintainerRateLimited(RateLimited):
    """Per-maintainer rate-limit was reached."""

    def __init__(self, maintainer_email, open_mps, max_open_mps):
        super(MaintainerRateLimited, self).__init__(
            "Maintainer %s already has %d merge proposal open (max: %d)" % (
                maintainer_email, open_mps, max_open_mps))
        self.maintainer_email = maintainer_email
        self.open_mps = open_mps
        self.max_open_mps = max_open_mps


class RateLimiter(object):
    def set_mps_per_maintainer(
        self, mps_per_maintainer: Dict[str, Dict[str, int]]
    ) -> None:
        raise NotImplementedError(self.set_mps_per_maintainer)

    def check_allowed(self, maintainer_email: str) -> None:
        raise NotImplementedError(self.check_allowed)

    def inc(self, maintainer_email: str) -> None:
        raise NotImplementedError(self.inc)

    def get_stats(self) -> Dict[str, Tuple[int, int]]:
        raise NotImplementedError(self.get_stats)


class FixedRateLimiter(RateLimiter):

    _open_mps_per_maintainer: Optional[Dict[str, int]]

    def __init__(self, max_mps_per_maintainer: Optional[int] = None):
        self._max_mps_per_maintainer = max_mps_per_maintainer
        self._open_mps_per_maintainer = None

    def set_mps_per_maintainer(self, mps_per_maintainer: Dict[str, Dict[str, int]]):
        self._open_mps_per_maintainer = mps_per_maintainer["open"]

    def check_allowed(self, maintainer_email: str):
        if not self._max_mps_per_maintainer:
            return
        if self._open_mps_per_maintainer is None:
            # Be conservative
            raise RateLimited("Open mps per maintainer not yet determined.")
        current = self._open_mps_per_maintainer.get(maintainer_email, 0)
        if current > self._max_mps_per_maintainer:
            raise MaintainerRateLimited(
                maintainer_email, current, self._max_mps_per_maintainer)

    def inc(self, maintainer_email: str):
        if self._open_mps_per_maintainer is None:
            return
        self._open_mps_per_maintainer.setdefault(maintainer_email, 0)
        self._open_mps_per_maintainer[maintainer_email] += 1

    def get_stats(self) -> Dict[str, Tuple[int, int]]:
        return {
            email: (current, self._max_mps_per_maintainer)
            for (email, current) in self._open_mps_per_maintainer.items()}


class NonRateLimiter(RateLimiter):
    def check_allowed(self, email):
        pass

    def inc(self, maintainer_email):
        pass

    def set_mps_per_maintainer(self, mps_per_maintainer):
        pass

    def get_stats(self):
        return {}


class SlowStartRateLimiter(RateLimiter):
    def __init__(self, max_mps_per_maintainer=None):
        self._max_mps_per_maintainer = max_mps_per_maintainer
        self._open_mps_per_maintainer: Optional[Dict[str, int]] = None
        self._merged_mps_per_maintainer: Optional[Dict[str, int]] = None

    def check_allowed(self, email: str) -> None:
        if (
            self._open_mps_per_maintainer is None
            or self._merged_mps_per_maintainer is None
        ):
            # Be conservative
            raise RateLimited("Open mps per maintainer not yet determined.")
        current = self._open_mps_per_maintainer.get(email, 0)
        if self._max_mps_per_maintainer and current >= self._max_mps_per_maintainer:
            raise MaintainerRateLimited(
                email, current, self._max_mps_per_maintainer)
        limit = self._get_limit(email)
        if current >= limit:
            raise MaintainerRateLimited(email, current, limit)

    def _get_limit(self, maintainer_email):
        return self._merged_mps_per_maintainer.get(email, 0) + 1

    def inc(self, maintainer_email: str):
        if self._open_mps_per_maintainer is None:
            return
        self._open_mps_per_maintainer.setdefault(maintainer_email, 0)
        self._open_mps_per_maintainer[maintainer_email] += 1

    def set_mps_per_maintainer(self, mps_per_maintainer: Dict[str, Dict[str, int]]):
        self._open_mps_per_maintainer = mps_per_maintainer["open"]
        self._merged_mps_per_maintainer = mps_per_maintainer["merged"]

    def get_stats(self):
        if self._open_mps_per_maintainer is None:
            return {}
        else:
            return {
                email: (
                    current,
                    min(self._max_mps_per_maintainer, self._get_limit(email)))
                for email, current in self._open_mps_per_maintainer.items()}


class PublishFailure(Exception):
    def __init__(self, mode: str, code: str, description: str):
        self.mode = mode
        self.code = code
        self.description = description


async def derived_branch_name(conn, campaign_config, run, role):
    if len(run.result_branches) == 1:
        name = campaign_config.branch_name
    else:
        name = "%s/%s" % (campaign_config.branch_name, role)

    if await state.has_cotenants(conn, run.package, run.branch_url):
        return name + "/" + run.package
    else:
        return name


def branches_match(url_a: Optional[str], url_b: Optional[str]) -> bool:
    if url_a == url_b:
        return True
    if url_a is None:
        return url_b is None
    if url_b is None:
        return False
    url_a, params_a = urlutils.split_segment_parameters(url_a.rstrip("/"))
    url_b, params_b = urlutils.split_segment_parameters(url_b.rstrip("/"))
    # TODO(jelmer): Support following redirects
    if url_a.rstrip("/") != url_b.rstrip("/"):  # type: ignore
        return False
    try:
        return open_branch(url_a).name == open_branch(url_b).name
    except BranchMissing:
        return False


@dataclass
class PublishResult:

    description: str
    is_new: bool = False
    proposal_url: Optional[str] = None
    branch_name: Optional[str] = None


async def publish_one(
    template_env_path: Optional[str],
    campaign: str,
    pkg: str,
    command,
    codemod_result,
    main_branch_url: str,
    mode: str,
    role: str,
    revision: bytes,
    log_id: str,
    unchanged_id: str,
    derived_branch_name: str,
    maintainer_email: str,
    vcs_manager: VcsManager,
    redis,
    maintainer_rate_limiter: RateLimiter,
    dry_run: bool,
    differ_url: str,
    external_url: str,
    require_binary_diff: bool = False,
    allow_create_proposal: bool = False,
    reviewers: Optional[List[str]] = None,
    derived_owner: Optional[str] = None,
    result_tags: Optional[List[Tuple[str, bytes]]] = None,
    commit_message_template: Optional[str] = None,
) -> PublishResult:
    """Publish a single run in some form.

    Args:
      campaign: The campaign name
      pkg: Package name
      command: Command that was run
    """
    assert mode in SUPPORTED_MODES, "mode is %r" % (mode, )
    local_branch = vcs_manager.get_branch(pkg, "%s/%s" % (campaign, role))
    if local_branch is None:
        raise PublishFailure(
            mode,
            "result-branch-not-found",
            "can not find local branch for %s / %s / %s (%s)"
            % (pkg, campaign, role, log_id),
        )

    request = {
        "dry-run": dry_run,
        "campaign": campaign,
        "package": pkg,
        "command": command,
        "codemod_result": codemod_result,
        "target_branch_url": main_branch_url.rstrip("/"),
        "source_branch_url": full_branch_url(local_branch),
        "derived_branch_name": derived_branch_name,
        "mode": mode,
        "role": role,
        "log_id": log_id,
        "unchanged_id": unchanged_id,
        "require-binary-diff": require_binary_diff,
        "allow_create_proposal": allow_create_proposal,
        "external_url": external_url,
        "differ_url": differ_url,
        "derived-owner": derived_owner,
        "revision": revision.decode("utf-8"),
        "reviewers": reviewers,
        "commit_message_template": commit_message_template,
    }

    if result_tags:
        request["tags"] = {n: r.decode("utf-8") for (n, r) in result_tags}
    else:
        request["tags"] = {}

    args = [sys.executable, "-m", "janitor.publish_one"]

    if template_env_path:
        args.append('--template-env-path=%s' % template_env_path)

    p = await asyncio.create_subprocess_exec(
        *args,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        stdin=asyncio.subprocess.PIPE
    )

    (stdout, stderr) = await p.communicate(json.dumps(request).encode())

    if p.returncode == 1:
        try:
            response = json.loads(stdout.decode())
        except json.JSONDecodeError:
            raise PublishFailure(mode, "publisher-invalid-response", stderr.decode())
        sys.stderr.write(stderr.decode())
        raise PublishFailure(mode, response["code"], response["description"])

    if p.returncode == 0:
        response = json.loads(stdout.decode())

        proposal_url = response.get("proposal_url")
        branch_name = response.get("branch_name")
        is_new = response.get("is_new")
        description = response.get('description')

        if proposal_url and is_new:
            await redis.publish_json(
                'merge-proposal',
                {"url": proposal_url, "status": "open", "package": pkg,
                 "campaign": campaign,
                 "target_branch_url": main_branch_url.rstrip("/")})

            merge_proposal_count.labels(status="open").inc()
            maintainer_rate_limiter.inc(maintainer_email)
            open_proposal_count.labels(maintainer=maintainer_email).inc()

        return PublishResult(
            proposal_url=proposal_url, branch_name=branch_name, is_new=is_new,
            description=description)

    raise PublishFailure(mode, "publisher-invalid-response", stderr.decode())


def calculate_next_try_time(finish_time: datetime, attempt_count: int) -> datetime:
    try:
        return finish_time + (2 ** attempt_count * timedelta(hours=1))
    except OverflowError:
        return datetime.utcnow() + timedelta(hours=(7 * 24))


async def consider_publish_run(
        conn, config, template_env_path,
        vcs_managers, maintainer_rate_limiter, external_url, differ_url,
        redis,
        run, maintainer_email,
        unpublished_branches, command,
        push_limit=None, require_binary_diff=False,
        dry_run=False):
    if run.revision is None:
        logger.warning(
            "Run %s is publish ready, but does not have revision set.", run.id
        )
        return {}
    campaign_config = get_campaign_config(config, run.suite)
    # TODO(jelmer): next try in SQL query
    attempt_count = await get_publish_attempt_count(
        conn, run.revision, {"differ-unreachable"}
    )
    next_try_time = calculate_next_try_time(run.finish_time, attempt_count)
    if datetime.utcnow() < next_try_time:
        logger.info(
            "Not attempting to push %s / %s (%s) due to "
            "exponential backoff. Next try in %s.",
            run.package,
            run.suite,
            run.id,
            next_try_time - datetime.utcnow(),
        )
        exponential_backoff_count.inc()
        return {}
    ms = [b[4] for b in unpublished_branches]
    if push_limit is not None and (
            MODE_PUSH in ms or MODE_ATTEMPT_PUSH in ms):
        if push_limit == 0:
            logger.info(
                "Not pushing %s / %s: push limit reached",
                run.package,
                run.suite,
            )
            push_limit_count.inc()
            return {}
    if run.branch_url is None:
        logger.warning(
            '%s: considering publishing for branch without branch url',
            run.id)
        missing_branch_url_count.inc()
        return {}
    actual_modes = {}
    for (
        role,
        remote_name,
        base_revision,
        revision,
        publish_mode,
        max_frequency_days
    ) in unpublished_branches:
        if publish_mode is None:
            logger.warning(
                "%s: No publish mode for branch with role %s", run.id, role
            )
            missing_publish_mode_count.labels(role=role).inc()
            continue
        if role == 'main' and None in actual_modes.values():
            logger.warning(
                "%s: Skipping branch with role %s, as not all "
                "auxiliary branches were published.", run.id, role)
            unpublished_aux_branches_count.labels(role=role).inc()
            continue
        actual_modes[role] = await publish_from_policy(
            conn,
            campaign_config,
            template_env_path,
            maintainer_rate_limiter,
            vcs_managers,
            run,
            role,
            maintainer_email,
            run.branch_url,
            redis,
            publish_mode,
            max_frequency_days,
            command,
            dry_run=dry_run,
            external_url=external_url,
            differ_url=differ_url,
            require_binary_diff=require_binary_diff,
            force=False,
            requestor="publisher (publish pending)",
        )

    return actual_modes


async def iter_publish_ready(
    conn: asyncpg.Connection,
    campaigns: Optional[List[str]] = None,
    review_status: Optional[List[str]] = None,
    limit: Optional[int] = None,
    needs_review: Optional[bool] = None,
    run_id: Optional[str] = None,
) -> AsyncIterable[
    Tuple[
        state.Run,
        str,
        str,
        List[Tuple[str, str, bytes, bytes, Optional[str], Optional[int], Optional[str]]],
    ]
]:
    args: List[Any] = []
    query = """
SELECT * FROM publish_ready
"""
    conditions = []
    if campaigns is not None:
        args.append(campaigns)
        conditions.append("suite = ANY($%d::text[])" % len(args))
    if run_id is not None:
        args.append(run_id)
        conditions.append("id = $%d" % len(args))
    if review_status is not None:
        args.append(review_status)
        conditions.append("review_status = ANY($%d::review_status[])" % (len(args),))

    publishable_condition = (
        "exists (select from unnest(unpublished_branches) where "
        "mode in ('propose', 'attempt-push', 'push-derived', 'push'))"
    )

    order_by = []

    conditions.append(publishable_condition)

    if needs_review is not None:
        args.append(needs_review)
        conditions.append('needs_review = $%d' % (len(args)))

    if conditions:
        query += " WHERE " + " AND ".join(conditions)

    order_by.extend(["value DESC NULLS LAST", "finish_time DESC"])

    if order_by:
        query += " ORDER BY " + ", ".join(order_by) + " "

    if limit is not None:
        query += " LIMIT %d" % limit
    for record in await conn.fetch(query, *args):
        yield tuple(  # type: ignore
            [state.Run.from_row(record),
             record['maintainer_email'],
             record['policy_command'],
             record['unpublished_branches']
             ]
        )


async def publish_pending_ready(
    db,
    redis,
    config,
    template_env_path,
    maintainer_rate_limiter,
    vcs_managers,
    dry_run: bool,
    external_url: str,
    differ_url: str,
    reviewed_only: bool = False,
    push_limit: Optional[int] = None,
    require_binary_diff: bool = False,
):
    start = time.time()
    actions: Dict[str, int] = {}

    if reviewed_only:
        review_status = ["approved"]
    else:
        review_status = ["approved", "unreviewed"]

    async with db.acquire() as conn1, db.acquire() as conn:
        async for (
            run,
            maintainer_email,
            command,
            unpublished_branches,
        ) in iter_publish_ready(
            conn1, review_status=review_status,
            needs_review=False,
        ):
            actual_modes = await consider_publish_run(
                conn, config=config,
                template_env_path=template_env_path,
                vcs_managers=vcs_managers,
                maintainer_rate_limiter=maintainer_rate_limiter,
                external_url=external_url, differ_url=differ_url,
                redis=redis,
                run=run,
                command=command,
                maintainer_email=maintainer_email,
                unpublished_branches=unpublished_branches,
                push_limit=push_limit,
                require_binary_diff=require_binary_diff,
                dry_run=dry_run)
            for actual_mode in actual_modes.values():
                actions.setdefault(actual_mode, 0)
                actions[actual_mode] += 1
            if MODE_PUSH in actual_modes.values() and push_limit is not None:
                push_limit -= 1

    logger.info("Actions performed: %r", actions)
    logger.info(
        "Done publishing pending changes; duration: %.2fs" % (time.time() - start)
    )

    last_publish_pending_success.set_to_current_time()


async def handle_publish_failure(e, conn, run, bucket):
    from .schedule import (
        do_schedule,
        do_schedule_control,
    )

    unchanged_run = await conn.fetchrow(
        "SELECT result_code, package, revision FROM last_runs WHERE revision = $2 AND package = $1 and result_code = 'success'",
        run.package, run.main_branch_revision.decode('utf-8')
    )

    code = e.code
    description = e.description
    if e.code == "merge-conflict":
        logger.info("Merge proposal would cause conflict; restarting.")
        await do_schedule(
            conn,
            package=run.package,
            campaign=run.suite,
            change_set=run.change_set,
            requestor="publisher (pre-creation merge conflict)",
            bucket=bucket,
        )
    elif e.code == "diverged-branches":
        logger.info("Branches have diverged; restarting.")
        await do_schedule(
            conn,
            package=run.package,
            campaign=run.suite,
            change_set=run.change_set,
            requestor="publisher (diverged branches)",
            bucket=bucket,
        )
    elif e.code == "missing-build-diff-self":
        if run.result_code != "success":
            description = "Missing build diff; run was not actually successful?"
        else:
            description = "Missing build artifacts, rescheduling"
            await do_schedule(
                conn,
                package=run.package,
                campaign=run.suite,
                change_set=run.change_set,
                refresh=True,
                requestor="publisher (missing build artifacts - self)",
                bucket=bucket,
            )
    elif e.code == "missing-build-diff-control":
        if unchanged_run and unchanged_run['result_code'] != "success":
            description = (
                "Missing build diff; last control run failed (%s)."
                % unchanged_run['result_code']
            )
        elif unchanged_run and unchanged_run['result_code'] == 'success':
            description = (
                "Missing build diff due to control run, but successful "
                "control run exists. Rescheduling."
            )
            await do_schedule_control(
                conn,
                package=unchanged_run['package'],
                main_branch_revision=unchanged_run['revision'].encode('utf-8'),
                refresh=True,
                requestor="publisher (missing build artifacts - control)",
                bucket=bucket,
            )
        else:
            description = "Missing binary diff; requesting control run."
            if run.main_branch_revision is not None:
                await do_schedule_control(
                    conn,
                    package=run.package,
                    main_branch_revision=run.main_branch_revision,
                    requestor="publisher (missing control run for diff)",
                    bucket=bucket,
                )
            else:
                logger.warning(
                    "Successful run (%s) does not have main branch " "revision set",
                    run.id,
                )
    return (code, description)


async def already_published(
    conn: asyncpg.Connection, package: str, branch_name: str, revision: bytes, mode: str
) -> bool:
    row = await conn.fetchrow(
        """\
SELECT * FROM publish
WHERE mode = $1 AND revision = $2 AND package = $3 AND branch_name = $4
""",
        mode,
        revision.decode("utf-8"),
        package,
        branch_name,
    )
    if row:
        return True
    return False


async def get_open_merge_proposal(
    conn: asyncpg.Connection, package: str, branch_name: str
) -> bytes:
    query = """\
SELECT
    merge_proposal.revision
FROM
    merge_proposal
INNER JOIN publish ON merge_proposal.url = publish.merge_proposal_url
WHERE
    merge_proposal.status = 'open' AND
    merge_proposal.package = $1 AND
    publish.branch_name = $2
ORDER BY timestamp DESC
"""
    return await conn.fetchrow(query, package, branch_name)


async def check_last_published(
        conn: asyncpg.Connection, campaign: str, package: str) -> Optional[datetime]:
    return await conn.fetchval("""
SELECT timestamp from publish left join run on run.revision = publish.revision
WHERE run.suite = $1 and run.package = $2 AND publish.result_code = 'success'
order by timestamp desc limit 1
""", campaign, package)


async def store_publish(
    conn: asyncpg.Connection,
    package,
    branch_name,
    main_branch_revision,
    revision,
    role,
    mode,
    result_code,
    description,
    merge_proposal_url=None,
    publish_id=None,
    requestor=None,
):
    if isinstance(revision, bytes):
        revision = revision.decode("utf-8")
    if isinstance(main_branch_revision, bytes):
        main_branch_revision = main_branch_revision.decode("utf-8")
    async with conn.transaction():
        if merge_proposal_url:
            await conn.execute(
                "INSERT INTO merge_proposal (url, package, status, "
                "revision) VALUES ($1, $2, 'open', $3) ON CONFLICT (url) "
                "DO UPDATE SET package = EXCLUDED.package, "
                "revision = EXCLUDED.revision",
                merge_proposal_url,
                package,
                revision,
            )
        else:
            # TODO(jelmer): do something by branch instead?
            if revision is None:
                raise AssertionError
            await conn.execute(
                "UPDATE new_result_branch SET absorbed = true WHERE revision = $1",
                revision)
        await conn.execute(
            "INSERT INTO publish (package, branch_name, "
            "main_branch_revision, revision, role, mode, result_code, "
            "description, merge_proposal_url, id, requestor) "
            "values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) ",
            package,
            branch_name,
            main_branch_revision,
            revision,
            role,
            mode,
            result_code,
            description,
            merge_proposal_url,
            publish_id,
            requestor,
        )


async def publish_from_policy(
    conn,
    campaign_config: Campaign,
    template_env_path,
    maintainer_rate_limiter,
    vcs_managers,
    run: state.Run,
    role: str,
    maintainer_email: str,
    main_branch_url: str,
    redis,
    mode: str,
    max_frequency_days: Optional[int],
    command: str,
    dry_run: bool,
    external_url: str,
    differ_url: str,
    require_binary_diff: bool = False,
    force: bool = False,
    requestor: Optional[str] = None,
):
    if not command:
        logger.warning("no command set for %s", run.id)
        return
    if command != run.command:
        command_changed_count.inc()
        logger.warning(
            "Not publishing %s/%s: command is different (policy changed?). "
            "Build used %r, now: %r. Rescheduling.",
            run.package,
            run.suite,
            run.command,
            command,
        )
        await do_schedule(
            conn,
            run.package,
            run.suite,
            change_set=run.change_set,
            command=command,
            bucket="update-new-mp",
            refresh=True,
            requestor="publisher (changed policy: %r => %r)" % (
                run.command, command),
        )
        return

    publish_id = str(uuid.uuid4())
    if mode in (None, MODE_BUILD_ONLY, MODE_SKIP):
        return
    if run.result_branches is None:
        logger.warning("no result branches for %s", run.id)
        no_result_branches_count.inc()
        return
    try:
        (remote_branch_name, base_revision, revision) = run.get_result_branch(role)
    except KeyError:
        missing_main_result_branch_count.inc()
        logger.warning("unable to find main branch: %s", run.id)
        return

    main_branch_url = role_branch_url(main_branch_url, remote_branch_name)

    if not force and await already_published(
        conn, run.package, campaign_config.branch_name, revision, mode
    ):
        return
    if mode in (MODE_PROPOSE, MODE_ATTEMPT_PUSH):
        open_mp = await get_open_merge_proposal(
            conn, run.package, campaign_config.branch_name
        )
        if not open_mp:
            try:
                maintainer_rate_limiter.check_allowed(maintainer_email)
            except RateLimited as e:
                proposal_rate_limited_count.labels(
                    package=run.package, campaign=run.suite
                ).inc()
                logger.debug(
                    "Not creating proposal for %s/%s: %s", run.package, run.suite, e
                )
                mode = MODE_BUILD_ONLY
            if max_frequency_days is not None:
                last_published = await check_last_published(
                    conn, run.suite, run.package)
                if (last_published is not None
                        and (datetime.utcnow() - last_published).days < max_frequency_days):
                    logger.debug(
                        'Not creating proposal for %s/%s: '
                        'was published already in last %d days (at %s)',
                        run.package, run.suite, max_frequency_days, last_published)
                    mode = MODE_BUILD_ONLY
    if mode in (MODE_BUILD_ONLY, MODE_SKIP):
        return

    unchanged_run = await conn.fetchrow(
        "SELECT id, result_code FROM last_runs WHERE package = $1 AND revision = $2 AND result_code = 'success'",
        run.package, base_revision.decode('utf-8'))

    # TODO(jelmer): Make this more generic
    if (
        unchanged_run
        and unchanged_run['result_code'] in (
            "debian-upstream-metadata-invalid", )
        and run.suite == "lintian-fixes"
    ):
        require_binary_diff = False

    logger.info(
        "Publishing %s / %r / %s (mode: %s)", run.package, run.command, role, mode
    )
    try:
        publish_result = await publish_one(
            template_env_path,
            run.suite,
            run.package,
            run.command,
            run.result,
            main_branch_url=main_branch_url,
            mode=mode,
            role=role,
            revision=revision,
            log_id=run.id,
            unchanged_id=(unchanged_run['id'] if unchanged_run else None),
            derived_branch_name=await derived_branch_name(conn, campaign_config, run, role),
            maintainer_email=maintainer_email,
            vcs_manager=vcs_managers[run.vcs_type],
            redis=redis,
            dry_run=dry_run,
            external_url=external_url,
            differ_url=differ_url,
            require_binary_diff=require_binary_diff,
            maintainer_rate_limiter=maintainer_rate_limiter,
            result_tags=run.result_tags,
            allow_create_proposal=run_allow_proposal_creation(campaign_config, run),
            commit_message_template=(
                campaign_config.merge_proposal.commit_message
                if campaign_config.merge_proposal else None),
        )
    except PublishFailure as e:
        code, description = await handle_publish_failure(
            e, conn, run, bucket="update-new-mp"
        )
        publish_result = PublishResult(description="Nothing to do")
        if e.code == "nothing-to-do":
            logger.info('Nothing to do.')
        else:
            logger.info("Failed(%s): %s", code, description)
    else:
        code = "success"
        description = "Success"

    if mode == MODE_ATTEMPT_PUSH:
        if publish_result.proposal_url:
            mode = MODE_PROPOSE
        else:
            mode = MODE_PUSH

    await store_publish(
        conn,
        run.package,
        publish_result.branch_name,
        base_revision,
        revision,
        role,
        mode,
        code,
        description,
        publish_result.proposal_url if publish_result.proposal_url else None,
        publish_id=publish_id,
        requestor=requestor,
    )

    if code == "success" and mode == MODE_PUSH:
        # TODO(jelmer): Call state.update_branch_status() for the
        # main branch URL
        pass

    if code == "success":
        publish_delay = datetime.utcnow() - run.finish_time
        publish_latency.observe(publish_delay.total_seconds())
    else:
        publish_delay = None

    topic_entry: Dict[str, Any] = {
        "id": publish_id,
        "package": run.package,
        "campaign": run.suite,
        "proposal_url": publish_result.proposal_url or None,
        "mode": mode,
        "main_branch_url": main_branch_url,
        "main_branch_browse_url": bzr_to_browse_url(main_branch_url),
        "branch_name": publish_result.branch_name,
        "result_code": code,
        "result": run.result,
        "run_id": run.id,
        "publish_delay": (publish_delay.total_seconds() if publish_delay else None),
    }

    await redis.publish_json('publish', topic_entry)

    if code == "success":
        return mode


def role_branch_url(url: str, remote_branch_name: Optional[str]) -> str:
    if remote_branch_name is None:
        return url
    base_url, params = urlutils.split_segment_parameters(url.rstrip("/"))
    params["branch"] = urlutils.escape(remote_branch_name, safe="")
    return urlutils.join_segment_parameters(base_url, params)


def run_allow_proposal_creation(campaign_config: Campaign, run: state.Run) -> bool:
    if (run.value is not None and campaign_config.merge_proposal is not None
            and campaign_config.merge_proposal.value_threshold):
        return (run.value >= campaign_config.merge_proposal.value_threshold)
    else:
        return True


async def publish_and_store(
    db,
    redis,
    campaign_config,
    template_env_path,
    publish_id,
    run,
    mode,
    role: str,
    maintainer_email,
    vcs_managers,
    maintainer_rate_limiter,
    dry_run,
    external_url: str,
    differ_url: str,
    allow_create_proposal: bool = True,
    require_binary_diff: bool = False,
    requestor: Optional[str] = None,
):
    remote_branch_name, base_revision, revision = run.get_result_branch(role)

    main_branch_url = role_branch_url(run.branch_url, remote_branch_name)

    if allow_create_proposal is None:
        allow_create_proposal = run_allow_proposal_creation(campaign_config, run)

    async with db.acquire() as conn:
        unchanged_run_id = await conn.fetchval(
            "SELECT id FROM run "
            "WHERE revision = $2 AND package = $1 and result_code = 'success' "
            "ORDER BY finish_time DESC LIMIT 1",
            run.package, run.main_branch_revision.decode('utf-8')
        )

        try:
            publish_result = await publish_one(
                template_env_path,
                run.suite,
                run.package,
                run.command,
                run.result,
                main_branch_url,
                mode,
                role,
                revision,
                run.id,
                unchanged_run_id,
                await derived_branch_name(conn, campaign_config, run, role),
                maintainer_email,
                vcs_managers[run.vcs_type],
                dry_run=dry_run,
                external_url=external_url,
                differ_url=differ_url,
                require_binary_diff=require_binary_diff,
                allow_create_proposal=allow_create_proposal,
                redis=redis,
                maintainer_rate_limiter=maintainer_rate_limiter,
                result_tags=run.result_tags,
                commit_message_template=(
                    campaign_config.merge_proposal.commit_message if campaign_config.merge_proposal else None),
            )
        except PublishFailure as e:
            await store_publish(
                conn,
                run.package,
                campaign_config.branch_name,
                run.main_branch_revision,
                run.revision,
                role,
                e.mode,
                e.code,
                e.description,
                None,
                publish_id=publish_id,
                requestor=requestor,
            )
            publish_entry = {
                "id": publish_id,
                "mode": e.mode,
                "result_code": e.code,
                "description": e.description,
                "package": run.package,
                "campaign": run.suite,
                "main_branch_url": run.branch_url,
                "main_branch_browse_url": bzr_to_browse_url(run.branch_url),
                "result": run.result,
            }

            await redis.publish_json('publish', publish_entry)
            return

        if mode == MODE_ATTEMPT_PUSH:
            if publish_result.proposal_url:
                mode = MODE_PROPOSE
            else:
                mode = MODE_PUSH

        await store_publish(
            conn,
            run.package,
            publish_result.branch_name,
            run.main_branch_revision,
            run.revision,
            role,
            mode,
            "success",
            "Success",
            publish_result.proposal_url if publish_result.proposal_url else None,
            publish_id=publish_id,
            requestor=requestor,
        )

        publish_delay = datetime.utcnow() - run.finish_time
        publish_latency.observe(publish_delay.total_seconds())

        publish_entry = {
            "id": publish_id,
            "package": run.package,
            "campaign": run.suite,
            "proposal_url": publish_result.proposal_url or None,
            "mode": mode,
            "main_branch_url": run.branch_url,
            "main_branch_browse_url": bzr_to_browse_url(run.branch_url),
            "branch_name": publish_result.branch_name,
            "result_code": "success",
            "result": run.result,
            "role": role,
            "publish_delay": publish_delay.total_seconds(),
            "run_id": run.id,
        }

        await redis.publish_json('publish', publish_entry)


def create_background_task(fn, title):
    loop = asyncio.get_event_loop()
    task = loop.create_task(fn)

    def log_result(future):
        try:
            future.result()
        except BaseException:
            logger.exception('%s failed', title)
        else:
            logger.debug('%s succeeded', title)
    task.add_done_callback(log_result)


async def get_publish_attempt_count(
    conn: asyncpg.Connection, revision: bytes, transient_result_codes: Set[str]
) -> int:
    return await conn.fetchval(
        "select count(*) from publish where revision = $1 "
        "and result_code != ALL($2::text[])",
        revision.decode("utf-8"),
        transient_result_codes,
    )


class BranchPublishPolicySchema(Schema):

    mode = fields.Str(description="publish mode")


class PolicySchema(Schema):

    per_branch_policy = fields.Dict(keys=fields.Str(), values=fields.Nested(BranchPublishPolicySchema))


@docs(
    responses={
        404: {"description": "Package does not exist or does not have a policy"},
        200: {"description": "Success response"}
    }
)
@response_schema(PolicySchema())
@routes.get("/{package}/{campaign}/policy", name="get-policy")
async def handle_policy_get(request):
    package = request.match_info["package"]
    campaign = request.match_info["campaign"]
    async with request.app['db'].acquire() as conn:
        row = await conn.fetchrow(
            "SELECT * "
            "FROM publish_policy WHERE package = $1 AND campaign = $2", package, campaign)
    if not row:
        return web.json_response({"reason": "Package or campaign not found"}, status=404)
    return web.json_response({
        "per_branch": {p['role']: {'mode': p['mode']} for p in row['publish']},
        "qa_policy": row['qa_policy'],
    })


@docs(
    responses={
        404: {"description": "Package does not exist or does not have a policy"},
        200: {"description": "Success response"}
    }
)
@response_schema(PolicySchema())
@routes.put("/{package}/{campaign}/policy", name="put-policy")
async def handle_policy_put(request):
    package = request.match_info["package"]
    campaign = request.match_info["campaign"]
    policy = await request.json()
    async with request.app['db'].acquire() as conn:
        await conn.execute(
            "INSERT INTO publish_policy (package, campaign, qa_review, per_branch_policy) "
            "VALUES ($1, $2, $3, $4) ON CONFLICT (package, campaign) "
            "DO UPDATE SET qa_review = EXCLUDED.qa_review, "
            "per_branch_policy = EXCLUDED.per_branch_policy", package, campaign,
            policy['qa_review'], policy['per_branch'])
    # TODO(jelmer): Call consider_publish_run
    return web.json_response({})


@docs(
    responses={
        404: {"description": "Package does not exist or does not have a policy"},
        200: {"description": "Success response"}
    }
)
@response_schema(PolicySchema())
@routes.delete("/{package}/{campaign}/policy", name="delete-policy")
async def handle_policy_del(request):
    package = request.match_info["package"]
    campaign = request.match_info["campaign"]
    async with request.app['db'].acquire() as conn:
        await conn.execute(
            "DELETE FROM publish_policy WHERE package = $1 AND campaign = $2",
            package, campaign)
    return web.json_response({})


@routes.post("/merge-proposal", name="merge-proposal")
async def update_merge_proposal_request(request):
    post = await request.post()
    async with request.app['db'].acquire() as conn:
        await conn.execute(
            "UPDATE merge_proposal SET status = $1 WHERE url = $2",
            post['status'], post['url'])


@routes.post("/consider/{run_id}", name="consider")
async def consider_request(request):
    run_id = request.match_info['run_id']

    # TODO(jelmer): Allow this to vary?
    review_status = ["approved"]

    async def run():
        async with request.app['db'].acquire() as conn:
            async for (run, maintainer_email, command, unpublished_branches) in iter_publish_ready(
                    conn, review_status=review_status,
                    needs_review=False, run_id=run_id):
                break
            else:
                return
            await consider_publish_run(
                conn, request.app['config'],
                template_env_path=request.app['template_env_path'],
                vcs_managers=request.app['vcs_managers'],
                maintainer_rate_limiter=request.app['maintainer_rate_limiter'],
                external_url=request.app['external_url'],
                differ_url=request.app['differ_url'],
                redis=request.app['redis'],
                run=run,
                command=command,
                maintainer_email=maintainer_email,
                unpublished_branches=unpublished_branches,
                require_binary_diff=request.app['require_binary_diff'],
                dry_run=request.app['dry_run'])
    create_background_task(
        run(), 'consider publishing %s' % run_id)
    return web.json_response({}, status=200)


async def get_publish_policy(conn: asyncpg.Connection, package: str, campaign: str):
    row = await conn.fetchrow(
        "SELECT publish, command "
        "FROM policy WHERE package = $1 AND suite = $2",
        package,
        campaign,
    )
    if row:
        return (
            {v['role']: (v['mode'], v['frequency_days']) for v in row['publish']},
            row['command']
        )
    return None, None, None


@routes.post("/{campaign}/{package}/publish", name='publish')
async def publish_request(request):
    dry_run = request.app['dry_run']
    vcs_managers = request.app['vcs_managers']
    maintainer_rate_limiter = request.app['maintainer_rate_limiter']
    package = request.match_info["package"]
    campaign = request.match_info["campaign"]
    role = request.query.get("role")
    post = await request.post()
    mode = post.get("mode")
    async with request.app['db'].acquire() as conn:
        package = await conn.fetchrow(
            'SELECT name, maintainer_email FROM package WHERE name = $1',
            package)
        if package is None:
            return web.json_response({}, status=400)

        run = await get_last_effective_run(conn, package['name'], campaign)
        if run is None:
            return web.json_response({}, status=400)

        publish_policy = (await get_publish_policy(conn, package['name'], campaign))[0]

        logger.info("Handling request to publish %s/%s", package['name'], campaign)

    if role is not None:
        roles = [role]
    else:
        roles = [e[0] for e in run.result_branches]

    if mode:
        branches = [(r, mode) for r in roles]
    else:
        branches = [(r, publish_policy.get(r, (MODE_SKIP, None))[0]) for r in roles]

    publish_ids = {}
    for role, mode in branches:
        publish_id = str(uuid.uuid4())
        publish_ids[role] = publish_id

        logger.info(".. publishing for role %s: %s", role, mode)

        if mode in (MODE_SKIP, MODE_BUILD_ONLY):
            continue

        create_background_task(
            publish_and_store(
                request.app['db'],
                request.app['redis'],
                get_campaign_config(request.app['config'], run.suite),
                request.app['template_env_path'],
                publish_id,
                run,
                mode,
                role,
                package['maintainer_email'],
                vcs_managers=vcs_managers,
                maintainer_rate_limiter=maintainer_rate_limiter,
                dry_run=dry_run,
                external_url=request.app['external_url'],
                differ_url=request.app['differ_url'],
                allow_create_proposal=True,
                require_binary_diff=False,
                requestor=post.get("requestor"),
            ), 'publish of %s/%s, role %s' % (package['name'], campaign, role)
        )

    if not publish_ids:
        return web.json_response(
            {"run_id": run.id, "code": "done", "description": "Nothing to do"}
        )

    return web.json_response(
        {"run_id": run.id, "mode": mode, "publish_ids": publish_ids}, status=202
    )


@routes.get("/credentials", name='credentials')
async def credentials_request(request):
    ssh_keys = []
    for entry in os.scandir(os.path.expanduser("~/.ssh")):
        if entry.name.endswith(".pub"):
            with open(entry.path, "r") as f:
                ssh_keys.extend([line.strip() for line in f.readlines()])
    pgp_keys = []
    for entry in list(request.app['gpg'].keylist(secret=True)):
        pgp_keys.append(request.app['gpg'].key_export_minimal(entry.fpr).decode())
    hosting = []
    for name, forge_cls in forges.items():
        for instance in forge_cls.iter_instances():
            try:
                current_user = instance.get_current_user()
            except ForgeLoginRequired:
                continue
            except UnsupportedForge:
                # WTF? Well, whatever.
                continue
            if current_user:
                current_user_url = instance.get_user_url(current_user)
            else:
                current_user_url = None
            forge = {
                "kind": name,
                "name": instance.name,
                "url": instance.base_url,
                "user": current_user,
                "user_url": current_user_url,
            }
            hosting.append(forge)

    return web.json_response(
        {
            "ssh_keys": ssh_keys,
            "pgp_keys": pgp_keys,
            "hosting": hosting,
        }
    )


async def run_web_server(
    listen_addr: str,
    port: int,
    template_env_path: Optional[str],
    maintainer_rate_limiter: RateLimiter,
    forge_rate_limiter: Dict[str, datetime],
    vcs_managers: Dict[str, VcsManager],
    db: asyncpg.pool.Pool,
    redis,
    config,
    dry_run: bool,
    external_url: str,
    differ_url: str,
    require_binary_diff: bool = False,
    push_limit: Optional[int] = None,
    modify_mp_limit: Optional[int] = None,
):
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[trailing_slash_redirect])
    app.router.add_routes(routes)
    app['gpg'] = gpg.Context(armor=True)
    app['template_env_path'] = template_env_path
    app['vcs_managers'] = vcs_managers
    app['db'] = db
    app['redis'] = redis
    app['config'] = config
    app['external_url'] = external_url
    app['differ_url'] = differ_url
    app['maintainer_rate_limiter'] = maintainer_rate_limiter
    app['forge_rate_limiter'] = forge_rate_limiter
    app['modify_mp_limit'] = modify_mp_limit
    app['dry_run'] = dry_run
    app['push_limit'] = push_limit
    app['require_binary_diff'] = require_binary_diff
    setup_metrics(app)
    endpoint = aiozipkin.create_endpoint("janitor.publish", ipv4=listen_addr, port=port)
    if config.zipkin_address:
        tracer = await aiozipkin.create(config.zipkin_address, endpoint, sample_rate=1.0)
    else:
        tracer = await aiozipkin.create_custom(endpoint)
    setup_aiohttp_apispec(
        app=app,
        title="Publish Documentation",
        version=None,
        url="/swagger.json",
        swagger_path="/docs",
    )

    aiozipkin.setup(app, tracer)
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    logger.info("Listening on %s:%s", listen_addr, port)
    await site.start()


@routes.get("/health", name="health")
async def handle_health(request):
    return web.Response(text="OK")


@routes.get("/ready", name="ready")
async def handle_ready(request):
    return web.Response(text="OK")


async def get_mp_status(mp):
    if await to_thread(mp.is_merged):
        return "merged"
    elif await to_thread(mp.is_closed):
        return "closed"
    else:
        return "open"


@routes.post("/check-proposal", name='check-proposal')
async def check_mp_request(request):
    post = await request.post()
    url = post["url"]
    try:
        mp = await to_thread(get_proposal_by_url, url)
    except UnsupportedForge:
        raise web.HTTPNotFound()
    status = await get_mp_status(mp)
    async with request.app['db'].acquire() as conn:
        try:
            modified = await check_existing_mp(
                conn,
                request.app['redis'],
                request.app['config'],
                request.app['template_env_path'],
                mp,
                status,
                vcs_managers=request.app['vcs_managers'],
                dry_run=("dry_run" in post),
                external_url=request.app['external_url'],
                differ_url=request.app['differ_url'],
                maintainer_rate_limiter=request.app['maintainer_rate_limiter'],
            )
        except NoRunForMergeProposal as e:
            return web.Response(
                status=500,
                text="Unable to find stored metadata for %s (%r), skipping."
                % (e.mp.url, e.revision),
            )
    if modified:
        return web.Response(status=200, text="Merge proposal updated.")
    else:
        return web.Response(status=200, text="Merge proposal not updated.")


@routes.post("/scan", name='scan')
async def scan_request(request):
    async def scan():
        async with request.app['db'].acquire() as conn:
            await check_existing(
                conn,
                request.app['redis'],
                request.app['config'],
                request.app['template_env_path'],
                request.app['maintainer_rate_limiter'],
                request.app['forge_rate_limiter'],
                request.app['vcs_managers'],
                dry_run=request.app['dry_run'],
                differ_url=request.app['differ_url'],
                external_url=request.app['external_url'],
                modify_limit=request.app['modify_mp_limit'],
            )

    create_background_task(scan(), 'merge proposal refresh scan')
    return web.Response(status=202, text="Scan started.")


@routes.post("/refresh-status", name='refresh-status')
async def refresh_proposal_status_request(request):
    post = await request.post()
    try:
        url = post["url"]
    except KeyError:
        raise web.HTTPBadRequest(body="missing url parameter")
    logger.info("Request to refresh proposal status for %s", url)

    async def scan():
        mp = await to_thread(get_proposal_by_url, url)
        async with request.app['db'].acquire() as conn:
            status = await get_mp_status(mp)
            try:
                await check_existing_mp(
                    conn,
                    request.app['redis'],
                    request.app['config'],
                    request.app['template_env_path'],
                    mp,
                    status,
                    vcs_managers=request.app['vcs_managers'],
                    maintainer_rate_limiter=request.app['maintainer_rate_limiter'],
                    dry_run=request.app['dry_run'],
                    differ_url=request.app['differ_url'],
                    external_url=request.app['external_url'],
                )
            except NoRunForMergeProposal as e:
                logger.warning(
                    "Unable to find stored metadata for %s, skipping.", e.mp.url
                )
    create_background_task(scan(), 'Refresh of proposal %s' % url)
    return web.Response(status=202, text="Refresh of proposal started.")


@routes.post("/autopublish", name='autopublish')
async def autopublish_request(request):
    reviewed_only = "unreviewed" not in request.query

    async def autopublish():
        await publish_pending_ready(
            request.app['db'],
            request.app['redis'],
            request.app['config'],
            request.app['template_env_path'],
            request.app['maintainer_rate_limiter'],
            request.app['vcs_managers'],
            dry_run=request.app['dry_run'],
            external_url=request.app['external_url'],
            differ_url=request.app['differ_url'],
            reviewed_only=reviewed_only,
            push_limit=request.app['push_limit'],
            require_binary_diff=request.app['require_binary_diff'],
        )

    create_background_task(autopublish(), 'autopublish')
    return web.Response(status=202, text="Autopublish started.")


@routes.get("/rate-limits/{maintainer}", name="maintainer-rate-limits")
async def maintainer_rate_limits_request(request):
    maintainer_rate_limiter = request.app['maintainer_rate_limiter']

    stats = maintainer_rate_limiter.get_stats()

    (current_open, max_open) = stats.get(
        request.match_info['maintainer'], (None, None))

    ret = {
        'open': current_open,
        'max_open': max_open,
        'remaining':
            None if (current_open is None or max_open is None)
            else max_open - current_open}

    return web.json_response(ret)


@routes.get("/rate-limits", name="rate-limits")
async def maintainer_rate_limits_request(request):
    maintainer_rate_limiter = request.app['maintainer_rate_limiter']

    per_maintainer = {}
    for email, (current_open, max_open) in (
            maintainer_rate_limiter.get_stats().items()):
        per_maintainer[email] = {
            'open': current_open,
            'max_open': max_open,
            'remaining': (
                None if (current_open is None or max_open is None)
                else max_open - current_open)}

    return web.json_response({
        'proposals_per_maintainer': per_maintainer,
        'per_forge': {
            str(f): dt.isoformat()
            for f, dt in request.app['forge_rate_limiter'].items()},
        'push_limit': request.app['push_limit']})


@routes.get("/blockers/{run_id}", name='blockers')
async def blockers_request(request):
    async with request.app['db'].acquire() as conn:
        run = await conn.fetchrow("""\
SELECT
  run.finish_time AS finish_time,
  run.review_status AS review_status,
  run.command AS run_command,
  review_comment AS review_comment,
  policy.qa_review AS qa_review_policy,
  package.maintainer_email AS maintainer_email,
  run.revision AS revision,
  policy.command AS policy_command,
  package.removed AS removed,
  run.result_code AS result_code
FROM run
LEFT JOIN package ON package.name = run.package
INNER JOIN policy ON policy.package = run.package
   AND policy.suite = run.suite
WHERE id = $1
""", request.match_info['run_id'])

        if run is None:
            return web.json_response({
                'reason': 'No such publish-ready run',
                'run_id': request.match_info['run_id']}, status=404)

        if run['revision'] is not None:
            attempt_count = await get_publish_attempt_count(
                conn, run['revision'].encode('utf-8'),
                {"differ-unreachable"})
        else:
            attempt_count = 0
    ret = {}
    ret['success'] = {
        'result': (run['result_code'] == 'success'),
        'details': {'result_code': run['result_code']}}
    ret['removed'] = {
        'result': not run['removed'],
        'details': {'removed': run['removed']}}
    ret['command'] = {
        'result': run['run_command'] == run['policy_command'],
        'details': {
            'correct': run['policy_command'],
            'actual': run['run_command']}}
    ret['qa_review'] = {
        'result': (
            run['review_status'] != 'rejected'
            and not (run['qa_review_policy'] == 'required'
                     and run['review_status'] == 'unreviewed')),
        'details': {
            'status': run['review_status'],
            'comment': run['review_comment'],
            'needs_review': (
                run['qa_review_policy'] == 'required'
                and run['review_status'] == 'unreviewed'),
            'policy': run['qa_review_policy']}}

    next_try_time = calculate_next_try_time(
        run['finish_time'], attempt_count)
    ret['backoff'] = {
        'result': datetime.utcnow() >= next_try_time,
        'details': {
            'attempt_count': attempt_count,
            'next_try_time': next_try_time.isoformat()}}

    # TODO(jelmer): include forge rate limits?

    ret['maintainer_propose_rate_limit'] = {
        'details': {
            'maintainer': run['maintainer_email']}}
    try:
        request.app['maintainer_rate_limiter'].check_allowed(run['maintainer_email'])
    except MaintainerRateLimited as e:
        ret['maintainer_propose_rate_limit']['result'] = False
        ret['maintainer_propose_rate_limit']['details'] = {
            'open': e.open_mps,
            'max_open': e.max_open_mps}
    except RateLimited:
        ret['maintainer_propose_rate_limit']['result'] = False
    else:
        ret['maintainer_propose_rate_limit']['result'] = True
    return web.json_response(ret)


async def process_queue_loop(
    db,
    redis,
    config,
    template_env_path,
    maintainer_rate_limiter,
    forge_rate_limiter,
    dry_run,
    vcs_managers,
    interval,
    external_url: str,
    differ_url: str,
    auto_publish: bool = True,
    reviewed_only: bool = False,
    push_limit: Optional[int] = None,
    modify_mp_limit: Optional[int] = None,
    require_binary_diff: bool = False,
):
    while True:
        cycle_start = datetime.utcnow()
        async with db.acquire() as conn:
            await check_existing(
                conn,
                redis,
                config,
                template_env_path,
                maintainer_rate_limiter,
                forge_rate_limiter,
                vcs_managers,
                dry_run=dry_run,
                external_url=external_url,
                differ_url=differ_url,
                modify_limit=modify_mp_limit,
            )
        if auto_publish:
            await publish_pending_ready(
                db,
                redis,
                config,
                template_env_path,
                maintainer_rate_limiter,
                vcs_managers,
                dry_run=dry_run,
                external_url=external_url,
                differ_url=differ_url,
                reviewed_only=reviewed_only,
                push_limit=push_limit,
                require_binary_diff=require_binary_diff,
            )
        cycle_duration = datetime.utcnow() - cycle_start
        to_wait = max(0, interval - cycle_duration.total_seconds())
        logger.info("Waiting %d seconds for next cycle." % to_wait)
        if to_wait > 0:
            await asyncio.sleep(to_wait)


async def is_conflicted(mp):
    try:
        return not await to_thread(mp.can_be_merged)
    except NotImplementedError:
        # TODO(jelmer): Download and attempt to merge locally?
        return None


class NoRunForMergeProposal(Exception):
    """No run matching merge proposal."""

    def __init__(self, mp, revision):
        self.mp = mp
        self.revision = revision


async def get_last_effective_run(conn, package, campaign):
    last_success = False
    query = """
SELECT
    id, command, start_time, finish_time, description, package,
    result_code,
    value, main_branch_revision, revision, context, result, suite,
    instigated_context, vcs_type, branch_url, logfilenames, review_status,
    review_comment, worker,
    array(SELECT row(role, remote_name, base_revision,
     revision) FROM new_result_branch WHERE run_id = id) AS result_branches,
    result_tags, target_branch_url, change_set
FROM
    run
LEFT JOIN
    debian_build ON debian_build.run_id = run.id
WHERE package = $1 AND suite = $2
ORDER BY finish_time DESC
"""
    for row in await conn.fetch(query, package, campaign):
        run = state.Run.from_row(row)
        if run.result_code in ("success", "nothing-to-do"):
            return run
        elif run.result_code == "nothing-new-to-do":
            last_success = True
            continue
        elif not last_success:
            return run
    else:
        return None


async def get_merge_proposal_run(
        conn: asyncpg.Connection, mp_url: str) -> asyncpg.Record:
    query = """
SELECT
    run.id AS id,
    run.package AS package,
    run.suite AS campaign,
    run.branch_url AS branch_url,
    run.command AS command,
    rb.role AS role,
    rb.remote_name AS remote_branch_name,
    rb.revision AS revision
FROM new_result_branch rb
RIGHT JOIN run ON rb.run_id = run.id
WHERE rb.revision IN (
    SELECT revision from merge_proposal WHERE merge_proposal.url = $1)
ORDER BY run.finish_time DESC
LIMIT 1
"""
    return await conn.fetchrow(query, mp_url)


async def get_proposal_info(
    conn: asyncpg.Connection, url
) -> Tuple[Optional[bytes], str, str, str, str]:
    row = await conn.fetchrow(
        """\
SELECT
    package.maintainer_email,
    merge_proposal.revision,
    merge_proposal.status,
    merge_proposal.target_branch_url,
    package.name
FROM
    merge_proposal
LEFT JOIN package ON merge_proposal.package = package.name
WHERE
    merge_proposal.url = $1
""",
        url,
    )
    if not row:
        raise KeyError
    return (row[1].encode("utf-8") if row[1] else None, row[2], row[3], row[4], row[0])


async def guess_package_from_revision(
    conn: asyncpg.Connection, revision: bytes
) -> Tuple[Optional[str], Optional[str]]:
    query = """\
select distinct package, maintainer_email from run
left join new_result_branch rb ON rb.run_id = run.id
left join package on package.name = run.package
where rb.revision = $1 and run.package is not null
"""
    rows = await conn.fetch(query, revision.decode("utf-8"))
    if len(rows) == 1:
        return rows[0][0], rows[0][1]
    return None, None


async def guess_package_from_branch_url(
        conn: asyncpg.Connection, url: str, possible_transports=None):
    query = """
SELECT
  name, maintainer_email, branch_url
FROM
  package
WHERE
  branch_url = ANY($1::text[])
ORDER BY length(branch_url) DESC
"""
    options = [
        url.rstrip('/'),
        url.rstrip('/') + '/',
        urlutils.split_segment_parameters(url.rstrip('/'))[0],
        urlutils.split_segment_parameters(url.rstrip('/'))[0] + '/'
    ]
    result = await conn.fetchrow(query, options)
    if result is None:
        return None

    source_branch = await to_thread(
        open_branch,
        result['branch_url'], possible_transports=possible_transports)
    if source_branch.user_url.rstrip('/') != url.rstrip('/'):
        return None
    return result


async def check_existing_mp(
    conn,
    redis,
    config,
    template_env_path,
    mp,
    status,
    vcs_managers,
    maintainer_rate_limiter,
    dry_run: bool,
    external_url: str,
    differ_url: str,
    mps_per_maintainer=None,
    possible_transports: Optional[List[Transport]] = None,
    check_only: bool = False,
) -> bool:
    async def update_proposal_status(
            mp, status, revision, package_name, target_branch_url,
            campaign):
        if status == "closed":
            # TODO(jelmer): Check if changes were applied manually and mark
            # as applied rather than closed?
            pass
        if status == "merged":
            merged_by = mp.get_merged_by()
            merged_at = mp.get_merged_at()
            if merged_at is not None:
                merged_at = merged_at.replace(tzinfo=None)
        else:
            merged_by = None
            merged_at = None
        if not dry_run:
            async with conn.transaction():
                await conn.execute(
                    """INSERT INTO merge_proposal (
                        url, status, revision, package, merged_by, merged_at,
                        target_branch_url)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (url)
                    DO UPDATE SET
                      status = EXCLUDED.status,
                      revision = EXCLUDED.revision,
                      package = EXCLUDED.package,
                      merged_by = EXCLUDED.merged_by,
                      merged_at = EXCLUDED.merged_at,
                      target_branch_url = EXCLUDED.target_branch_url
                    """, mp.url, status,
                    revision.decode("utf-8") if revision is not None else None,
                    package_name, merged_by, merged_at, target_branch_url)
                if revision:
                    await conn.execute("""
                    UPDATE new_result_branch SET absorbed = $1 WHERE revision = $2
                    """, (status == 'merged'), revision.decode('utf-8'))

            # TODO(jelmer): Check if the change_set should be marked as published

            await redis.publish_json(
                'merge-proposal', {
                "url": mp.url,
                "target_branch_url": target_branch_url,
                "status": status,
                "package": package_name,
                "merged_by": merged_by,
                "merged_at": str(merged_at),
                "campaign": campaign,
            })

    old_status: Optional[str]
    maintainer_email: Optional[str]
    package_name: Optional[str]
    try:
        (
            old_revision,
            old_status,
            old_target_branch_url,
            package_name,
            maintainer_email,
        ) = await get_proposal_info(conn, mp.url)
    except KeyError:
        old_revision = None
        old_status = None
        old_target_branch_url = None
        maintainer_email = None
        package_name = None
    revision = await to_thread(mp.get_source_revision)
    source_branch_url = await to_thread(mp.get_source_branch_url)
    if revision is None:
        if source_branch_url is None:
            logger.warning("No source branch for %r", mp)
            revision = None
            source_branch_name = None
        else:
            try:
                source_branch = await to_thread(
                    open_branch,
                    source_branch_url, possible_transports=possible_transports)
            except (BranchMissing, BranchUnavailable):
                revision = None
                source_branch_name = None
            else:
                revision = await to_thread(source_branch.last_revision)
                source_branch_name = source_branch.name
    else:
        source_branch_name = None
    if source_branch_name is None and source_branch_url is not None:
        segment_params = urlutils.split_segment_parameters(source_branch_url)[1]
        source_branch_name = segment_params.get("branch")
        if source_branch_name is not None:
            source_branch_name = urlutils.unescape(source_branch_name)
    if revision is None:
        revision = old_revision
    target_branch_url = await to_thread(mp.get_target_branch_url)
    if maintainer_email is None:
        row = await guess_package_from_branch_url(
            conn, target_branch_url,
            possible_transports=possible_transports)
        if row is not None:
            maintainer_email = row['maintainer_email']
            package_name = row['name']
        else:
            if revision is not None:
                (
                    package_name,
                    maintainer_email,
                ) = await guess_package_from_revision(conn, revision)
            if package_name is None:
                logger.warning(
                    "No package known for %s (%s)", mp.url, target_branch_url
                )
            else:
                logger.info(
                    "Guessed package name (%s) for %s based on revision.",
                    package_name,
                    mp.url,
                )
    if old_status in ("abandoned", "applied", "rejected") and status == "closed":
        status = old_status
    if (old_status != status
            or revision != old_revision
            or target_branch_url != old_target_branch_url):
        mp_run = await get_merge_proposal_run(conn, mp.url)
        await update_proposal_status(
            mp, status, revision, package_name, target_branch_url,
            campaign=mp_run['campaign'])
    else:
        mp_run = None
    if maintainer_email is not None and mps_per_maintainer is not None:
        mps_per_maintainer[status].setdefault(maintainer_email, 0)
        mps_per_maintainer[status][maintainer_email] += 1
    if status != "open":
        return False
    if check_only:
        return False

    if mp_run is None:
        mp_run = await get_merge_proposal_run(conn, mp.url)
    if mp_run is None:
        raise NoRunForMergeProposal(mp, revision)

    mp_remote_branch_name = mp_run['remote_branch_name']

    if mp_remote_branch_name is None:
        target_branch_url = await to_thread(mp.get_target_branch_url)
        if target_branch_url is None:
            logger.warning("No target branch for %r", mp)
        else:
            try:
                mp_remote_branch_name = (await to_thread(
                    open_branch,
                    target_branch_url, possible_transports=possible_transports)
                ).name
            except (BranchMissing, BranchUnavailable):
                pass

    last_run = await get_last_effective_run(conn, mp_run['package'], mp_run['campaign'])
    if last_run is None:
        logger.warning("%s: Unable to find any relevant runs.", mp.url)
        return False

    removed = await conn.fetchval(
        'SELECT removed FROM package WHERE name = $1', mp_run['package'])
    if removed is None:
        logger.warning("%s: Unable to find package.", mp.url)
        return False

    if removed:
        logger.info(
            "%s: package has been removed from the archive, " "closing proposal.",
            mp.url,
        )
        if not dry_run:
            try:
                await to_thread(
                    mp.post_comment,
                    """
This merge proposal will be closed, since the package has been removed from the
archive.
"""
                )
            except PermissionDenied as e:
                logger.warning(
                    "Permission denied posting comment to %s: %s", mp.url, e
                )
            try:
                await to_thread(mp.close)
            except PermissionDenied as e:
                logger.warning(
                    "Permission denied closing merge request %s: %s", mp.url, e
                )
                return False
            return True

    if last_run.result_code == "nothing-to-do":
        # A new run happened since the last, but there was nothing to
        # do.
        logger.info(
            "%s: Last run did not produce any changes, " "closing proposal.", mp.url
        )
        if not dry_run:
            await update_proposal_status(
                mp, "applied", revision, package_name, target_branch_url,
                mp_run['campaign'])
            try:
                await to_thread(
                    mp.post_comment,
                    """
This merge proposal will be closed, since all remaining changes have been
applied independently.
"""
                )
            except PermissionDenied as e:
                logger.warning(
                    "Permission denied posting comment to %s: %s", mp.url, e
                )
            try:
                await to_thread(mp.close)
            except PermissionDenied as e:
                logger.warning(
                    "Permission denied closing merge request %s: %s", mp.url, e
                )
                return False
        return True

    if last_run.result_code != "success":
        last_run_age = datetime.utcnow() - last_run.finish_time
        if last_run.result_code in TRANSIENT_ERROR_RESULT_CODES:
            logger.info(
                "%s: Last run failed with transient error (%s). " "Rescheduling.",
                mp.url,
                last_run.result_code,
            )
            try:
                await do_schedule(
                    conn,
                    last_run.package,
                    last_run.suite,
                    change_set=last_run.change_set,
                    bucket="update-existing-mp",
                    refresh=False,
                    requestor="publisher (transient error)",
                )
            except PolicyUnavailable as e:
                logging.warning(
                    'Policy unavailable while attempting to reschedule %s/%s: %s',
                    last_run.package, last_run.suite, e)
        elif last_run_age.days > EXISTING_RUN_RETRY_INTERVAL:
            logger.info(
                "%s: Last run failed (%s) a long time ago (%d days). " "Rescheduling.",
                mp.url,
                last_run.result_code,
                last_run_age.days,
            )
            await do_schedule(
                conn,
                last_run.package,
                last_run.suite,
                change_set=last_run.change_set,
                bucket="update-existing-mp",
                refresh=False,
                requestor="publisher (retrying failed run after %d days)"
                % last_run_age.days,
            )
        else:
            logger.info(
                "%s: Last run failed (%s). Not touching merge proposal.",
                mp.url,
                last_run.result_code,
            )
        return False

    if maintainer_email is None:
        logger.info("%s: No maintainer email known.", mp.url)
        return False

    try:
        (
            last_run_remote_branch_name,
            last_run_base_revision,
            last_run_revision,
        ) = last_run.get_result_branch(mp_run['role'])
    except KeyError:
        logger.warning(
            "%s: Merge proposal run %s had role %s" " but it is gone now (%s)",
            mp.url,
            mp_run['id'],
            mp_run['role'],
            last_run.id,
        )
        return False

    if (
        last_run_remote_branch_name != mp_remote_branch_name
        and last_run_remote_branch_name is not None
    ):
        logger.warning(
            "%s: Remote branch name has changed: %s => %s, " "skipping...",
            mp.url,
            mp_remote_branch_name,
            last_run_remote_branch_name,
        )
        # Note that we require that mp_remote_branch_name is set.
        # For some old runs it is not set because we didn't track
        # the default branch name.
        if not dry_run and mp_remote_branch_name is not None:
            try:
                mp.set_target_branch_name(last_run_remote_branch_name or "")
            except NotImplementedError:
                logger.info(
                    "%s: Closing merge proposal, since branch for role "
                    "'%s' has changed from %s to %s.",
                    mp.url,
                    mp_run['role'],
                    mp_remote_branch_name,
                    last_run_remote_branch_name,
                )
                await update_proposal_status(
                    mp, "abandoned", revision, package_name, target_branch_url,
                    campaign=mp_run['campaign'])
                try:
                    await to_thread(mp.post_comment, """
This merge proposal will be closed, since the branch for the role '%s'
has changed from %s to %s.
""" % (mp_run['role'], mp_remote_branch_name, last_run_remote_branch_name))
                except PermissionDenied as e:
                    logger.warning(
                        "Permission denied posting comment to %s: %s", mp.url, e
                    )
                try:
                    await to_thread(mp.close)
                except PermissionDenied as e:
                    logger.warning(
                        "Permission denied closing merge request %s: %s", mp.url, e
                    )
                    return False
            else:
                return True
        return False

    if not await to_thread(branches_match, mp_run['branch_url'], last_run.branch_url):
        logger.warning(
            "%s: Remote branch URL appears to have have changed: "
            "%s => %s, skipping.",
            mp.url,
            mp_run['branch_url'],
            last_run.branch_url,
        )
        return False

        # TODO(jelmer): Don't do this if there's a redirect in place,
        # or if one of the branches has a branch name included and the other
        # doesn't
        if not dry_run:
            await update_proposal_status(
                mp, "abandoned", revision, package_name, target_branch_url,
                campaign=mp_run['campaign'])
            try:
                await to_thread(
                    mp.post_comment,
                    """
This merge proposal will be closed, since the branch has moved to %s.
"""
                    % (bzr_to_browse_url(last_run.branch_url),)
                )
            except PermissionDenied as e:
                logger.warning(
                    "Permission denied posting comment to %s: %s", mp.url, e
                )
            try:
                await to_thread(mp.close)
            except PermissionDenied as e:
                logger.warning(
                    "Permission denied closing merge request %s: %s", mp.url, e
                )
                return False
        return False

    if last_run.id != mp_run['id']:
        publish_id = str(uuid.uuid4())
        logger.info(
            "%s (%s) needs to be updated (%s => %s).",
            mp.url,
            mp_run['package'],
            mp_run['id'],
            last_run.id,
        )
        if last_run_revision == mp_run['revision'].encode('utf-8'):
            logger.warning(
                "%s (%s): old run (%s/%s) has same revision as new run (%s/%s)" ": %r",
                mp.url,
                mp_run['package'],
                mp_run['id'],
                mp_run['role'],
                last_run.id,
                mp_run['role'],
                mp_run['revision'].encode('utf-8'),
            )
        campaign_config = get_campaign_config(config, mp_run['campaign'])
        if source_branch_name is None:
            source_branch_name = await derived_branch_name(conn, campaign_config, last_run, mp_run['role'])

        unchanged_run_id = await conn.fetchval(
            "SELECT id FROM run "
            "WHERE revision = $2 AND package = $1 and result_code = 'success' "
            "ORDER BY finish_time DESC LIMIT 1",
            last_run.package, last_run.main_branch_revision.decode('utf-8')
        )

        try:
            publish_result = await publish_one(
                template_env_path,
                last_run.suite,
                last_run.package,
                last_run.command,
                last_run.result,
                role_branch_url(mp_run['branch_url'], mp_remote_branch_name),
                MODE_PROPOSE,
                mp_run['role'],
                last_run_revision,
                last_run.id,
                unchanged_run_id,
                source_branch_name,
                maintainer_email,
                vcs_manager=vcs_managers[last_run.vcs_type],
                dry_run=dry_run,
                external_url=external_url,
                differ_url=differ_url,
                require_binary_diff=False,
                allow_create_proposal=True,
                redis=redis,
                maintainer_rate_limiter=maintainer_rate_limiter,
                result_tags=last_run.result_tags,
                commit_message_template=(
                    campaign_config.merge_proposal.commit_message if campaign_config.merge_proposal else None),
            )
        except PublishFailure as e:
            code, description = await handle_publish_failure(
                e, conn, last_run, bucket="update-existing-mp"
            )
            if code == "empty-merge-proposal":
                # The changes from the merge proposal have already made it in
                # somehow.
                logger.info(
                    "%s: Empty merge proposal, changes must have been merged "
                    "some other way. Closing.",
                    mp.url,
                )
                if not dry_run:
                    await update_proposal_status(
                        mp, "applied", revision, package_name,
                        target_branch_url, campaign=mp_run['campaign'])
                    try:
                        await to_thread(
                            mp.post_comment,
                            """
This merge proposal will be closed, since all remaining changes have been
applied independently.
"""
                        )
                    except PermissionDenied as f:
                        logger.warning(
                            "Permission denied posting comment to %s: %s", mp.url, f
                        )
                    try:
                        await to_thread(mp.close)
                    except PermissionDenied as f:
                        logger.warning(
                            "Permission denied closing merge request %s: %s", mp.url, f
                        )
                        code = "empty-failed-to-close"
                        description = "Permission denied closing merge request: %s" % f
                code = "success"
                description = (
                    "Closing merge request for which changes were "
                    "applied independently"
                )
            if code != "success":
                logger.info(
                    "%s: Updating merge proposal failed: %s (%s)",
                    mp.url,
                    code,
                    description,
                )
            if not dry_run:
                await store_publish(
                    conn,
                    last_run.package,
                    campaign_config.branch_name,
                    last_run_base_revision,
                    last_run_revision,
                    mp_run['role'],
                    e.mode,
                    code,
                    description,
                    mp.url,
                    publish_id=publish_id,
                    requestor="publisher (regular refresh)",
                )
        else:
            if not dry_run:
                await store_publish(
                    conn,
                    last_run.package,
                    publish_result.branch_name,
                    last_run_base_revision,
                    last_run_revision,
                    mp_run['role'],
                    MODE_PROPOSE,
                    "success",
                    publish_result.description or "Succesfully updated",
                    publish_result.proposal_url,
                    publish_id=publish_id,
                    requestor="publisher (regular refresh)",
                )

            if publish_result.is_new:
                # This can happen when the default branch changes
                logger.warning(
                    "Intended to update proposal %r, but created %r", mp.url, publish_result.proposal_url
                )
        return True
    else:
        # It may take a while for the 'conflicted' bit on the proposal to
        # be refreshed, so only check it if we haven't made any other
        # changes.
        if await is_conflicted(mp):
            logger.info("%s is conflicted. Rescheduling.", mp.url)
            if not dry_run:
                try:
                    await do_schedule(
                        conn,
                        mp_run['package'],
                        mp_run['campaign'],
                        change_set=last_run.change_set,
                        bucket="update-existing-mp",
                        refresh=True,
                        requestor="publisher (merge conflict)",
                    )
                except PolicyUnavailable:
                    logging.warning(
                        'Policy unavailable while attempting to reschedule '
                        'conflicted %s/%s',
                        mp_run['package'], mp_run['campaign'])
        return False


def iter_all_mps(
    statuses: Optional[List[str]] = None,
) -> Iterator[Tuple[Forge, MergeProposal, str]]:
    """iterate over all existing merge proposals."""
    if statuses is None:
        statuses = ["open", "merged", "closed"]
    for instance in iter_forge_instances():
        for status in statuses:
            try:
                for mp in instance.iter_my_proposals(status=status):
                    yield instance, mp, status
            except ForgeLoginRequired:
                logging.info(
                    'Skipping %r, no credentials known.',
                    instance)
            except UnexpectedHttpStatus as e:
                logging.warning(
                    'Got unexpected HTTP status %s, skipping %r',
                    e, instance)
            except UnsupportedForge as e:
                logging.warning(
                    'Unsupported host instance, skipping %r: %s',
                    instance, e)


async def check_existing(
    conn,
    redis,
    config,
    template_env_path,
    maintainer_rate_limiter,
    forge_rate_limiter: Dict[Forge, datetime],
    vcs_managers,
    dry_run: bool,
    external_url: str,
    differ_url: str,
    modify_limit=None,
    unexpected_limit=5,
):
    mps_per_maintainer: Dict[str, Dict[str, int]] = {
        "open": {},
        "closed": {},
        "merged": {},
        "applied": {},
        "abandoned": {},
        "rejected": {},
    }
    possible_transports: List[Transport] = []
    status_count = {
        "open": 0,
        "closed": 0,
        "merged": 0,
        "applied": 0,
        "abandoned": 0,
        "rejected": 0,
    }

    modified_mps = 0
    unexpected = 0
    check_only = False
    was_forge_ratelimited = False

    for forge, mp, status in iter_all_mps():
        status_count[status] += 1
        if forge in forge_rate_limiter:
            if datetime.utcnow() < forge_rate_limiter[forge]:
                del forge_rate_limiter[forge]
            else:
                forge_rate_limited_count.inc(forge=str(forge))
                was_forge_ratelimited = True
                continue
        try:
            modified = await check_existing_mp(
                conn,
                redis,
                config,
                template_env_path,
                mp,
                status,
                vcs_managers=vcs_managers,
                dry_run=dry_run,
                external_url=external_url,
                differ_url=differ_url,
                maintainer_rate_limiter=maintainer_rate_limiter,
                possible_transports=possible_transports,
                mps_per_maintainer=mps_per_maintainer,
                check_only=check_only,
            )
        except NoRunForMergeProposal as e:
            logger.warning("Unable to find metadata for %s, skipping.", e.mp.url)
            modified = False
        except ForgeLoginRequired as e:
            logger.warning('Login required for forge %s, skipping.', e)
            modified = False
        except BranchRateLimited as e:
            logger.warning(
                "Rate-limited accessing %s. Skipping %r for this cycle.",
                mp.url, forge)
            forge_rate_limiter[forge] = datetime.utcnow() + timedelta(seconds=e.retry_after)
            continue
        except UnexpectedHttpStatus as e:
            logging.warning(
                'Got unexpected HTTP status %s, skipping %r',
                e, mp.url)
            # TODO(jelmer): print traceback?
            unexpected += 1

        if unexpected > unexpected_limit:
            logging.warning(
                "Saw %d unexpected HTTP responses, over threshold of %d. "
                "Giving up for now.", unexpected, unexpected_limit)
            return

        if modified:
            modified_mps += 1
            if modify_limit and modified_mps > modify_limit:
                logger.warning(
                    "Already modified %d merge proposals, "
                    "waiting with the rest.", modified_mps,
                )
                check_only = True

    logging.info('Successfully scanned existing merge proposals')
    last_scan_existing_success.set_to_current_time()

    if not was_forge_ratelimited:
        for status, count in status_count.items():
            merge_proposal_count.labels(status=status).set(count)

        maintainer_rate_limiter.set_mps_per_maintainer(mps_per_maintainer)
        for maintainer_email, count in mps_per_maintainer["open"].items():
            open_proposal_count.labels(maintainer=maintainer_email).set(count)
    else:
        logging.info('Rate-Limited for forges %r. Not updating stats', forge_rate_limiter)


async def get_run(conn: asyncpg.Connection, run_id):
    query = """
SELECT
    id, command, start_time, finish_time, description, package,
    result_code,
    value, main_branch_revision, revision, context, result, suite,
    instigated_context, vcs_type, branch_url, logfilenames, review_status,
    review_comment, worker,
    array(SELECT row(role, remote_name, base_revision,
     revision) FROM new_result_branch WHERE run_id = id) AS result_branches,
    result_tags, target_branch_url, change_set
FROM
    run
WHERE id = $1
"""
    row = await conn.fetch(query, run_id)
    if row:
        return state.Run.from_row(row)
    return None


async def iter_control_matching_runs(conn: asyncpg.Connection, main_branch_revision: bytes, package: str):
    query = """
SELECT
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
FROM last_runs
WHERE main_branch_revision = $1 AND package = $2 AND main_branch_revision != revision AND suite NOT in ('unchanged', 'control')
ORDER BY start_time DESC
"""
    return await conn.fetch(
        query, main_branch_revision.decode('utf-8'), package)



async def listen_to_runner(
    db,
    redis,
    config,
    template_env_path,
    maintainer_rate_limiter,
    vcs_managers,
    dry_run: bool,
    external_url: str,
    differ_url: str,
    require_binary_diff: bool = False,
):
    async def process_run(conn, run, maintainer_email, branch_url):
        publish_policy, command = await get_publish_policy(
            conn, run.package, run.suite
        )
        for role, (mode, max_frequency_days) in publish_policy.items():
            await publish_from_policy(
                conn,
                get_campaign_config(config, run.suite),
                template_env_path,
                maintainer_rate_limiter,
                vcs_managers,
                run,
                role,
                maintainer_email,
                branch_url,
                redis,
                mode,
                max_frequency_days,
                command,
                dry_run=dry_run,
                external_url=external_url,
                differ_url=differ_url,
                require_binary_diff=require_binary_diff,
                force=True,
                requestor="runner",
            )

    channel = (await redis.subscribe('result'))[0]
    while (await channel.wait_message()):
        result = channel.get_json()
        if result["code"] != "success":
            continue
        async with db.acquire() as conn:
            # TODO(jelmer): Fold these into a single query ?
            package = await conn.fetchrow(
                'SELECT maintainer_email, branch_url FROM package WHERE name = $1',
                result["package"])
            if package is None:
                logging.warning('Package %s not in database?', result['package'])
                continue
            run = await get_run(conn, result["log_id"])
            if run.suite != "unchanged":
                await process_run(
                    conn, run, package['maintainer_email'],
                    package['branch_url'])
            else:
                for run in await iter_control_matching_runs(
                        conn, main_branch_revision=run.revision,
                        package=run.package):
                    await process_run(
                        conn, run, package['maintainer_email'],
                        package['branch_url'])


async def main(argv=None):
    import argparse

    parser = argparse.ArgumentParser(prog="janitor.publish")
    parser.add_argument(
        "--max-mps-per-maintainer",
        default=0,
        type=int,
        help="Maximum number of open merge proposals per maintainer.",
    )
    parser.add_argument(
        "--dry-run",
        help="Create branches but don't push or propose anything.",
        action="store_true",
        default=False,
    )
    parser.add_argument(
        "--prometheus", type=str, help="Prometheus push gateway to export to."
    )
    parser.add_argument(
        "--once",
        action="store_true",
        help="Just do one pass over the queue, don't run as a daemon.",
    )
    parser.add_argument(
        "--listen-address", type=str, help="Listen address", default="localhost"
    )
    parser.add_argument("--port", type=int, help="Listen port", default=9912)
    parser.add_argument(
        "--interval",
        type=int,
        help=("Seconds to wait in between publishing " "pending proposals"),
        default=7200,
    )
    parser.add_argument(
        "--no-auto-publish",
        action="store_true",
        help="Do not create merge proposals automatically.",
    )
    parser.add_argument(
        "--config",
        type=str,
        default="janitor.conf",
        help="Path to load configuration from.",
    )
    parser.add_argument(
        "--slowstart", action="store_true", help="Use slow start rate limiter."
    )
    parser.add_argument(
        "--reviewed-only",
        action="store_true",
        help="Only publish changes that were reviewed.",
    )
    parser.add_argument(
        "--push-limit", type=int, help="Limit number of pushes per cycle."
    )
    parser.add_argument(
        "--require-binary-diff",
        action="store_true",
        default=False,
        help="Require a binary diff when publishing merge requests.",
    )
    parser.add_argument(
        "--modify-mp-limit",
        type=int,
        default=10,
        help="Maximum number of merge proposals to update per cycle.",
    )
    parser.add_argument(
        "--external-url",
        type=str,
        help="External URL",
        default=None)
    parser.add_argument(
        "--differ-url", type=str, help="Differ URL.", default="http://localhost:9920/"
    )
    parser.add_argument("--gcp-logging", action='store_true', help='Use Google cloud logging.')
    parser.add_argument(
        "--template-env-path", type=str,
        help="Path to merge proposal templates")

    args = parser.parse_args()

    if args.gcp_logging:
        import google.cloud.logging
        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    else:
        logging.basicConfig(level=logging.INFO)

    with open(args.config, "r") as f:
        config = read_config(f)

    if args.slowstart:
        maintainer_rate_limiter = SlowStartRateLimiter(args.max_mps_per_maintainer)
    elif args.max_mps_per_maintainer > 0:
        maintainer_rate_limiter = FixedRateLimiter(args.max_mps_per_maintainer)
    else:
        maintainer_rate_limiter = NonRateLimiter()

    if args.no_auto_publish and args.once:
        sys.stderr.write("--no-auto-publish and --once are mutually exclude.")
        sys.exit(1)

    forge_rate_limiter: Dict[Forge, datetime] = {}

    loop = asyncio.get_event_loop()
    vcs_managers = get_vcs_managers_from_config(config)
    db = await state.create_pool(config.database_location)
    async with AsyncExitStack() as stack:
        redis = await aioredis.create_redis(config.redis_location)
        stack.callback(redis.close)

        if args.once:
            await publish_pending_ready(
                db,
                redis,
                config,
                args.template_env_path,
                maintainer_rate_limiter,
                dry_run=args.dry_run,
                external_url=args.external_url,
                differ_url=args.differ_url,
                vcs_managers=vcs_managers,
                reviewed_only=args.reviewed_only,
                require_binary_diff=args.require_binary_diff,
            )
            if args.prometheus:
                await push_to_gateway(
                    args.prometheus, job="janitor.publish", registry=REGISTRY)
        else:
            tasks = [
                loop.create_task(
                    process_queue_loop(
                        db,
                        redis,
                        config,
                        args.template_env_path,
                        maintainer_rate_limiter,
                        forge_rate_limiter,
                        dry_run=args.dry_run,
                        vcs_managers=vcs_managers,
                        interval=args.interval,
                        auto_publish=not args.no_auto_publish,
                        external_url=args.external_url,
                        differ_url=args.differ_url,
                        reviewed_only=args.reviewed_only,
                        push_limit=args.push_limit,
                        modify_mp_limit=args.modify_mp_limit,
                        require_binary_diff=args.require_binary_diff,
                    )
                ),
                loop.create_task(
                    run_web_server(
                        args.listen_address,
                        args.port,
                        args.template_env_path,
                        maintainer_rate_limiter,
                        forge_rate_limiter,
                        vcs_managers,
                        db, redis, config,
                        dry_run=args.dry_run,
                        external_url=args.external_url,
                        differ_url=args.differ_url,
                        require_binary_diff=args.require_binary_diff,
                        modify_mp_limit=args.modify_mp_limit,
                        push_limit=args.push_limit,
                    )
                ),
            ]
            if not args.reviewed_only and not args.no_auto_publish:
                tasks.append(
                    loop.create_task(
                        listen_to_runner(
                            db,
                            redis,
                            config,
                            args.template_env_path,
                            maintainer_rate_limiter,
                            vcs_managers,
                            dry_run=args.dry_run,
                            external_url=args.external_url,
                            differ_url=args.differ_url,
                            require_binary_diff=args.require_binary_diff,
                        )
                    )
                )
            await asyncio.gather(*tasks)


if __name__ == "__main__":
    sys.exit(asyncio.run(main(sys.argv)))
