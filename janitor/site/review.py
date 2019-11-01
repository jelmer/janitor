#!/usr/bin/python3

from aiohttp import ClientConnectorError
import urllib.parse

from janitor import state
from janitor.site import (
    env,
    highlight_diff,
    )


async def generate_review(conn, client, publisher_url, suite=None):
    async for (package_name, command, build_version, result_code, context,
               start_time, run_id, revision, result, branch_name, suite,
               maintainer_email, uploader_emails, branch_url,
               main_branch_revision, review_status
               ) in state.iter_publish_ready(
                       conn, review_status=['unreviewed'], limit=1,
                       suite=suite):
        break

    async def show_diff():
        if not revision or revision == main_branch_revision:
            return ''
        url = urllib.parse.urljoin(publisher_url, 'diff/%s' % run_id)
        try:
            async with client.get(url) as resp:
                if resp.status == 200:
                    return (await resp.read()).decode('utf-8', 'replace')
                else:
                    return (
                        'Unable to retrieve diff; error %d' % resp.status)
        except ClientConnectorError as e:
            return 'Unable to retrieve diff; error %s' % e
    kwargs = {
        'show_diff': show_diff,
        'package_name': package_name,
        'highlight_diff': highlight_diff,
        'run_id': run_id,
        'suite': suite,
        }
    template = env.get_template('review.html')
    return await template.render_async(**kwargs)
