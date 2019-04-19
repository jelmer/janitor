#!/usr/bin/python3

import sys
import time

import os
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402


def write_report(f, open_proposals, merged_proposals, closed_proposals):
    f.write("""\
Status
======

""")

    if open_proposals:
        f.write("""\
Open Proposals
--------------

These proposals are currently waiting for review.

""")

    for url in open_proposals:
        f.write('- %s\n' % url)

    if merged_proposals:
        f.write("""

Merged Proposals
----------------

These proposals have been merged in the past.

""")

    for url in merged_proposals:
        f.write('- %s\n' % url)

    if closed_proposals:
        f.write("""

Closed Proposals
----------------

Proposals can be closed without being merged for a number of reasons - a
similar change has already been applied, the change was rejected or the change
was merged without history being referenced (i.e. in the case of a
cherry-pick.

""")

    for url in closed_proposals:
        f.write('- %s\n' % url)

    print("*Last Updated: " + time.asctime() + "*")


proposals_by_status = {}


for url, status, package in state.iter_all_proposals():
    proposals_by_status.setdefault(status, []).append(url)


write_report(
    sys.stdout, proposals_by_status.get('open', []),
    proposals_by_status.get('merged', []),
    proposals_by_status.get('closed', []))
