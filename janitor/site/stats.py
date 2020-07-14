#!/usr/bin/python3

from . import env, json_chart_data
from aiohttp import web
from .. import state


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


async def write_maintainer_overview(conn, maintainer):
    template = env.get_template('maintainer-overview.html')
    packages = [p for p, removed in
                await state.iter_packages_by_maintainer(
                    conn, maintainer)
                if not removed]
    proposals = []
    for package, url, status in await state.iter_proposals(conn, packages):
        proposals.append((package, url, status))
    candidates = []
    for row in await state.iter_candidates(conn, packages=packages):
        candidates.append(row)
    runs = []
    async for run in state.iter_last_unabsorbed_runs(
            conn, packages=packages):
        runs.append(run)

    return await template.render_async(
        packages=packages, runs=runs,
        candidates=candidates, maintainer=maintainer,
        proposals=proposals)


@json_chart_data(max_age=60)
async def handle_graph_pushes_over_time(request, conn):
    labels = []
    counts = []
    for (timestamp, count) in await conn.fetch(
            'SELECT timestamp, '
            'sum(count(*)) over (order by timestamp asc rows '
            'between unbounded preceding and current row) FROM publish '
            'WHERE mode = \'push\' and result_code = \'success\' '
            'group by 1 order by timestamp'):
        labels.append(timestamp.isoformat())
        counts.append(int(count))
    return {
        'labels': labels,
        'push_count': counts}


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

    return await template.render_async(
        by_hoster=by_hoster,
        by_status_chart=by_status)


async def handle_stats(request):
    async with request.app.database.acquire() as conn:
        return web.Response(
            content_type='text/html', text=await write_stats(
                conn), headers={'Cache-Control': 'max-age=60'})


@json_chart_data(max_age=60)
async def handle_graph_merges_over_time(request, conn):
    return {
        'opened': {timestamp.isoformat(): int(count)
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
        'merged': {timestamp.isoformat(): int(count)
                   for (timestamp, count) in await conn.fetch("""
select merged_at, sum(count(*)) over (
    order by merged_at asc rows between unbounded preceding and current row)
as merged from merge_proposal
where status = 'merged' and merged_at is not null group by 1""")}
        }


@json_chart_data(max_age=60)
async def handle_graph_review_status(request, conn):
    return {
        status: count for (status, count) in await conn.fetch("""\
select review_status, count(*) from last_unabsorbed_runs
LEFT JOIN publish_policy
ON publish_policy.package = last_unabsorbed_runs.package
AND publish_policy.suite = last_unabsorbed_runs.suite
where result_code = \'success\' AND
publish_policy.mode in ('propose', 'attempt-push', 'push-derived', 'push')
group by 1""")}


@json_chart_data(max_age=60)
async def handle_graph_time_to_merge(request, conn):
    return {
            ndays: count for (ndays, count) in await conn.fetch("""
select extract(day from merged_at - timestamp) ndays, count(*)
from merge_proposal
left join publish on publish.merge_proposal_url = merge_proposal.url and
status = 'merged' and merged_at is not null group by 1
order by 1
""") if ndays is not None and ndays > 0}


@json_chart_data(max_age=60)
async def handle_graph_burndown(request, conn):
    suite = request.query.get('suite')
    query = "select count(*) from perpetual_candidates"
    args = []
    if suite is not None:
        query += " WHERE suite = $1"
        args.append(suite)
    total_candidates = await conn.fetchval(query, *args)

    args = [total_candidates]
    if suite is not None:
        additional = " WHERE suite = $2"
        args.append(suite)
    else:
        additional = ""

    query = """
select start_time, c from (select row_number() over() as rn,
start_time,
$1 - row_number() over (order by start_time asc) as c
from first_run_time%s) as r where mod(rn, 200) = 0
""" % additional

    return [
        (start_time.isoformat(), int(c))
        for (start_time, c) in await conn.fetch(query, *args)]


def stats_app(database):
    app = web.Application()
    app.database = database
    app.router.add_get(
        '/', handle_stats, name='index')
    app.router.add_get(
        '/+chart/review-status', handle_graph_review_status,
        name='graph-review-status')
    app.router.add_get(
        '/+chart/pushes-over-time', handle_graph_pushes_over_time,
        name='graph-pushes-over-time')
    app.router.add_get(
        '/+chart/merges-over-time', handle_graph_merges_over_time,
        name='graph-merges-over-time')
    app.router.add_get(
        '/+chart/time-to-merge', handle_graph_time_to_merge,
        name='graph-time-to-merge')
    app.router.add_get(
        '/+chart/burndown', handle_graph_burndown,
        name='graph-burndown')
    return app
