
from . import env
from .. import state


async def write_stats(conn):
    template = env.get_template('stats.html')

    by_status = {}
    by_hoster = {}
    for hoster, status, count in await state.get_hoster_merge_proposal_stats(
            conn):
        by_hoster.setdefault(hoster, {})[status] = count
        by_status.setdefault(status, {})[hoster] = count

    by_status_chart = [
        {'name': status, 'data': data}
        for (status, data) in by_status.items()]

    return await template.render_async(
        by_hoster=by_hoster,
        by_status_chart=by_status_chart)
