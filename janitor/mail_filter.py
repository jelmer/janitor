#!/usr/bin/python3

# Copyright (C) 2019-2020 Jelmer Vernooij <jelmer@jelmer.uk>
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

"""E-mail based merge proposal status refresher.

This script parses e-mails on stdin and triggers a poll of the status
of any merge request mentioned in the body.
"""


import sys
from typing import Optional, cast

import uvloop

from ._mail_filter import parse_plain_text_body, parse_html_body


def parse_email(f):
    from email import policy
    from email.message import EmailMessage, MIMEPart
    from email.parser import BytesParser

    msg = cast(EmailMessage, BytesParser(policy=policy.default).parse(f))
    html_body = cast(Optional[MIMEPart], msg.get_body(preferencelist=('html', )))
    if html_body:
        ret = parse_html_body(html_body.get_content())
        if ret:
            return ret

    text_body = cast(Optional[MIMEPart], msg.get_body(preferencelist=('plain', )))

    assert text_body
    return parse_plain_text_body(text_body.get_content())


async def refresh_merge_proposal(api_url, merge_proposal_url):
    import aiohttp
    data = {'url': merge_proposal_url}
    async with aiohttp.ClientSession() as session:
        async with session.post(api_url, data=data) as resp:
            if resp.status not in (200, 202):
                raise Exception("error %d triggering refresh for %s" % (
                    resp.status, api_url))


async def main(argv):
    import argparse
    import logging
    parser = argparse.ArgumentParser()
    parser.add_argument(
        '--refresh-url', type=str,
        help='URL to submit requests to.',
        default='https://janitor.debian.net/api/refresh-proposal-status')
    parser.add_argument(
        '--input', type=argparse.FileType('rb'),
        default='/dev/stdin',
        help='Path to read mail from.')
    args = parser.parse_args()
    logging.basicConfig()
    merge_proposal_url = parse_email(args.input)
    if merge_proposal_url is None:
        sys.exit(0)
    logging.info('Found merge proposal URL: %s', merge_proposal_url)
    await refresh_merge_proposal(args.refresh_url, merge_proposal_url)
    return 0


if __name__ == '__main__':
    import asyncio
    asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())
    sys.exit(asyncio.run(main(sys.argv[1:])))
