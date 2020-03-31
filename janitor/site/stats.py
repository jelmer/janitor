
from . import env
from .. import state


async def write_stats(conn):
    template = env.get_template('stats.html')

    by_hoster = {}
    for hoster, status, count in await state.get_hoster_merge_proposal_stats(
            conn):
        by_hoster.setdefault(hoster, {})[status] = count

    return await template.render_async(
        by_hoster=by_hoster)
