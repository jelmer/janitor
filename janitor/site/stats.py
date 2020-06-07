#!/usr/bin/python3

from . import env
from aiohttp import web


async def write_maintainer_stats(conn):
    template = env.get_template('maintainer-stats.html')

    by_maintainer = {}

    for maintainer_email, status, count in await conn.fetch("""
select maintainer_email, status, count(*) from merge_proposal
left join package on package.name = merge_proposal.package
group by maintainer_email, status
order by maintainer_email asc
"""):
        by_maintainer.setdefault(maintainer_email, {})[status] = count
    return await template.render_async(by_maintainer=by_maintainer)


async def graph_pushes_over_time(conn):
    labels = []
    counts = []
    for (timestamp, count) in await conn.fetch(
            'SELECT timestamp, '
            'sum(count(*)) over (order by timestamp asc rows '
            'between unbounded preceding and current row) FROM publish '
            'WHERE mode = \'push\' and result_code = \'success\' '
            'group by 1 order by timestamp'):
        labels.append(timestamp.isoformat())
        counts.append(count)
    return {
        'labels': labels,
        'push_count': counts}


async def handle_graph_pushes_over_time(request):
    async with request.app.database.acquire() as conn:
        return web.json_response(
            await graph_pushes_over_time(conn),
            headers={'Cache-Control': 'max-age=60'})


async def write_stats(conn):
    template = env.get_template('stats.html')

    by_status = {}
    by_hoster = {}
    for hoster, status, count in await conn.fetch("""
SELECT
    REGEXP_REPLACE(url, '^(https?://)([^/]+)/.*', '\\2'),
    status,
    count(*)
FROM merge_proposal group by 1, 2"""):
        by_hoster.setdefault(hoster, {})[status] = count
        by_status.setdefault(status, {})[hoster] = count

    merges_over_time = {
        'opened': {timestamp: int(count)
                   for (timestamp, count) in await conn.fetch("""
select
  timestamp,
  sum(count(*)) over (order by timestamp asc rows
                      between unbounded preceding and current row) as open
from
    (select distinct on (merge_proposal_url) timestamp from
     publish where mode = 'propose' and result_code = 'success'
     group by merge_proposal_url, timestamp
     order by merge_proposal_url, timestamp)
as i group by 1""")},
        'merged': {timestamp: int(count)
                   for (timestamp, count) in await conn.fetch("""
select merged_at, sum(count(*)) over (
    order by merged_at asc rows between unbounded preceding and current row)
as merged from merge_proposal
where status = 'merged' and merged_at is not null group by 1""")}
        }

    time_to_merge = [
            (ndays, count) for (ndays, count) in await conn.fetch("""
select extract(day from merged_at - timestamp) ndays, count(*)
from merge_proposal
left join publish on publish.merge_proposal_url = merge_proposal.url and
status = 'merged' and merged_at is not null group by 1
order by 1
""") if ndays is not None and ndays > 0]

    total_candidates = await conn.fetchval(
        """select count(*) from perpetual_candidates""")

    burndown = await conn.fetch("""
select start_time, c from (select row_number() over() as rn,
start_time,
$1 - row_number() over (order by start_time asc) as c
from first_run_time) as r where mod(rn, 200) = 0
""", total_candidates)

    return await template.render_async(
        burndown=burndown,
        by_hoster=by_hoster,
        by_status_chart=by_status,
        merges_over_time=merges_over_time,
        time_to_merge=time_to_merge)


async def handle_cupboard_stats(request):
    async with request.app.database.acquire() as conn:
        return web.Response(
            content_type='text/html', text=await write_stats(
                conn), headers={'Cache-Control': 'max-age=60'})


async def graph_review_status(conn):
    return {
        status: count for (status, count) in await conn.fetch("""\
select review_status, count(*) from last_unabsorbed_runs
LEFT JOIN publish_policy
ON publish_policy.package = last_unabsorbed_runs.package
AND publish_policy.suite = last_unabsorbed_runs.suite
where result_code = \'success\' AND
publish_policy.mode in ('propose', 'attempt-push', 'push-derived', 'push')
group by 1""")}


async def handle_cupboard_stats_graph_review_status(request):
    async with request.app.database.acquire() as conn:
        return web.json_response(
            await graph_review_status(conn),
            headers={'Cache-Control': 'max-age=60'})


def stats_app(database):
    app = web.Application()
    app.database = database
    app.router.add_get(
        '/', handle_stats, name='index')
    app.router.add_get(
        '/+chart/review-status', handle_graph_review_status,
        'graph-review-status')
    app.router.add_get(
        '/+chart/pushes-over-time', handle_graph_pushes_over_time,
        'graph-pushes-over-time')
    return app
