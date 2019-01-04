#!/usr/bin/python3

import sys
from breezy.plugins.propose.propose import hosters

open_proposals = []
merged_proposals = []
closed_proposals = []

for name, hoster_cls in hosters.items():
    for instance in hoster_cls.iter_instances():
        open_proposals.extend(instance.iter_my_proposals(status='open'))
        merged_proposals.extend(instance.iter_my_proposals(status='merged'))
        closed_proposals.extend(instance.iter_my_proposals(status='closed'))


def write_report(f, open_proposal, merged_proposals, closed_proposals):
    if open_proposals:
        f.write("""\
Open Proposals
==============

These proposals are currently waiting for review.

""")

    for mp in open_proposals:
        f.write('- %s\n' % mp.url)

    if merged_proposals:
        f.write("""

Merged Proposals
================

These proposals have been merged in the past.

""")

    for mp in merged_proposals:
        f.write('- %s\n' % mp.url)

    if closed_proposals:
        f.write("""

Closed Proposals
================

Proposals can be closed without being merged for a number of reasons - a
similar change has already been applied, the change was rejected or the change
was merged without history being referenced (i.e. in the case of a
cherry-pick.

""")

    for mp in closed_proposals:
        f.write('- %s\n' % mp.url)


write_report(sys.stdout, open_proposals, merged_proposals, closed_proposals)
