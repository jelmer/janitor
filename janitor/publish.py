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
import warnings
from yarl import URL


import aioredlock
import aiozipkin
from aiohttp.web_middlewares import normalize_path_middleware
from aiohttp import web, ClientSession
import asyncpg
import asyncpg.pool

from aiohttp_apispec import (
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

from breezy import urlutils
import gpg
from redis.asyncio import Redis

from breezy.errors import PermissionDenied, UnexpectedHttpStatus
from breezy.forge import (
    Forge,
    forges,
    ForgeLoginRequired,
    get_forge_by_hostname,
    get_proposal_by_url,
    UnsupportedForge,
    MergeProposal,
    iter_forge_instances,
)
from breezy.transport import Transport
import breezy.plugins.gitlab  # noqa: F401
import breezy.plugins.launchpad  # noqa: F401
import breezy.plugins.github  # noqa: F401

from silver_platter.utils import (
    open_branch,
    BranchMissing,
    BranchUnavailable,
    BranchRateLimited,
)



from . import (
    set_user_agent,
    state,
)
from .config import read_config, get_campaign_config, Campaign, Config
from .schedule import (
    do_schedule,
    do_schedule_control,
    CandidateUnavailable,
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
    ["codebase", "campaign"],
)
open_proposal_count = Gauge("open_proposal_count", "Number of open proposals.")
bucket_proposal_count = Gauge(
    "bucket_proposal_count", "Number of proposals per bucket.", labelnames=("bucket",)
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

rejected_last_mp_count = Counter(
    "rejected_last_mp",
    "Last merge proposal was rejected")

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

unexpected_http_response_count = Counter(
    "unexpected_http_response_count",
    "Number of unexpected HTTP responses during checks of existing "
    "proposals")


logger = logging.getLogger('janitor.publish')


routes = web.RouteTableDef()


def get_merged_by_user_url(url, user):
    hostname = URL(url).host
    if hostname is None:
        return None
    try:
        forge = get_forge_by_hostname(hostname)
    except UnsupportedForge:
        return None
    return forge.get_user_url(user)


class RateLimited(Exception):
    """A rate limit was reached."""


class BucketRateLimited(RateLimited):
    """Per-bucket rate-limit was reached."""

    def __init__(self, bucket, open_mps, max_open_mps):
        super(BucketRateLimited, self).__init__(
            "Bucke %s already has %d merge proposal open (max: %d)" % (
                bucket, open_mps, max_open_mps))
        self.bucket = bucket
        self.open_mps = open_mps
        self.max_open_mps = max_open_mps


class RateLimiter(object):
    def set_mps_per_bucket(
        self, mps_per_bucket: Dict[str, Dict[str, int]]
    ) -> None:
        raise NotImplementedError(self.set_mps_per_bucket)

    def check_allowed(self, bucket: str) -> None:
        raise NotImplementedError(self.check_allowed)

    def inc(self, bucket: str) -> None:
        raise NotImplementedError(self.inc)

    def get_stats(self) -> Dict[str, Tuple[int, Optional[int]]]:
        raise NotImplementedError(self.get_stats)


class FixedRateLimiter(RateLimiter):

    _open_mps_per_bucket: Optional[Dict[str, int]]

    def __init__(self, max_mps_per_bucket: Optional[int] = None):
        self._max_mps_per_bucket = max_mps_per_bucket
        self._open_mps_per_bucket = None

    def set_mps_per_bucket(self, mps_per_bucket: Dict[str, Dict[str, int]]):
        self._open_mps_per_bucket = mps_per_bucket["open"]

    def check_allowed(self, bucket: str):
        if not self._max_mps_per_bucket:
            return
        if self._open_mps_per_bucket is None:
            # Be conservative
            raise RateLimited("Open mps per bucket not yet determined.")
        current = self._open_mps_per_bucket.get(bucket, 0)
        if current > self._max_mps_per_bucket:
            raise BucketRateLimited(bucket, current, self._max_mps_per_bucket)

    def inc(self, bucket: str):
        if self._open_mps_per_bucket is None:
            return
        self._open_mps_per_bucket.setdefault(bucket, 0)
        self._open_mps_per_bucket[bucket] += 1

    def get_stats(self) -> Dict[str, Tuple[int, Optional[int]]]:
        if self._open_mps_per_bucket:
            return {
                bucket: (current, self._max_mps_per_bucket)
                for (bucket, current) in self._open_mps_per_bucket.items()}
        else:
            return {}


class NonRateLimiter(RateLimiter):
    def check_allowed(self, bucket):
        pass

    def inc(self, bucket):
        pass

    def set_mps_per_bucket(self, mps_per_bucket):
        pass

    def get_stats(self):
        return {}


class SlowStartRateLimiter(RateLimiter):
    def __init__(self, max_mps_per_bucket=None):
        self._max_mps_per_bucket = max_mps_per_bucket
        self._open_mps_per_bucket: Optional[Dict[str, int]] = None
        self._absorbed_mps_per_bucket: Optional[Dict[str, int]] = None

    def check_allowed(self, bucket: str) -> None:
        if (
            self._open_mps_per_bucket is None
            or self._absorbed_mps_per_bucket is None
        ):
            # Be conservative
            raise RateLimited("Open mps per bucket not yet determined.")
        current = self._open_mps_per_bucket.get(bucket, 0)
        if self._max_mps_per_bucket and current >= self._max_mps_per_bucket:
            raise BucketRateLimited(bucket, current, self._max_mps_per_bucket)
        limit = self._get_limit(bucket)
        if limit is not None and current >= limit:
            raise BucketRateLimited(bucket, current, limit)

    def _get_limit(self, bucket) -> Optional[int]:
        if self._absorbed_mps_per_bucket is None:
            return None
        return self._absorbed_mps_per_bucket.get(bucket, 0) + 1

    def inc(self, bucket: str):
        if self._open_mps_per_bucket is None:
            return
        self._open_mps_per_bucket.setdefault(bucket, 0)
        self._open_mps_per_bucket[bucket] += 1

    def set_mps_per_bucket(self, mps_per_bucket: Dict[str, Dict[str, int]]):
        self._open_mps_per_bucket = mps_per_bucket.get("open", {})
        ms: Dict[str, int] = {}
        for status in ['merged', 'applied']:
            for m, c in mps_per_bucket.get(status, {}).items():
                ms.setdefault(m, 0)
                ms[m] += c
        self._absorbed_mps_per_bucket = ms

    def get_stats(self):
        if self._open_mps_per_bucket is None:
            return {}
        else:
            return {
                bucket: (
                    current,
                    min(self._max_mps_per_bucket, self._get_limit(bucket)))
                for bucket, current in self._open_mps_per_bucket.items()}


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

    if await state.has_cotenants(conn, run.codebase, run.branch_url):
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
        return open_branch(url_a).name == open_branch(url_b).name  # type: ignore
    except BranchMissing:
        return False


@dataclass
class PublishResult:

    description: str
    is_new: bool = False
    proposal_url: Optional[str] = None
    proposal_web_url: Optional[str] = None
    branch_name: Optional[str] = None


class BranchBusy(Exception):
    """The branch is already busy."""

    def __init__(self, branch_url):
        self.branch_url = branch_url


class WorkerInvalidResponse(Exception):
    """Invalid response from worker."""

    def __init__(self, output):
        self.output = output


async def run_worker_process(args, request, *, encoding='utf-8'):
    p = await asyncio.create_subprocess_exec(
        *args,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        stdin=asyncio.subprocess.PIPE
    )

    (stdout, stderr) = await p.communicate(
        json.dumps(request).encode(encoding))

    if p.returncode == 1:
        try:
            response = json.loads(stdout.decode(encoding))
        except json.JSONDecodeError as e:
            raise WorkerInvalidResponse(stderr.decode(encoding)) from e
        sys.stderr.write(stderr.decode(encoding))
        return 1, response

    if p.returncode == 0:
        return p.returncode, json.loads(stdout.decode(encoding))

    raise WorkerInvalidResponse(stderr.decode(encoding))


class PublishWorker(object):

    def __init__(self, *,
                 lock_manager=None,
                 redis=None,
                 template_env_path: Optional[str] = None,
                 external_url: Optional[str] = None,
                 differ_url: Optional[str] = None):
        self.template_env_path = template_env_path
        self.external_url = external_url
        self.differ_url = differ_url
        self.lock_manager = lock_manager
        self.redis = redis

    async def publish_one(
        self,
        *,
        campaign: str,
        pkg: str,
        command,
        main_branch_url: str,
        mode: str,
        role: str,
        revision: bytes,
        log_id: str,
        unchanged_id: str,
        derived_branch_name: str,
        rate_limit_bucket: Optional[str],
        vcs_manager: VcsManager,
        bucket_rate_limiter: Optional[RateLimiter] = None,
        dry_run: bool = False,
        require_binary_diff: bool = False,
        allow_create_proposal: bool = False,
        reviewers: Optional[List[str]] = None,
        result_tags: Optional[List[Tuple[str, bytes]]] = None,
        commit_message_template: Optional[str] = None,
        title_template: Optional[str] = None,
        codemod_result=None,
        existing_mp_url: Optional[str] = None,
    ) -> PublishResult:
        """Publish a single run in some form.

        Args:
          campaign: The campaign name
          pkg: Package name
          command: Command that was run
        """
        assert mode in SUPPORTED_MODES, "mode is %r" % (mode, )
        local_branch_url = vcs_manager.get_branch_url(pkg, "%s/%s" % (campaign, role))
        target_branch_url = main_branch_url.rstrip("/")

        request = {
            "dry-run": dry_run,
            "campaign": campaign,
            "package": pkg,
            "command": command,
            "codemod_result": codemod_result,
            "target_branch_url": target_branch_url,
            "source_branch_url": local_branch_url,
            "existing_mp_url": existing_mp_url,
            "derived_branch_name": derived_branch_name,
            "mode": mode,
            "role": role,
            "log_id": log_id,
            "unchanged_id": unchanged_id,
            "require-binary-diff": require_binary_diff,
            "allow_create_proposal": allow_create_proposal,
            "external_url": self.external_url,
            "differ_url": self.differ_url,
            "revision": revision.decode("utf-8"),
            "reviewers": reviewers,
            "commit_message_template": commit_message_template,
            "title_template": title_template,
        }

        if result_tags:
            request["tags"] = {n: r.decode("utf-8") for (n, r) in result_tags}
        else:
            request["tags"] = {}

        args = [sys.executable, "-m", "janitor.publish_one"]

        if self.template_env_path:
            args.append('--template-env-path=%s' % self.template_env_path)

        try:
            async with AsyncExitStack() as es:
                if self.lock_manager:
                    await es.enter_async_context(
                        await self.lock_manager.lock("publish:%s" % target_branch_url))
                try:
                    returncode, response = await run_worker_process(args, request)
                except WorkerInvalidResponse as e:
                    raise PublishFailure(
                        mode, "publisher-invalid-response", e.output) from e
        except aioredlock.LockError as e:
            raise BranchBusy(target_branch_url) from e

        if returncode == 1:
            raise PublishFailure(mode, response["code"], response["description"])

        if returncode == 0:
            proposal_url = response.get("proposal_url")
            proposal_web_url = response.get("proposal_web_url")
            branch_name = response.get("branch_name")
            is_new = response.get("is_new")
            description = response.get('description')

            if proposal_url and is_new:
                if self.redis:
                    await self.redis.publish(
                        'merge-proposal',
                        json.dumps({
                            "url": proposal_url, "web_url": proposal_web_url,
                            "status": "open", "package": pkg,
                            "campaign": campaign,
                            "target_branch_url": main_branch_url.rstrip("/")}))

                merge_proposal_count.labels(status="open").inc()
                open_proposal_count.inc()
                if rate_limit_bucket:
                    if bucket_rate_limiter:
                        bucket_rate_limiter.inc(rate_limit_bucket)
                    bucket_proposal_count.labels(bucket=rate_limit_bucket).inc()

            return PublishResult(
                proposal_url=proposal_url,
                proposal_web_url=proposal_web_url,
                branch_name=branch_name, is_new=is_new,
                description=description)

        raise AssertionError


def calculate_next_try_time(finish_time: datetime, attempt_count: int) -> datetime:
    if attempt_count == 0:
        return finish_time
    try:
        return finish_time + (2 ** attempt_count * timedelta(hours=1))
    except OverflowError:
        return finish_time + timedelta(hours=(7 * 24))


async def consider_publish_run(
        conn: asyncpg.Connection, redis, *, config: Config, publish_worker: PublishWorker,
        vcs_managers, bucket_rate_limiter,
        run, rate_limit_bucket,
        unpublished_branches, command,
        push_limit=None, require_binary_diff=False,
        dry_run=False):
    if run.revision is None:
        logger.warning(
            "Run %s is publish ready, but does not have revision set.", run.id
        )
        return {}
    campaign_config = get_campaign_config(config, run.campaign)
    attempt_count = await get_publish_attempt_count(
        conn, run.revision, {"differ-unreachable"})
    next_try_time = calculate_next_try_time(run.finish_time, attempt_count)
    if datetime.utcnow() < next_try_time:
        logger.info(
            "Not attempting to push %s / %s (%s) due to "
            "exponential backoff. Next try in %s.",
            run.package,
            run.campaign,
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
                run.campaign,
            )
            push_limit_count.inc()
            return {}
    if run.branch_url is None:
        logger.warning(
            '%s: considering publishing for branch without branch url',
            run.id)
        missing_branch_url_count.inc()
        # TODO(jelmer): Support target_branch_url ?
        return {}

    last_mps = await get_previous_mp_status(conn, run.codebase, run.campaign)
    if any(last_mp[1] not in ('rejected', 'closed')
           for last_mp in last_mps):
        logger.warning(
            '%s: last merge proposal was rejected by maintainer: %r', run.id,
            last_mps)
        rejected_last_mp_count.inc()
        return {}

    actual_modes: Dict[str, Optional[str]] = {}
    for (
        role,
        _remote_name,
        _base_revision,
        _revision,
        publish_mode,
        max_frequency_days
    ) in unpublished_branches:
        if publish_mode is None:
            logger.warning(
                "%s: No publish mode for branch with role %s", run.id, role)
            missing_publish_mode_count.labels(role=role).inc()
            continue
        if role == 'main' and None in actual_modes.values():
            logger.warning(
                "%s: Skipping branch with role %s, as not all "
                "auxiliary branches were published.", run.id, role)
            unpublished_aux_branches_count.labels(role=role).inc()
            continue
        actual_modes[role] = await publish_from_policy(
            conn=conn, campaign_config=campaign_config,
            publish_worker=publish_worker,
            bucket_rate_limiter=bucket_rate_limiter,
            vcs_managers=vcs_managers, run=run, role=role,
            rate_limit_bucket=rate_limit_bucket, main_branch_url=run.branch_url,
            mode=publish_mode,
            max_frequency_days=max_frequency_days, command=command,
            dry_run=dry_run,
            redis=redis,
            require_binary_diff=require_binary_diff,
            force=False,
            requestor="publisher (publish pending)",
        )

    return actual_modes


async def iter_publish_ready(
    conn: asyncpg.Connection,
    *,
    campaigns: Optional[List[str]] = None,
    review_status: Optional[List[str]] = None,
    limit: Optional[int] = None,
    needs_review: Optional[bool] = None,
    run_id: Optional[str] = None,
    change_set_state: Optional[List[str]] = None,
) -> AsyncIterable[
    Tuple[
        state.Run,
        str,
        str,
        List[Tuple[str, Optional[str], str, bytes, bytes, Optional[str],
                   Optional[int], Optional[str]]],
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
    if change_set_state is not None:
        args.append(change_set_state)
        conditions.append("change_set_state = ANY($%d::change_set_state[])" % (len(args),))

    publishable_condition = (
        "exists (select from unnest(unpublished_branches) where "
        "mode in ('propose', 'attempt-push', 'push-derived', 'push'))"
    )

    conditions.append(publishable_condition)

    if needs_review is not None:
        args.append(needs_review)
        conditions.append('needs_review = $%d' % (len(args)))

    if conditions:
        query += " WHERE " + " AND ".join(conditions)

    order_by = ["change_set_state = 'publishing' DESC", "change_set"]

    order_by.extend(["value DESC NULLS LAST", "finish_time DESC"])

    if order_by:
        query += " ORDER BY " + ", ".join(order_by) + " "

    if limit is not None:
        query += " LIMIT %d" % limit
    for record in await conn.fetch(query, *args):
        yield tuple(  # type: ignore
            [state.Run.from_row(record),
             record['rate_limit_bucket'],
             record['policy_command'],
             record['unpublished_branches']
             ]
        )


async def publish_pending_ready(
    *,
    db,
    redis,
    config,
    publish_worker,
    bucket_rate_limiter,
    vcs_managers,
    dry_run: bool,
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
            rate_limit_bucket,
            command,
            unpublished_branches,
        ) in iter_publish_ready(
            conn1, review_status=review_status,
            needs_review=False,
            change_set_state=['ready', 'publishing'],
        ):
            actual_modes = await consider_publish_run(
                conn, redis=redis, config=config,
                publish_worker=publish_worker,
                vcs_managers=vcs_managers,
                bucket_rate_limiter=bucket_rate_limiter,
                run=run,
                command=command,
                rate_limit_bucket=rate_limit_bucket,
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
    unchanged_run = await conn.fetchrow(
        "SELECT result_code, package, revision FROM last_runs "
        "WHERE revision = $2 AND package = $1 and result_code = 'success'",
        run.package, run.main_branch_revision.decode('utf-8')
    )

    code = e.code
    description = e.description
    if e.code == "merge-conflict":
        logger.info("Merge proposal would cause conflict; restarting.")
        await do_schedule(
            conn,
            package=run.package,
            campaign=run.campaign,
            change_set=run.change_set,
            codebase=run.codebase,
            requestor="publisher (pre-creation merge conflict)",
            bucket=bucket,
        )
    elif e.code == "diverged-branches":
        logger.info("Branches have diverged; restarting.")
        await do_schedule(
            conn,
            package=run.package,
            campaign=run.campaign,
            change_set=run.change_set,
            requestor="publisher (diverged branches)",
            bucket=bucket,
            codebase=run.codebase,
        )
    elif e.code == "missing-build-diff-self":
        if run.result_code != "success":
            description = "Missing build diff; run was not actually successful?"
        else:
            description = "Missing build artifacts, rescheduling"
            await do_schedule(
                conn,
                package=run.package,
                campaign=run.campaign,
                change_set=run.change_set,
                refresh=True,
                requestor="publisher (missing build artifacts - self)",
                bucket=bucket, codebase=run.codebase,
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
                bucket=bucket, codebase=run.codebase,
            )
        else:
            description = "Missing binary diff; requesting control run."
            if run.main_branch_revision is not None:
                await do_schedule_control(
                    conn,
                    package=run.package,
                    main_branch_revision=run.main_branch_revision,
                    requestor="publisher (missing control run for diff)",
                    bucket=bucket, codebase=run.codebase,
                )
            else:
                logger.warning(
                    "Successful run (%s) does not have main branch revision set",
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
    change_set_id: str,
    package: str,
    branch_name: Optional[str],
    main_branch_revision: Optional[bytes],
    revision: Optional[bytes],
    role: str,
    mode: str,
    result_code,
    description,
    merge_proposal_url=None,
    target_branch_url=None,
    publish_id=None,
    requestor=None,
    run_id=None,
):
    if isinstance(revision, bytes):
        revision = revision.decode("utf-8")  # type: ignore
    if isinstance(main_branch_revision, bytes):
        main_branch_revision = main_branch_revision.decode("utf-8")  # type: ignore
    async with conn.transaction():
        if merge_proposal_url:
            await conn.execute(
                "INSERT INTO merge_proposal "
                "(url, package, status, revision, last_scanned, "
                " target_branch_url) "
                "VALUES ($1, $2, 'open', $3, NOW(), $4) ON CONFLICT (url) "
                "DO UPDATE SET package = EXCLUDED.package, "
                "revision = EXCLUDED.revision, "
                "last_scanned = EXCLUDED.last_scanned, "
                "target_branch_url = EXCLUDED.target_branch_url",
                merge_proposal_url,
                package,
                revision,
                target_branch_url
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
            "description, merge_proposal_url, id, requestor, change_set, run_id) "
            "values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) ",
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
            change_set_id,
            run_id,
        )
        if result_code == 'success':
            await conn.execute(
                "UPDATE change_set SET state = 'publishing' WHERE state = 'ready' AND id = $1",
                change_set_id)
            # TODO(jelmer): if there is nothing left to publish, then mark this
            # change_set as done


async def publish_from_policy(
    *,
    conn: asyncpg.Connection,
    redis,
    campaign_config: Campaign,
    publish_worker: PublishWorker,
    bucket_rate_limiter: RateLimiter,
    vcs_managers: Dict[str, VcsManager],
    run: state.Run,
    role: str,
    rate_limit_bucket: Optional[str],
    main_branch_url: str,
    mode: str,
    max_frequency_days: Optional[int],
    command: str,
    dry_run: bool,
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
            run.campaign,
            run.command,
            command,
        )
        await do_schedule(
            conn,
            package=run.package,
            campaign=run.campaign,
            change_set=run.change_set,
            command=command,
            bucket="update-new-mp",
            refresh=True,
            requestor="publisher (changed policy: %r => %r)" % (
                run.command, command),
            codebase=run.codebase,
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
                if rate_limit_bucket:
                    bucket_rate_limiter.check_allowed(rate_limit_bucket)
            except RateLimited as e:
                proposal_rate_limited_count.labels(
                    codebase=run.codebase, campaign=run.campaign
                ).inc()
                logger.debug(
                    "Not creating proposal for %s/%s: %s", run.package, run.campaign, e
                )
                mode = MODE_BUILD_ONLY
            if max_frequency_days is not None:
                last_published = await check_last_published(
                    conn, run.campaign, run.package)
                if (last_published is not None
                        and (datetime.utcnow() - last_published).days < max_frequency_days):
                    logger.debug(
                        'Not creating proposal for %s/%s: '
                        'was published already in last %d days (at %s)',
                        run.package, run.campaign, max_frequency_days, last_published)
                    mode = MODE_BUILD_ONLY
    if mode in (MODE_BUILD_ONLY, MODE_SKIP):
        return

    if base_revision is None:
        unchanged_run = None
    else:
        unchanged_run = await conn.fetchrow(
            "SELECT id, result_code FROM last_runs "
            "WHERE package = $1 AND revision = $2 AND result_code = 'success'",
            run.package, base_revision.decode('utf-8'))

    # TODO(jelmer): Make this more generic
    if (
        unchanged_run
        and unchanged_run['result_code'] in (
            "debian-upstream-metadata-invalid", )
        and run.campaign == "lintian-fixes"
    ):
        require_binary_diff = False

    logger.info(
        "Publishing %s / %r / %s (mode: %s)", run.package, run.command, role, mode
    )
    try:
        publish_result = await publish_worker.publish_one(
            campaign=run.campaign,
            pkg=run.package,
            command=run.command,
            codemod_result=run.result,
            main_branch_url=main_branch_url,
            mode=mode,
            role=role,
            revision=revision,
            log_id=run.id,
            unchanged_id=(unchanged_run['id'] if unchanged_run else None),
            derived_branch_name=await derived_branch_name(conn, campaign_config, run, role),
            rate_limit_bucket=rate_limit_bucket,
            vcs_manager=vcs_managers[run.vcs_type],
            dry_run=dry_run,
            require_binary_diff=require_binary_diff,
            bucket_rate_limiter=bucket_rate_limiter,
            result_tags=run.result_tags,
            allow_create_proposal=run_sufficient_for_proposal(campaign_config, run.value),
            commit_message_template=(
                campaign_config.merge_proposal.commit_message
                if campaign_config.merge_proposal else None),
            title_template=(
                campaign_config.merge_proposal.title
                if campaign_config.merge_proposal else None),
        )
    except BranchBusy as e:
        logger.info('Branch %r was busy', e.branch_url)
        return
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
        run.change_set,
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
        target_branch_url=main_branch_url,
        requestor=requestor,
        run_id=run.id,
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
        "campaign": run.campaign,
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

    await pubsub_publish(redis, topic_entry)

    if code == "success":
        return mode


async def pubsub_publish(redis, topic_entry):
    await redis.publish('publish', json.dumps(topic_entry))


def role_branch_url(url: str, remote_branch_name: Optional[str]) -> str:
    if remote_branch_name is None:
        return url
    base_url, params = urlutils.split_segment_parameters(url.rstrip("/"))
    params["branch"] = urlutils.escape(remote_branch_name, safe="")
    return urlutils.join_segment_parameters(base_url, params)


def run_sufficient_for_proposal(campaign_config: Campaign, run_value: Optional[int]) -> bool:
    if (run_value is not None and campaign_config.merge_proposal is not None
            and campaign_config.merge_proposal.value_threshold):
        return (run_value >= campaign_config.merge_proposal.value_threshold)
    else:
        # Assume yes, if the run doesn't have an associated value or if there
        # is no threshold configured.
        return True


async def publish_and_store(
    *,
    db: asyncpg.Connection,
    redis,
    campaign_config: Campaign,
    publish_worker: PublishWorker,
    publish_id: str,
    run: state.Run,
    mode: str,
    role: str,
    rate_limit_bucket: Optional[str],
    vcs_managers: Dict[str, VcsManager],
    bucket_rate_limiter: RateLimiter,
    dry_run: bool,
    allow_create_proposal: bool = True,
    require_binary_diff: bool = False,
    requestor: Optional[str] = None,
):
    remote_branch_name, base_revision, revision = run.get_result_branch(role)

    main_branch_url = role_branch_url(run.branch_url, remote_branch_name)

    if allow_create_proposal is None:
        allow_create_proposal = run_sufficient_for_proposal(
            campaign_config, run.value)

    async with db.acquire() as conn:
        if run.main_branch_revision:
            unchanged_run_id = await conn.fetchval(
                "SELECT id FROM run "
                "WHERE revision = $2 AND package = $1 and result_code = 'success' "
                "ORDER BY finish_time DESC LIMIT 1",
                run.package, run.main_branch_revision.decode('utf-8')
            )
        else:
            unchanged_run_id = None

        try:
            publish_result = await publish_worker.publish_one(
                campaign=run.campaign,
                pkg=run.package,
                command=run.command,
                codemod_result=run.result,
                main_branch_url=main_branch_url,
                mode=mode,
                role=role,
                revision=revision,
                log_id=run.id,
                unchanged_id=unchanged_run_id,
                derived_branch_name=await derived_branch_name(conn, campaign_config, run, role),
                rate_limit_bucket=rate_limit_bucket,
                vcs_manager=vcs_managers[run.vcs_type],
                dry_run=dry_run,
                require_binary_diff=require_binary_diff,
                allow_create_proposal=allow_create_proposal,
                bucket_rate_limiter=bucket_rate_limiter,
                result_tags=run.result_tags,
                commit_message_template=(
                    campaign_config.merge_proposal.commit_message
                    if campaign_config.merge_proposal else None),
                title_template=(
                    campaign_config.merge_proposal.title
                    if campaign_config.merge_proposal else None),
            )
        except BranchBusy as e:
            logging.debug("Branch %r was busy while publishing",
                          e.branch_url)
            return
        except PublishFailure as e:
            await store_publish(
                conn,
                run.change_set,
                run.package,
                campaign_config.branch_name,
                run.main_branch_revision,
                run.revision,
                role,
                e.mode,
                e.code,
                description=e.description,
                publish_id=publish_id,
                requestor=requestor,
                run_id=run.id,
            )
            publish_entry = {
                "id": publish_id,
                "mode": e.mode,
                "result_code": e.code,
                "description": e.description,
                "package": run.package,
                "campaign": run.campaign,
                "main_branch_url": run.branch_url,
                "main_branch_browse_url": bzr_to_browse_url(run.branch_url),
                "result": run.result,
            }

            await pubsub_publish(redis, publish_entry)
            return

        if mode == MODE_ATTEMPT_PUSH:
            if publish_result.proposal_url:
                mode = MODE_PROPOSE
            else:
                mode = MODE_PUSH

        await store_publish(
            conn,
            run.change_set,
            run.package,
            publish_result.branch_name,
            run.main_branch_revision,
            run.revision,
            role,
            mode,
            "success",
            description="Success",
            merge_proposal_url=(
                publish_result.proposal_url
                if publish_result.proposal_url else None),
            target_branch_url=run.branch_url,
            publish_id=publish_id,
            requestor=requestor,
            run_id=run.id,
        )

        publish_delay = datetime.utcnow() - run.finish_time
        publish_latency.observe(publish_delay.total_seconds())

        publish_entry = {
            "id": publish_id,
            "package": run.package,
            "campaign": run.campaign,
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

        await pubsub_publish(redis, publish_entry)


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


@routes.get("/absorbed")
async def handle_absorbed(request):
    try:
        since = datetime.fromisoformat(request.query['since'])
    except KeyError:
        extra = ""
        args = []
    else:
        args = [since]
        extra = " AND absorbed_at >= $%d" % len(args)

    ret = []
    async with request.app['db'].acquire() as conn:
        query = """\
SELECT
   change_set,
   delay,
   campaign,
   result,
   id,
   absorbed_at,
   merged_by,
   merge_proposal_url
FROM absorbed_runs
"""
        for row in await conn.fetch(query + extra, *args):
            ret.append({
                'mode': row['mode'],
                'change_set': row['change_set'],
                'delay': row['delay'].total_seconds,
                'campaign': row['campaign'],
                'merged-by': row['merged_by'],
                'merged-by-url': await asyncio.to_thread(get_merged_by_user_url(
                    row['merge_proposal_url'], row['merged_by'])),
                'absorbed-at': row['absorbed-at'],
                'id': row['id'],
                'result': row['result'],
            })
    return web.json_response(ret)


@routes.get("/policy/{name}", name="get-policy")
async def handle_policy_get(request):
    name = request.match_info["name"]
    async with request.app['db'].acquire() as conn:
        row = await conn.fetchrow(
            "SELECT * "
            "FROM named_publish_policy WHERE name = $1", name)
    if not row:
        return web.json_response({"reason": "Publish policy not found"}, status=404)
    return web.json_response({
        "rate_limit_bucket": row["rate_limit_bucket"],
        "per_branch": {
            p['role']: {
                'mode': p['mode'],
                'max_frequency_days': p['frequency_days'],
            } for p in row['publish']},
        "qa_review": row['qa_review'],
    })


@routes.get("/policy", name="get-full-policy")
async def handle_full_policy_get(request):
    async with request.app['db'].acquire() as conn:
        rows = await conn.fetch("SELECT * FROM named_publish_policy")
    return web.json_response({row['name']: {
        "rate_limit_bucket": row["rate_limit_bucket"],
        "per_branch": {
            p['role']: {
                'mode': p['mode'],
                'max_frequency_days': p['frequency_days'],
            } for p in row['per_branch_policy']},
        "qa_review": row['qa_review'],
    } for row in rows})


@routes.put("/policy/{name}", name="put-policy")
async def handle_policy_put(request):
    name = request.match_info["name"]
    policy = await request.json()
    async with request.app['db'].acquire() as conn:
        await conn.execute(
            "INSERT INTO named_publish_policy "
            "(name, qa_review, per_branch_policy, rate_limit_bucket) "
            "VALUES ($1, $2, $3, $4) ON CONFLICT (name) "
            "DO UPDATE SET qa_review = EXCLUDED.qa_review, "
            "per_branch_policy = EXCLUDED.per_branch_policy, "
            "rate_limit_bucket = EXCLUDED.rate_limit_bucket",
            name, policy['qa_review'],
            [(r, v['mode'], v.get('max_frequency_days'))
             for (r, v) in policy['per_branch'].items()],
            policy.get('rate_limit_bucket'))
    # TODO(jelmer): Call consider_publish_run
    return web.json_response({})


@routes.put("/policy", name="put-full-policy")
async def handle_full_policy_put(request):
    policy = await request.json()
    async with request.app['db'].acquire() as conn, conn.transaction():
        entries = [
            (name, v['qa_review'],
             [(r, b['mode'], b.get('max_frequency_days'))
              for (r, b) in v['per_branch'].items()],
             v.get('rate_limit_bucket'))
            for (name, v) in policy.items()]
        await conn.executemany(
            "INSERT INTO named_publish_policy "
            "(name, qa_review, per_branch_policy, rate_limit_bucket) "
            "VALUES ($1, $2, $3, $4) ON CONFLICT (name) "
            "DO UPDATE SET qa_review = EXCLUDED.qa_review, "
            "per_branch_policy = EXCLUDED.per_branch_policy, "
            "rate_limit_bucket = EXCLUDED.rate_limit_bucket", entries)
        await conn.execute(
            "DELETE FROM named_publish_policy WHERE NOT (name = ANY($1::text[]))",
            policy.keys())
    # TODO(jelmer): Call consider_publish_run
    return web.json_response({})


@routes.delete("/policy/{name}", name="delete-policy")
async def handle_policy_del(request):
    name = request.match_info["name"]
    async with request.app['db'].acquire() as conn:
        try:
            await conn.execute(
                "DELETE FROM named_publish_policy WHERE name = $1",
                name)
        except asyncpg.ForeignKeyViolationError:
            # There's a candidate that still references this
            # publish policy
            return web.json_response({}, status=412)
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
            async for (run, rate_limit_bucket,
                       command, unpublished_branches) in iter_publish_ready(
                    conn, review_status=review_status,
                    needs_review=False, run_id=run_id,
                    change_set_state=['ready', 'publishing']):
                break
            else:
                return
            await consider_publish_run(
                conn, redis=request.app['redis'],
                config=request.app['config'],
                publish_worker=request.app['publish_worker'],
                vcs_managers=request.app['vcs_managers'],
                bucket_rate_limiter=request.app['bucket_rate_limiter'],
                run=run,
                command=command,
                rate_limit_bucket=rate_limit_bucket,
                unpublished_branches=unpublished_branches,
                require_binary_diff=request.app['require_binary_diff'],
                dry_run=request.app['dry_run'])
    create_background_task(
        run(), 'consider publishing %s' % run_id)
    return web.json_response({}, status=200)


async def get_publish_policy(conn: asyncpg.Connection, package: str, campaign: str):
    row = await conn.fetchrow(
        "SELECT per_branch_policy, command, rate_limit_bucket "
        "FROM candidate "
        "LEFT JOIN named_publish_policy "
        "ON named_publish_policy.name = candidate.publish_policy "
        "WHERE package = $1 AND suite = $2",
        package,
        campaign,
    )
    if row:
        return (
            {v['role']: (v['mode'], v['frequency_days'])
             for v in row['per_branch_policy']},
            row['command'], row['rate_limit_bucket'])
    return None, None, None


@routes.get("/publish/{publish_id}", name="publish-details")
async def handle_publish_id(request):
    publish_id = request.match_info["publish_id"]
    async with request.app['db'].acquire() as conn:
        row = await conn.fetchrow("""
SELECT
  package,
  branch_name,
  main_branch_revision,
  revision,
  mode,
  merge_proposal_url,
  result_code,
  description
FROM publish WHERE id = $1
""", publish_id)
        if row:
            raise web.HTTPNotFound(text="no such publish: %s" % publish_id)
    return web.json_response(
        {
            "package": row['package'],
            "branch": row['branch_name'],
            "main_branch_revision": row['main_branch_revision'],
            "revision": row['revision'],
            "mode": row['mode'],
            "merge_proposal_url": row['merge_proposal_url'],
            "result_code": row['result_code'],
            "description": row['description'],
        }
    )


@routes.post("/{campaign}/{package}/publish", name='publish')
async def publish_request(request):
    dry_run = request.app['dry_run']
    vcs_managers = request.app['vcs_managers']
    bucket_rate_limiter = request.app['bucket_rate_limiter']
    package = request.match_info["package"]
    campaign = request.match_info["campaign"]
    role = request.query.get("role")
    post = await request.post()
    mode = post.get("mode")
    async with request.app['db'].acquire() as conn:
        run = await get_last_effective_run(conn, package, campaign)
        if run is None:
            return web.json_response({}, status=400)

        publish_policy, _, rate_limit_bucket = (
            await get_publish_policy(conn, package, campaign))

        logger.info("Handling request to publish %s/%s", package, campaign)

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
                db=request.app['db'],
                redis=request.app['redis'],
                campaign_config=get_campaign_config(request.app['config'], run.campaign),
                publish_worker=request.app['publish_worker'],
                publish_id=publish_id,
                run=run,
                mode=mode,
                role=role,
                rate_limit_bucket=rate_limit_bucket,
                vcs_managers=vcs_managers,
                bucket_rate_limiter=bucket_rate_limiter,
                dry_run=dry_run,
                allow_create_proposal=True,
                require_binary_diff=False,
                requestor=post.get("requestor"),
            ), 'publish of %s/%s, role %s' % (package, campaign, role)
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
    for gpg_entry in list(request.app['gpg'].keylist(secret=True)):
        pgp_keys.append(request.app['gpg'].key_export_minimal(gpg_entry.fpr).decode())
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


async def create_app(
    *,
    vcs_managers: Dict[str, VcsManager],
    db: asyncpg.pool.Pool,
    redis,
    config,
    publish_worker: Optional[PublishWorker] = None,
    dry_run: bool = False,
    forge_rate_limiter: Optional[Dict[str, datetime]] = None,
    bucket_rate_limiter: Optional[RateLimiter] = None,
    require_binary_diff: bool = False,
    push_limit: Optional[int] = None,
    modify_mp_limit: Optional[int] = None,
):
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(middlewares=[
        trailing_slash_redirect, state.asyncpg_error_middleware])
    app.router.add_routes(routes)
    app['gpg'] = gpg.Context(armor=True)
    app['publish_worker'] = publish_worker
    app['vcs_managers'] = vcs_managers
    app['db'] = db
    app['redis'] = redis
    app['config'] = config
    if bucket_rate_limiter is None:
        bucket_rate_limiter = NonRateLimiter()
    app['bucket_rate_limiter'] = bucket_rate_limiter
    if forge_rate_limiter is None:
        forge_rate_limiter = {}
    app['forge_rate_limiter'] = forge_rate_limiter
    app['modify_mp_limit'] = modify_mp_limit
    app['dry_run'] = dry_run
    app['push_limit'] = push_limit
    app['require_binary_diff'] = require_binary_diff
    setup_metrics(app)
    setup_aiohttp_apispec(
        app=app,
        title="Publish Documentation",
        version=None,
        url="/swagger.json",
        swagger_path="/docs",
    )
    return app


async def run_web_server(listen_addr, port, **kwargs):
    app = await create_app(**kwargs)
    config = kwargs['config']
    endpoint = aiozipkin.create_endpoint("janitor.publish", ipv4=listen_addr, port=port)
    if config.zipkin_address:
        tracer = await aiozipkin.create(config.zipkin_address, endpoint, sample_rate=0.1)
    else:
        tracer = await aiozipkin.create_custom(endpoint)

    aiozipkin.setup(app, tracer)

    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    logger.info("Listening on %s:%s", listen_addr, port)
    await site.start()


@routes.get("/health", name="health")
async def handle_health(request):
    return web.Response(text="ok")


@routes.get("/ready", name="ready")
async def handle_ready(request):
    return web.Response(text="ok")


async def get_mp_status(mp):
    if await asyncio.to_thread(mp.is_merged):
        return "merged"
    elif await asyncio.to_thread(mp.is_closed):
        return "closed"
    else:
        return "open"


@routes.post("/scan", name='scan')
async def scan_request(request):
    async def scan():
        async with request.app['db'].acquire() as conn:
            await check_existing(
                conn=conn,
                redis=request.app['redis'],
                config=request.app['config'],
                publish_worker=request.app['publish_worker'],
                bucket_rate_limiter=request.app['bucket_rate_limiter'],
                forge_rate_limiter=request.app['forge_rate_limiter'],
                vcs_managers=request.app['vcs_managers'],
                dry_run=request.app['dry_run'],
                modify_limit=request.app['modify_mp_limit'],
            )

    create_background_task(scan(), 'merge proposal refresh scan')
    return web.Response(status=202, text="Scan started.")


@routes.post("/check-stragglers", name='check-stragglers')
async def refresh_stragglers(request):
    async def scan(db, redis, urls):
        async with db.acquire() as conn:
            proposal_info_manager = ProposalInfoManager(conn, redis)
            for url in urls:
                await check_straggler(proposal_info_manager, url)

    ndays = int(request.query.get('ndays', 5))
    async with request.app['db'].acquire() as conn:
        proposal_info_manager = ProposalInfoManager(conn, request.app['redis'])
        urls = await proposal_info_manager.iter_outdated_proposal_info_urls(ndays)
    create_background_task(scan(request.app['db'], request.app['redis'], urls), 'Refresh of straggling merge proposals')
    return web.json_response(urls)


@routes.post("/refresh-status", name='refresh-status')
async def refresh_proposal_status_request(request):
    post = await request.post()
    try:
        url = post["url"]
    except KeyError as e:
        raise web.HTTPBadRequest(body="missing url parameter") from e
    logger.info("Request to refresh proposal status for %s", url)

    async def scan():
        mp = await asyncio.to_thread(get_proposal_by_url, url)
        async with request.app['db'].acquire() as conn:
            status = await get_mp_status(mp)
            try:
                await check_existing_mp(
                    conn=conn,
                    redis=request.app['redis'],
                    config=request.app['config'],
                    publish_worker=request.app['publish_worker'],
                    mp=mp,
                    status=status,
                    vcs_managers=request.app['vcs_managers'],
                    bucket_rate_limiter=request.app['bucket_rate_limiter'],
                    dry_run=request.app['dry_run'],
                )
            except NoRunForMergeProposal as e:
                logger.warning(
                    "Unable to find stored metadata for %s, skipping.", e.mp.url
                )
            except BranchRateLimited:
                logger.warning("Rate-limited accessing %s. ", mp.url)
    create_background_task(scan(), 'Refresh of proposal %s' % url)
    return web.Response(status=202, text="Refresh of proposal started.")


@routes.post("/autopublish", name='autopublish')
async def autopublish_request(request):
    reviewed_only = "reviewed_only" in request.query

    async def autopublish():
        await publish_pending_ready(
            db=request.app['db'],
            redis=request.app['redis'],
            config=request.app['config'],
            publish_worker=request.app['publish_worker'],
            bucket_rate_limiter=request.app['bucket_rate_limiter'],
            vcs_managers=request.app['vcs_managers'],
            dry_run=request.app['dry_run'],
            push_limit=request.app['push_limit'],
            reviewed_only=reviewed_only,
            require_binary_diff=request.app['require_binary_diff'],
        )

    create_background_task(autopublish(), 'autopublish')
    return web.Response(status=202, text="Autopublish started.")


@routes.get("/rate-limits/{bucket}", name="bucket-rate-limits")
async def bucket_rate_limits_request(request):
    bucket_rate_limiter = request.app['bucket_rate_limiter']

    stats = bucket_rate_limiter.get_stats()

    (current_open, max_open) = stats.get(
        request.match_info['bucket'], (None, None))

    ret = {
        'open': current_open,
        'max_open': max_open,
        'remaining':
            None if (current_open is None or max_open is None)
            else max_open - current_open}

    return web.json_response(ret)


async def get_previous_mp_status(conn, codebase, campaign):
    rows = await conn.fetch("""\
SELECT run.id, ARRAY_AGG((merge_proposal.url, merge_proposal.status))
FROM run
INNER JOIN merge_proposal ON run.revision = merge_proposal.revision
WHERE run.codebase = $1
AND run.suite = $2
AND run.result_code = 'success'
AND merge_proposal.status NOT IN ('open', 'abandoned')
GROUP BY run.id
ORDER BY run.finish_time DESC
""", codebase, campaign)
    if len(rows) == 0:
        return []

    return rows[0]


@routes.get("/rate-limits", name="rate-limits")
async def rate_limits_request(request):
    bucket_rate_limiter = request.app['bucket_rate_limiter']

    per_bucket = {}
    for bucket, (current_open, max_open) in (
            bucket_rate_limiter.get_stats().items()):
        per_bucket[bucket] = {
            'open': current_open,
            'max_open': max_open,
            'remaining': (
                None if (current_open is None or max_open is None)
                else max_open - current_open)}

    return web.json_response({
        'proposals_per_bucket': per_bucket,
        'per_forge': {
            str(f): dt.isoformat()
            for f, dt in request.app['forge_rate_limiter'].items()},
        'push_limit': request.app['push_limit']})


@routes.get("/blockers/{run_id}", name='blockers')
async def blockers_request(request):
    span = aiozipkin.request_span(request)
    async with request.app['db'].acquire() as conn:
        with span.new_child('sql:publish-status'):
            run = await conn.fetchrow("""\
SELECT
  run.id AS id,
  run.codebase AS codebase,
  run.campaign AS campaign,
  run.finish_time AS finish_time,
  run.review_status AS review_status,
  run.command AS run_command,
  named_publish_policy.qa_review AS qa_review_policy,
  named_publish_policy.rate_limit_bucket AS rate_limit_bucket,
  run.revision AS revision,
  candidate.command AS policy_command,
  package.removed AS removed,
  run.result_code AS result_code,
  change_set.state AS change_set_state,
  change_set.id AS change_set
FROM run
LEFT JOIN package ON package.name = run.package
INNER JOIN candidate ON candidate.package = run.package AND candidate.suite = run.suite
INNER JOIN named_publish_policy ON candidate.publish_policy = named_publish_policy.name
INNER JOIN change_set ON change_set.id = run.change_set
WHERE run.id = $1
""", request.match_info['run_id'])

        if run is None:
            return web.json_response({
                'reason': 'No such publish-ready run',
                'run_id': request.match_info['run_id']}, status=404)

        with span.new_child('sql:reviews'):
            reviews = await conn.fetch(
                "SELECT * FROM review WHERE run_id = $1", run['id'])

        if run['revision'] is not None:
            with span.new_child('sql:publish-attempt-count'):
                attempt_count = await get_publish_attempt_count(
                    conn, run['revision'].encode('utf-8'),
                    {"differ-unreachable"})
        else:
            attempt_count = 0

        with span.new_child('sql:last-mp'):
            last_mps = await get_previous_mp_status(
                conn, run['codebase'], run['campaign'])
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
            'reviews': {review['reviewer']: {
                'timestamp': review['reviewed_at'].isoformat(),
                'comment': review['comment'],
                'status': review['review_status']} for review in reviews},
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

    ret['propose_rate_limit'] = {
        'details': {
            'bucket': run['rate_limit_bucket']}}
    try:
        request.app['bucket_rate_limiter'].check_allowed(run['rate_limit_bucket'])
    except BucketRateLimited as e:
        ret['propose_rate_limit']['result'] = False
        ret['propose_rate_limit']['details'] = {
            'open': e.open_mps,
            'max_open': e.max_open_mps}
    except RateLimited:
        ret['propose_rate_limit']['result'] = False
    else:
        ret['propose_rate_limit']['result'] = True

    ret['change_set'] = {
        'result': (run['change_set_state'] in ('publishing', 'ready')),
        'details': {
            'change_set_id': run['change_set'],
            'change_set_state': run['change_set_state']}}

    ret['previous_mp'] = {
        'result': any(last_mp[1] not in ('rejected', 'closed')
                      for last_mp in last_mps),
        'details': [{
            'url': last_mp[0],
            'status': last_mp[1]
        } for last_mp in last_mps]
    }

    return web.json_response(ret)


async def process_queue_loop(
    *,
    db,
    redis,
    config,
    publish_worker,
    bucket_rate_limiter,
    forge_rate_limiter,
    dry_run,
    vcs_managers,
    interval,
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
                conn=conn,
                redis=redis,
                config=config,
                publish_worker=publish_worker,
                bucket_rate_limiter=bucket_rate_limiter,
                forge_rate_limiter=forge_rate_limiter,
                vcs_managers=vcs_managers,
                dry_run=dry_run,
                modify_limit=modify_mp_limit,
            )
            await check_stragglers(conn, redis)
        if auto_publish:
            await publish_pending_ready(
                db=db,
                redis=redis,
                config=config,
                publish_worker=publish_worker,
                bucket_rate_limiter=bucket_rate_limiter,
                vcs_managers=vcs_managers,
                dry_run=dry_run,
                reviewed_only=reviewed_only,
                push_limit=push_limit,
                require_binary_diff=require_binary_diff)
        cycle_duration = datetime.utcnow() - cycle_start
        to_wait = max(0, interval - cycle_duration.total_seconds())
        logger.info("Waiting %d seconds for next cycle." % to_wait)
        if to_wait > 0:
            await asyncio.sleep(to_wait)


class NoRunForMergeProposal(Exception):
    """No run matching merge proposal."""

    def __init__(self, mp, revision):
        self.mp = mp
        self.revision = revision


async def get_last_effective_run(conn, package, campaign):
    query = """
SELECT
    id, command, start_time, finish_time, description, package,
    result_code,
    value, main_branch_revision, revision, context, result, suite,
    instigated_context, vcs_type, branch_url, logfilenames, review_status,
    worker,
    array(SELECT row(role, remote_name, base_revision,
     revision) FROM new_result_branch WHERE run_id = id) AS result_branches,
    result_tags, target_branch_url, change_set AS change_set,
    failure_transient, failure_stage, codebase
FROM
    last_effective_runs
WHERE package = $1 AND suite = $2
LIMIT 1
"""
    row = await conn.fetchrow(query, package, campaign)
    if row is None:
        return None
    return state.Run.from_row(row)


async def get_merge_proposal_run(
        conn: asyncpg.Connection, mp_url: str) -> asyncpg.Record:
    query = """
SELECT
    run.id AS id,
    run.package AS package,
    run.suite AS campaign,
    run.branch_url AS branch_url,
    run.command AS command,
    run.value AS value,
    rb.role AS role,
    rb.remote_name AS remote_branch_name,
    rb.revision AS revision,
    run.codebase AS codebase,
    run.change_set AS change_set
FROM new_result_branch rb
RIGHT JOIN run ON rb.run_id = run.id
WHERE rb.revision IN (
    SELECT revision from merge_proposal WHERE merge_proposal.url = $1)
ORDER BY run.finish_time DESC
LIMIT 1
"""
    return await conn.fetchrow(query, mp_url)


@dataclass
class ProposalInfo:

    can_be_merged: Optional[bool]
    status: str
    revision: bytes
    target_branch_url: Optional[str]
    rate_limit_bucket: Optional[str] = None
    package_name: Optional[str] = None


async def guess_proposal_info_from_revision(
    conn: asyncpg.Connection, revision: bytes
) -> Tuple[Optional[str], Optional[str]]:
    query = """\
SELECT DISTINCT run.package, named_publish_policy.rate_limit_bucket AS rate_limit_bucket
FROM run
LEFT JOIN new_result_branch rb ON rb.run_id = run.id
INNER JOIN candidate ON run.package = candidate.package AND run.suite = candidate.suite
INNER JOIN named_publish_policy ON named_publish_policy.name = candidate.publish_policy
WHERE rb.revision = $1 AND run.package is not null
"""
    rows = await conn.fetch(query, revision.decode("utf-8"))
    if len(rows) == 1:
        return rows[0][0], rows[0][1]
    return None, None


async def guess_rate_limit_bucket(
        conn: asyncpg.Connection, package_name: str, source_branch_name: str):
    # For now, just assume that source_branch_name is campaign
    campaign = source_branch_name.split('/')[0]
    query = """\
SELECT named_publish_policy.rate_limit_bucket FROM candidate
INNER JOIN named_publish_policy ON named_publish_policy.name = candidate.publish_policy
WHERE candidate.suite = $1 AND candidate.package = $2
"""
    return await conn.fetchval(query, campaign, package_name)


async def guess_package_from_branch_url(
        conn: asyncpg.Connection, url: str,
        possible_transports: Optional[List[Transport]] = None):
    query = """
SELECT
  name, branch_url
FROM
  package
WHERE
  TRIM(trailing '/' from branch_url) = ANY($1::text[])
ORDER BY length(branch_url) DESC
"""
    repo_url, params = urlutils.split_segment_parameters(url.rstrip('/'))
    try:
        branch = urlutils.unescape(params['branch'])
    except KeyError:
        branch = None
    options = [
        url.rstrip('/'),
        repo_url.rstrip('/'),
    ]
    result = await conn.fetchrow(query, options)
    if result is None:
        return None

    if url.rstrip('/') == result['branch_url'].rstrip('/'):
        return result['name']

    source_branch = await asyncio.to_thread(
        open_branch,
        result['branch_url'].rstrip('/'),
        possible_transports=possible_transports)
    if (source_branch.controldir.user_url.rstrip('/') != url.rstrip('/')
            and source_branch.name != branch):
        logging.info(
            'Did not resolve branch URL to package: %r (%r) != %r (%r)',
            source_branch.user_url, source_branch.name, url, branch)
        return None
    return result['name']


def find_campaign_by_branch_name(config, branch_name):
    for campaign in config.campaign:
        if campaign.branch_name == branch_name:
            return campaign.name, 'main'
    return None, None


class ProposalInfoManager(object):

    def __init__(self, conn: asyncpg.Connection, redis):
        self.conn = conn
        self.redis = redis

    async def iter_outdated_proposal_info_urls(self, days):
        return [row['url'] for row in await self.conn.fetch(
            "SELECT url FROM merge_proposal WHERE "
            "last_scanned is NULL OR now() - last_scanned > interval '%d days'" % days)]

    async def get_proposal_info(self, url) -> Optional[ProposalInfo]:
        row = await self.conn.fetchrow(
            """\
    SELECT
        merge_proposal.rate_limit_bucket AS rate_limit_bucket,
        merge_proposal.revision,
        merge_proposal.status,
        merge_proposal.target_branch_url,
        package.name AS package,
        can_be_merged
    FROM
        merge_proposal
    LEFT JOIN package ON merge_proposal.package = package.name
    WHERE
        merge_proposal.url = $1
    """,
            url,
        )
        if not row:
            return None
        return ProposalInfo(
            rate_limit_bucket=row['rate_limit_bucket'],
            revision=row['revision'].encode("utf-8") if row[1] else None,
            status=row['status'],
            target_branch_url=row['target_branch_url'],
            package_name=row['package'],
            can_be_merged=row['can_be_merged'])

    async def delete_proposal_info(self, url):
        await self.conn.execute('DELETE FROM merge_proposal WHERE url = $1', url)

    async def update_canonical_url(self, old_url: str, canonical_url: str):
        async with self.conn.transaction():
            old_url = await self.conn.fetchval(
                'UPDATE merge_proposal canonical SET package = COALESCE(canonical.package, old.package), '
                'rate_limit_bucket = COALESCE(canonical.rate_limit_bucket, old.rate_limit_bucket) '
                'FROM merge_proposal old WHERE old.url = $1 AND canonical.url = $2 RETURNING old.url',
                old_url, canonical_url)
            await self.conn.execute(
                'UPDATE publish SET merge_proposal_url = $1 WHERE merge_proposal_url = $2',
                canonical_url, old_url)
            if old_url:
                await self.conn.execute(
                    'DELETE FROM merge_proposal WHERE url = $1', old_url)
            else:
                await self.conn.execute(
                    "UPDATE merge_proposal SET url = $1 WHERE url = $2",
                    canonical_url, old_url)

    async def update_proposal_info(
            self, mp, *, status, revision, package_name, target_branch_url,
            campaign, can_be_merged: Optional[bool], rate_limit_bucket: Optional[str],
            dry_run: bool = False):
        if status == "closed":
            # TODO(jelmer): Check if changes were applied manually and mark
            # as applied rather than closed?
            pass
        if status == "merged":
            merged_by = await asyncio.to_thread(mp.get_merged_by)
            merged_by_url = await asyncio.to_thread(
                get_merged_by_user_url, mp.url, merged_by)
            merged_at = await asyncio.to_thread(mp.get_merged_at)
            if merged_at is not None:
                merged_at = merged_at.replace(tzinfo=None)
        else:
            merged_by = None
            merged_by_url = None
            merged_at = None
        if not dry_run:
            async with self.conn.transaction():
                await self.conn.execute(
                    """INSERT INTO merge_proposal (
                        url, status, revision, package, merged_by, merged_at,
                        target_branch_url, last_scanned, can_be_merged, rate_limit_bucket)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), $8, $9)
                    ON CONFLICT (url)
                    DO UPDATE SET
                      status = EXCLUDED.status,
                      revision = EXCLUDED.revision,
                      package = EXCLUDED.package,
                      merged_by = EXCLUDED.merged_by,
                      merged_at = EXCLUDED.merged_at,
                      target_branch_url = EXCLUDED.target_branch_url,
                      last_scanned = EXCLUDED.last_scanned,
                      can_be_merged = EXCLUDED.can_be_merged,
                      rate_limit_bucket = EXCLUDED.rate_limit_bucket
                    """, mp.url, status,
                    revision.decode("utf-8") if revision is not None else None,
                    package_name, merged_by, merged_at, target_branch_url,
                    can_be_merged, rate_limit_bucket)
                if revision:
                    await self.conn.execute("""
                    UPDATE new_result_branch SET absorbed = $1 WHERE revision = $2
                    """, (status == 'merged'), revision.decode('utf-8'))

            # TODO(jelmer): Check if the change_set should be marked as published

            await self.redis.publish('merge-proposal', json.dumps({
                "url": mp.url,
                "target_branch_url": target_branch_url,
                "rate_limit_bucket": rate_limit_bucket,
                "status": status,
                "package": package_name,
                "merged_by": merged_by,
                "merged_by_url": merged_by_url,
                "merged_at": str(merged_at),
                "campaign": campaign,
            }))


async def abandon_mp(proposal_info_manager: ProposalInfoManager,
                     mp: MergeProposal, revision: bytes,
                     package_name: Optional[str], target_branch_url: str,
                     campaign: Optional[str], can_be_merged: Optional[bool],
                     rate_limit_bucket: Optional[str],
                     comment: Optional[str], dry_run: bool = False):
    if comment:
        logger.info('%s: %s', mp.url, comment)
    if dry_run:
        return
    await proposal_info_manager.update_proposal_info(
        mp, status="abandoned", revision=revision, package_name=package_name,
        target_branch_url=target_branch_url, campaign=campaign,
        rate_limit_bucket=rate_limit_bucket, can_be_merged=can_be_merged)
    if comment:
        try:
            await asyncio.to_thread(mp.post_comment, comment)
        except PermissionDenied as e:
            logger.warning(
                "Permission denied posting comment to %s: %s", mp.url, e)

    try:
        await asyncio.to_thread(mp.close)
    except PermissionDenied as e:
        logger.warning(
            "Permission denied closing merge request %s: %s", mp.url, e
        )
        raise


async def close_applied_mp(proposal_info_manager, mp: MergeProposal,
                           revision: bytes, package_name: Optional[str],
                           target_branch_url: str,
                           campaign: Optional[str], can_be_merged: Optional[bool],
                           rate_limit_bucket: Optional[str],
                           comment: Optional[str], dry_run=False):

    await proposal_info_manager.update_proposal_info(
        mp, status="applied", revision=revision, package_name=package_name,
        target_branch_url=target_branch_url, campaign=campaign,
        can_be_merged=can_be_merged, rate_limit_bucket=rate_limit_bucket,
        dry_run=dry_run)
    try:
        await asyncio.to_thread(mp.post_comment, comment)
    except PermissionDenied as e:
        logger.warning(
            "Permission denied posting comment to %s: %s", mp.url, e)

    try:
        await asyncio.to_thread(mp.close)
    except PermissionDenied as e:
        logger.warning(
            "Permission denied closing merge request %s: %s", mp.url, e
        )
        raise


async def check_stragglers(conn, redis):
    proposal_info_manager = ProposalInfoManager(conn, redis)
    stragglers = await proposal_info_manager.iter_outdated_proposal_info_urls(5)
    for url in stragglers:
        await check_straggler(proposal_info_manager, url)


async def check_straggler(proposal_info_manager, url):
    # Find the canonical URL
    async with ClientSession() as session:
        async with session.get(url) as resp:
            if resp.status == 200 and resp.url != url:
                await proposal_info_manager.update_canonical_url(
                    url, resp.url)
            if resp.status == 404:
                # TODO(jelmer): Keep it but leave a tumbestone around?
                await proposal_info_manager.delete_proposal_info(url)
            else:
                logging.warning(
                    'Got status %d loading straggler %r', url)


async def check_existing_mp(
    conn,
    redis,
    config,
    publish_worker,
    mp,
    status,
    vcs_managers,
    bucket_rate_limiter,
    dry_run: bool,
    mps_per_bucket=None,
    possible_transports: Optional[List[Transport]] = None,
    check_only: bool = False,
    close_below_threshold: bool = True,
) -> bool:
    proposal_info_manager = ProposalInfoManager(conn, redis)
    old_proposal_info = await proposal_info_manager.get_proposal_info(mp.url)
    if old_proposal_info:
        package_name = old_proposal_info.package_name
        rate_limit_bucket = old_proposal_info.rate_limit_bucket
    else:
        package_name = None
        rate_limit_bucket = None
    revision = await asyncio.to_thread(mp.get_source_revision)
    source_branch_url = await asyncio.to_thread(mp.get_source_branch_url)
    try:
        can_be_merged = await asyncio.to_thread(mp.can_be_merged)
    except NotImplementedError:
        # TODO(jelmer): Download and attempt to merge locally?
        can_be_merged = None

    if revision is None:
        if source_branch_url is None:
            logger.warning("No source branch for %r", mp)
            revision = None
            source_branch_name = None
        else:
            try:
                source_branch = await asyncio.to_thread(
                    open_branch,
                    source_branch_url, possible_transports=possible_transports)
            except (BranchMissing, BranchUnavailable):
                revision = None
                source_branch_name = None
            else:
                revision = await asyncio.to_thread(source_branch.last_revision)
                source_branch_name = source_branch.name
    else:
        source_branch_name = None
    if source_branch_name is None and source_branch_url is not None:
        segment_params = urlutils.split_segment_parameters(source_branch_url)[1]
        source_branch_name = segment_params.get("branch")
        if source_branch_name is not None:
            source_branch_name = urlutils.unescape(source_branch_name)
    if revision is None and old_proposal_info:
        revision = old_proposal_info.revision
    target_branch_url = await asyncio.to_thread(mp.get_target_branch_url)
    if rate_limit_bucket is None:
        package_name = await guess_package_from_branch_url(
            conn, target_branch_url,
            possible_transports=possible_transports)
        if package_name is None:
            if revision is not None:
                (
                    package_name,
                    rate_limit_bucket,
                ) = await guess_proposal_info_from_revision(conn, revision)
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
        else:
            if source_branch_name is not None:
                rate_limit_bucket = await guess_rate_limit_bucket(
                    conn, package_name, source_branch_name)
    if old_proposal_info and old_proposal_info.status in ("abandoned", "applied", "rejected") and status == "closed":
        status = old_proposal_info.status

    if old_proposal_info is None or (
            old_proposal_info.status != status
            or revision != old_proposal_info.revision
            or target_branch_url != old_proposal_info.target_branch_url
            or rate_limit_bucket != old_proposal_info.rate_limit_bucket
            or can_be_merged != old_proposal_info.can_be_merged):
        mp_run = await get_merge_proposal_run(conn, mp.url)
        await proposal_info_manager.update_proposal_info(
            mp, status=status, revision=revision, package_name=package_name,
            target_branch_url=target_branch_url,
            campaign=mp_run['campaign'] if mp_run else None,
            can_be_merged=can_be_merged, rate_limit_bucket=rate_limit_bucket,
            dry_run=dry_run)
    else:
        await conn.execute(
            'UPDATE merge_proposal SET last_scanned = NOW() WHERE url = $1',
            mp.url)
        mp_run = None
    if rate_limit_bucket is not None and mps_per_bucket is not None:
        mps_per_bucket[status].setdefault(rate_limit_bucket, 0)
        mps_per_bucket[status][rate_limit_bucket] += 1
    if status != "open":
        return False
    if check_only:
        return False

    if mp_run is None:
        mp_run = await get_merge_proposal_run(conn, mp.url)

    if mp_run is None:
        # If we don't have any information about this merge proposal, then
        # it might be one that we lost the data for. Let's reschedule.
        if package_name and source_branch_name:
            campaign, role = find_campaign_by_branch_name(config, source_branch_name)
            if campaign:
                logging.warning(
                    'Recovered orphaned merge proposal %s', mp.url)
                last_run = await get_last_effective_run(
                    conn, package_name, campaign)
                if last_run is None:
                    try:
                        await do_schedule(
                            conn,
                            package=package_name,
                            campaign=campaign,
                            change_set=None,
                            bucket="update-existing-mp",
                            refresh=True,
                            requestor="publisher (orphaned merge proposal)",
                            # TODO(jelmer): Determine codebase
                            codebase=None
                        )
                    except CandidateUnavailable as e:
                        logging.warning(
                            'Candidate unavailable while attempting to reschedule '
                            'orphaned %s: %s/%s',
                            mp.url, package_name, campaign)
                        raise NoRunForMergeProposal(mp, revision) from e
                    else:
                        logging.warning('Rescheduled')
                        return False
                else:
                    mp_run = {
                        'remote_branch_name': None,
                        'package': package_name,
                        'campaign': campaign,
                        'change_set': None,
                        'codebase': None,
                        'role': role,
                        'id': None,
                        'branch_url': target_branch_url,
                        'revision': revision.decode('utf-8'),
                        'value': None,
                    }
                    logging.warning('Going ahead with dummy old run')
            else:
                raise NoRunForMergeProposal(mp, revision)
        else:
            raise NoRunForMergeProposal(mp, revision)

    mp_remote_branch_name = mp_run['remote_branch_name']

    if mp_remote_branch_name is None:
        if target_branch_url is None:
            logger.warning("No target branch for %r", mp)
        else:
            try:
                mp_remote_branch_name = (await asyncio.to_thread(
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
            "%s: package has been removed from the archive, closing proposal.",
            mp.url,
        )
        try:
            await abandon_mp(
                proposal_info_manager, mp, revision, package_name,
                target_branch_url, campaign=mp_run['campaign'],
                can_be_merged=can_be_merged, rate_limit_bucket=rate_limit_bucket,
                comment="""\
This merge proposal will be closed, since the package has been removed from the \
archive.
""", dry_run=dry_run)
        except PermissionDenied:
            return False
        return True

    if last_run.result_code == "nothing-to-do":
        # A new run happened since the last, but there was nothing to
        # do.
        logger.info(
            "%s: Last run did not produce any changes, closing proposal.", mp.url
        )

        try:
            await close_applied_mp(
                proposal_info_manager, mp, revision, package_name, target_branch_url,
                mp_run['campaign'], can_be_merged=can_be_merged,
                rate_limit_bucket=rate_limit_bucket, comment="""
This merge proposal will be closed, since all remaining changes have been \
applied independently.
""", dry_run=dry_run)
        except PermissionDenied:
            return False
        else:
            return True

    if last_run.result_code != "success":
        last_run_age = datetime.utcnow() - last_run.finish_time
        if last_run.failure_transient:
            logger.info(
                "%s: Last run failed with transient error (%s). Rescheduling.",
                mp.url,
                last_run.result_code,
            )
            try:
                await do_schedule(
                    conn,
                    package=last_run.package,
                    campaign=last_run.campaign,
                    change_set=last_run.change_set,
                    bucket="update-existing-mp",
                    refresh=False,
                    requestor="publisher (transient error)",
                    codebase=last_run.codebase,
                )
            except CandidateUnavailable as e:
                logging.warning(
                    'Candidate unavailable while attempting to reschedule %s/%s: %s',
                    last_run.package, last_run.campaign, e)
        elif last_run_age.days > EXISTING_RUN_RETRY_INTERVAL:
            logger.info(
                "%s: Last run failed (%s) a long time ago (%d days). Rescheduling.",
                mp.url,
                last_run.result_code,
                last_run_age.days,
            )
            try:
                await do_schedule(
                    conn,
                    package=last_run.package,
                    campaign=last_run.campaign,
                    change_set=last_run.change_set,
                    bucket="update-existing-mp",
                    refresh=False,
                    requestor="publisher (retrying failed run after %d days)"
                    % last_run_age.days,
                    codebase=last_run.codebase,
                )
            except CandidateUnavailable as e:
                logging.warning(
                    'Candidate unavailable while attempting to reschedule %s/%s: %s',
                    last_run.package, last_run.campaign, e)
        else:
            logger.info(
                "%s: Last run failed (%s). Not touching merge proposal.",
                mp.url,
                last_run.result_code,
            )
        return False

    campaign_config = get_campaign_config(config, mp_run['campaign'])

    if close_below_threshold and not run_sufficient_for_proposal(
            campaign_config, mp_run['value']):
        try:
            await abandon_mp(
                proposal_info_manager, mp, revision, package_name, target_branch_url,
                campaign=mp_run['campaign'], can_be_merged=can_be_merged,
                rate_limit_bucket=rate_limit_bucket,
                comment="This merge proposal will be closed, since only trivial changes are left.",
                dry_run=dry_run)
        except PermissionDenied:
            return False
        return True

    try:
        (
            last_run_remote_branch_name,
            last_run_base_revision,
            last_run_revision,
        ) = last_run.get_result_branch(mp_run['role'])
    except KeyError:
        logger.warning(
            "%s: Merge proposal run %s had role %s but it is gone now (%s)",
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
            "%s: Remote branch name has changed: %s => %s ",
            mp.url,
            mp_remote_branch_name,
            last_run_remote_branch_name,
        )
        # Note that we require that mp_remote_branch_name is set.
        # For some old runs it is not set because we didn't track
        # the default branch name.
        if not dry_run and mp_remote_branch_name is not None:
            try:
                await asyncio.to_thread(
                    mp.set_target_branch_name,
                    last_run_remote_branch_name or "")
            except NotImplementedError:
                logger.info(
                    "%s: Closing merge proposal, since branch for role "
                    "'%s' has changed from %s to %s.",
                    mp.url,
                    mp_run['role'],
                    mp_remote_branch_name,
                    last_run_remote_branch_name,
                )
                try:
                    await abandon_mp(
                        proposal_info_manager, mp, revision, package_name, target_branch_url,
                        rate_limit_bucket=rate_limit_bucket, campaign=mp_run['campaign'],
                        can_be_merged=can_be_merged, comment="""\
This merge proposal will be closed, since the branch for the role '%s'
has changed from %s to %s.
""" % (mp_run['role'], mp_remote_branch_name, last_run_remote_branch_name), dry_run=dry_run)
                except PermissionDenied:
                    return False
                return True
            else:
                target_branch_url = role_branch_url(
                    mp_run['branch_url'], mp_remote_branch_name)
        else:
            return False

    if not await asyncio.to_thread(branches_match, mp_run['branch_url'], last_run.branch_url):
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
        try:
            await abandon_mp(
                proposal_info_manager, mp, revision, package_name, target_branch_url,
                campaign=mp_run['campaign'], can_be_merged=can_be_merged,
                rate_limit_bucket=rate_limit_bucket, comment="""\
This merge proposal will be closed, since the branch has moved to %s.
""" % (bzr_to_browse_url(last_run.branch_url),), dry_run=dry_run)
        except PermissionDenied:
            return False
        return True

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
                "%s (%s): old run (%s/%s) has same revision as new run (%s/%s): %r",
                mp.url,
                mp_run['package'],
                mp_run['id'],
                mp_run['role'],
                last_run.id,
                mp_run['role'],
                mp_run['revision'].encode('utf-8'),
            )
        if source_branch_name is None:
            source_branch_name = await derived_branch_name(
                conn, campaign_config, last_run, mp_run['role'])

        unchanged_run_id = await conn.fetchval(
            "SELECT id FROM run "
            "WHERE revision = $2 AND package = $1 and result_code = 'success' "
            "ORDER BY finish_time DESC LIMIT 1",
            last_run.package, last_run.main_branch_revision.decode('utf-8')
        )

        try:
            publish_result = await publish_worker.publish_one(
                campaign=last_run.campaign,
                pkg=last_run.package,
                command=last_run.command,
                codemod_result=last_run.result,
                main_branch_url=target_branch_url,
                mode=MODE_PROPOSE,
                role=mp_run['role'],
                revision=last_run_revision,
                log_id=last_run.id,
                unchanged_id=unchanged_run_id,
                derived_branch_name=source_branch_name,
                rate_limit_bucket=rate_limit_bucket,
                vcs_manager=vcs_managers[last_run.vcs_type],
                dry_run=dry_run,
                require_binary_diff=False,
                allow_create_proposal=True,
                bucket_rate_limiter=bucket_rate_limiter,
                result_tags=last_run.result_tags,
                commit_message_template=(
                    campaign_config.merge_proposal.commit_message
                    if campaign_config.merge_proposal else None),
                title_template=(
                    campaign_config.merge_proposal.title
                    if campaign_config.merge_proposal else None),
                existing_mp_url=mp.url,
            )
        except BranchBusy as e:
            logger.info(
                '%s: Branch %r was busy while publishing',
                mp.url, e.branch_url)
            return False
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
                try:
                    await close_applied_mp(
                        proposal_info_manager, mp, revision, package_name,
                        target_branch_url, campaign=mp_run['campaign'],
                        can_be_merged=can_be_merged, rate_limit_bucket=rate_limit_bucket,
                        comment="""
This merge proposal will be closed, since all remaining changes have been \
applied independently.
""", dry_run=dry_run)
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
                    change_set_id=last_run.change_set,
                    package=last_run.package,
                    branch_name=campaign_config.branch_name,
                    main_branch_revision=last_run_base_revision,
                    revision=last_run_revision,
                    role=mp_run['role'],
                    mode=e.mode,
                    result_code=code,
                    description=description,
                    merge_proposal_url=mp.url,
                    publish_id=publish_id,
                    requestor="publisher (regular refresh)",
                    run_id=last_run.id,
                )
        else:
            if not dry_run:
                await store_publish(
                    conn,
                    change_set_id=last_run.change_set,
                    package=last_run.package,
                    branch_name=publish_result.branch_name,
                    main_branch_revision=last_run_base_revision,
                    revision=last_run_revision,
                    role=mp_run['role'],
                    mode=MODE_PROPOSE,
                    result_code="success",
                    description=(publish_result.description or "Succesfully updated"),
                    merge_proposal_url=publish_result.proposal_url,
                    target_branch_url=target_branch_url,
                    publish_id=publish_id,
                    requestor="publisher (regular refresh)",
                    run_id=last_run.id,
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
        if can_be_merged is False:
            logger.info("%s can not be merged (conflict?). Rescheduling.", mp.url)
            if not dry_run:
                try:
                    await do_schedule(
                        conn,
                        package=mp_run['package'],
                        campaign=mp_run['campaign'],
                        change_set=mp_run['change_set'],
                        bucket="update-existing-mp",
                        refresh=True,
                        requestor="publisher (merge conflict)",
                        codebase=mp_run['codebase'],
                    )
                except CandidateUnavailable:
                    logging.warning(
                        'Candidate unavailable while attempting to reschedule '
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
    *,
    conn,
    redis,
    config,
    publish_worker,
    bucket_rate_limiter,
    forge_rate_limiter: Dict[Forge, datetime],
    vcs_managers,
    dry_run: bool,
    modify_limit=None,
    unexpected_limit: int = 5,
):
    mps_per_bucket: Dict[str, Dict[str, int]] = {
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
                forge_rate_limited_count.labels(forge=str(forge)).inc()
                was_forge_ratelimited = True
                continue
        try:
            modified = await check_existing_mp(
                conn=conn,
                redis=redis,
                config=config,
                publish_worker=publish_worker,
                mp=mp,
                status=status,
                vcs_managers=vcs_managers,
                dry_run=dry_run,
                bucket_rate_limiter=bucket_rate_limiter,
                possible_transports=possible_transports,
                mps_per_bucket=mps_per_bucket,
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
            if e.retry_after is None:
                retry_after = timedelta(minutes=30)
            else:
                retry_after = timedelta(seconds=e.retry_after)
            forge_rate_limiter[forge] = datetime.utcnow() + retry_after
            continue
        except UnexpectedHttpStatus as e:
            logging.warning(
                'Got unexpected HTTP status %s, skipping %r',
                e, mp.url)
            # TODO(jelmer): print traceback?
            unexpected += 1

        if unexpected > unexpected_limit:
            unexpected_http_response_count.inc()
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

        bucket_rate_limiter.set_mps_per_bucket(mps_per_bucket)
        total = 0
        for bucket, count in mps_per_bucket["open"].items():
            total += count
            if bucket is not None:
                bucket_proposal_count.labels(bucket=bucket).set(count)
        open_proposal_count.set(total)
    else:
        logging.info('Rate-Limited for forges %r. Not updating stats', forge_rate_limiter)


async def get_run(conn: asyncpg.Connection, run_id):
    query = """
SELECT
    id, command, start_time, finish_time, description, package,
    result_code,
    value, main_branch_revision, revision, context, result, suite,
    instigated_context, vcs_type, branch_url, logfilenames, review_status,
    worker,
    array(SELECT row(role, remote_name, base_revision,
     revision) FROM new_result_branch WHERE run_id = id) AS result_branches,
    result_tags, target_branch_url, change_set as change_set,
    failure_transient AS failure_transient, failure_stage, codebase
FROM
    run
WHERE id = $1
"""
    row = await conn.fetch(query, run_id)
    if row:
        return state.Run.from_row(row)
    return None


async def iter_control_matching_runs(conn: asyncpg.Connection, main_branch_revision: bytes, codebase: str):
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
  worker,
  array(SELECT row(role, remote_name, base_revision,
   revision) FROM new_result_branch WHERE run_id = id) AS result_branches,
  result_tags
FROM last_runs
WHERE main_branch_revision = $1 AND codebase = $2 AND main_branch_revision != revision AND suite NOT in ('unchanged', 'control')
ORDER BY start_time DESC
"""
    return await conn.fetch(
        query, main_branch_revision.decode('utf-8'), codebase)



async def listen_to_runner(
    *,
    db,
    redis,
    config,
    publish_worker,
    bucket_rate_limiter,
    vcs_managers,
    dry_run: bool,
    require_binary_diff: bool = False,
):
    async def process_run(conn, run, branch_url):
        publish_policy, command, rate_limit_bucket = await get_publish_policy(
            conn, run.package, run.campaign)
        for role, (mode, max_frequency_days) in publish_policy.items():
            await publish_from_policy(
                conn=conn,
                campaign_config=get_campaign_config(config, run.campaign),
                publish_worker=publish_worker,
                bucket_rate_limiter=bucket_rate_limiter,
                vcs_managers=vcs_managers,
                run=run,
                redis=redis,
                role=role,
                rate_limit_bucket=rate_limit_bucket,
                main_branch_url=branch_url,
                mode=mode,
                max_frequency_days=max_frequency_days,
                command=command,
                dry_run=dry_run,
                require_binary_diff=require_binary_diff,
                force=True,
                requestor="runner",
            )

    async def handle_result_message(msg):
        result = json.loads(msg['data'])
        if result["code"] != "success":
            return
        async with db.acquire() as conn:
            # TODO(jelmer): Fold these into a single query ?
            codebase = await conn.fetchrow(
                'SELECT branch_url FROM codebase WHERE name = $1',
                result["codebase"])
            if codebase is None:
                logging.warning('Codebase %s not in database?', result['codebase'])
                return
            run = await get_run(conn, result["log_id"])
            if run.campaign != "unchanged":
                await process_run(conn, run, codebase['branch_url'])
            else:
                for dependent_run in await iter_control_matching_runs(
                        conn, main_branch_revision=run.revision,
                        codebase=run.codebase):
                    await process_run(conn, dependent_run, codebase['branch_url'])

    try:
        async with redis.pubsub(ignore_subscribe_messages=True) as ch:
            await ch.subscribe('result', result=handle_result_message)
            await ch.run()
    finally:
        await redis.close()


async def refresh_bucket_mp_counts(db, bucket_rate_limiter):
    per_bucket: Dict[str, Dict[str, int]] = {}
    async with db.acquire() as conn:
        for row in await conn.fetch("""
             SELECT
             rate_limit_bucket AS rate_limit_bucket,
             status AS status,
             count(*) as c
             FROM merge_proposal
             GROUP BY 1, 2
             """):
            per_bucket.setdefault(
                row['status'], {})[row['rate_limit_bucket']] = row['c']
    bucket_rate_limiter.set_mps_per_bucket(per_bucket)


async def main(argv=None):
    import argparse

    parser = argparse.ArgumentParser(prog="janitor.publish")
    parser.add_argument(
        "--max-mps-per-bucket",
        default=0,
        type=int,
        help="Maximum number of open merge proposals per bucket.",
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
        help=("Seconds to wait in between publishing pending proposals"),
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
    parser.add_argument("--debug", action="store_true", help="Print debugging info")
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
    elif args.debug:
        logging.basicConfig(level=logging.DEBUG)
    else:
        logging.basicConfig(level=logging.INFO)

    loop = asyncio.get_event_loop()
    if args.debug:
        loop.set_debug(True)
        loop.slow_callback_duration = 0.001
        warnings.simplefilter('always', ResourceWarning)

    with open(args.config, "r") as f:
        config = read_config(f)

    set_user_agent(config.user_agent)

    bucket_rate_limiter: RateLimiter
    if args.slowstart:
        bucket_rate_limiter = SlowStartRateLimiter(args.max_mps_per_bucket)
    elif args.max_mps_per_bucket > 0:
        bucket_rate_limiter = FixedRateLimiter(args.max_mps_per_bucket)
    else:
        bucket_rate_limiter = NonRateLimiter()

    if args.no_auto_publish and args.once:
        sys.stderr.write("--no-auto-publish and --once are mutually exclude.")
        sys.exit(1)

    forge_rate_limiter: Dict[Forge, datetime] = {}

    vcs_managers = get_vcs_managers_from_config(config)
    db = await state.create_pool(config.database_location)
    async with AsyncExitStack() as stack:
        redis = Redis.from_url(config.redis_location)
        stack.push_async_callback(redis.close)

        lock_manager = aioredlock.Aioredlock([config.redis_location])
        stack.push_async_callback(lock_manager.destroy)

        publish_worker = PublishWorker(
            template_env_path=args.template_env_path,
            external_url=args.external_url,
            differ_url=args.differ_url,
            lock_manager=lock_manager,
            redis=redis)

        if args.once:
            await publish_pending_ready(
                db=db,
                redis=redis,
                config=config,
                publish_worker=publish_worker,
                bucket_rate_limiter=bucket_rate_limiter,
                dry_run=args.dry_run,
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
                        db=db,
                        redis=redis,
                        config=config,
                        publish_worker=publish_worker,
                        bucket_rate_limiter=bucket_rate_limiter,
                        forge_rate_limiter=forge_rate_limiter,
                        dry_run=args.dry_run,
                        vcs_managers=vcs_managers,
                        interval=args.interval,
                        auto_publish=not args.no_auto_publish,
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
                        publish_worker=publish_worker,
                        bucket_rate_limiter=bucket_rate_limiter,
                        forge_rate_limiter=forge_rate_limiter,
                        vcs_managers=vcs_managers,
                        db=db, redis=redis, config=config,
                        dry_run=args.dry_run,
                        require_binary_diff=args.require_binary_diff,
                        modify_mp_limit=args.modify_mp_limit,
                        push_limit=args.push_limit,
                    )
                ),
                loop.create_task(
                    refresh_bucket_mp_counts(db, bucket_rate_limiter),
                ),
            ]
            if not args.reviewed_only and not args.no_auto_publish:
                tasks.append(
                    loop.create_task(
                        listen_to_runner(
                            db=db,
                            redis=redis,
                            config=config,
                            publish_worker=publish_worker,
                            bucket_rate_limiter=bucket_rate_limiter,
                            vcs_managers=vcs_managers,
                            dry_run=args.dry_run,
                            require_binary_diff=args.require_binary_diff,
                        )
                    )
                )
            await asyncio.gather(*tasks)


if __name__ == "__main__":
    sys.exit(asyncio.run(main(sys.argv)))
