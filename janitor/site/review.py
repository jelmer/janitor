#!/usr/bin/python3

from aiohttp import ClientConnectorError
import json
import urllib.parse

from janitor import state
from janitor.site import (
    env,
    )


async def generate_review(conn, client, publisher_url, suite=None):
    entries = [entry async for entry in
               state.iter_publish_ready(
                       conn, review_status=['unreviewed'], limit=40,
                       suite=suite)]

    (package_name, command, build_version, result_code, context,
     start_time, run_id, revision, result, branch_name, suite,
     maintainer_email, uploader_emails, branch_url,
     main_branch_revision, review_status) = entries.pop(0)

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
        'run_id': run_id,
        'suite': suite,
        'json_dumps': json.dumps,
        'todo': [(entry[0], entry[6]) for entry in entries],
        }
    template = env.get_template('review.html')
    return await template.render_async(**kwargs)
