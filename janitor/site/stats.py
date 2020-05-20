
from . import env


async def write_maintainer_stats(conn):
    template = env.get_template('maintainer-stats.html')

    by_maintainer = {}

    for maintainer_email, status, count in await conn.fetch("""
select maintainer_email, status, count(*) from merge_proposal
left join package on package.name = merge_proposal.package
group by maintainer_email, status
"""):
        by_maintainer.setdefault(maintainer_email, {})[status] = count
    return await template.render_async(by_maintainer=by_maintainer)


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

    review_status_stats = {
        status: count for (status, count) in await conn.fetch(
            'select review_status, count(*) from '
            'last_unabsorbed_runs where result_code = \'success\' group by 1')}

    pushes_over_time = {
        timestamp: int(count) for (timestamp, count) in await conn.fetch(
            'SELECT timestamp, '
            'sum(count(*)) over (order by timestamp asc rows '
            'between unbounded preceding and current row) FROM publish '
            'WHERE mode = \'push\' and result_code = \'success\' '
            'group by 1 order by timestamp')}

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
""") if ndays is not None and ndays > 0]
    time_to_merge.sort()

    return await template.render_async(
        by_hoster=by_hoster,
        by_status_chart=by_status,
        review_status_stats=review_status_stats,
        pushes_over_time=pushes_over_time,
        merges_over_time=merges_over_time,
        time_to_merge=time_to_merge)
