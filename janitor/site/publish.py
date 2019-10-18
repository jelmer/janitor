#!/usr/bin/python3

from janitor import state
from janitor.site import env


async def write_history(conn, limit=None):
    template = env.get_template('publish-history.html')
    return await template.render_async(
        count=limit,
        history=state.iter_publish_history(conn, limit=limit))


async def write_publish(package, branch_name, main_branch_revision, revision,
                        mode, merge_proposal_url, result_code, description):
    # For now..
    return result_code
