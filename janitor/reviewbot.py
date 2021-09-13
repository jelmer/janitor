#!/usr/bin/python3
# Copyright (C) 2021 Jelmer Vernooij <jelmer@jelmer.uk>
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

from aiohttp import ClientSession, ClientResponseError


async def fetch_diffs(session, base_url, run_id, branches):
    diff = {}
    for role in branches:
        async with session.get('%s/api/run/%s/diff?role=%s' % (base_url, run_id, role)) as resp:
            diff[role] = await resp.read()
    return diff


async def review_unreviewed(session, base_url, reviewer, do_review):
    async with session.get('%s/api/needs-review' % base_url, params={'reviewer': reviewer}, raise_for_status=True) as resp:
        for entry in await resp.json():
            package = entry['package']
            run_id = entry['id']
            branches = entry['branches']
            diff = await fetch_diffs(session, base_url, run_id, branches)
            try:
                async with session.get('%s/api/run/%s/debdiff' % (base_url, run_id)) as resp:
                    debdiff = await resp.read()
            except ClientResponseError as e:
                if e.status == 404:
                    debdiff = None
                else:
                    raise
            status, comment = do_review(package, run_id, diff, debdiff)
            print('%s => %s,%s' % (run_id, status, comment))
            continue
            await session.post(
                '%s/api/run/%s' % (base_url, run_id),
                data={'review-status': status, 'review-comment': comment})
