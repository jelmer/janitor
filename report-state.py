#!/usr/bin/python3

from breezy import urlutils
from breezy.plugins.propose.propose import hosters

print("""\
Open Proposals
==============

These proposals are currently waiting for review.
""")

for name, hoster_cls in hosters.items():
    for instance in hoster_cls.iter_instances():
        for mp in instance.iter_my_proposals(status='open'):
            print('- %s' % mp.url)


print("""

Merged Proposals
================

These proposals have been merged in the past.
""")

for name, hoster_cls in hosters.items():
    for instance in hoster_cls.iter_instances():
        for mp in instance.iter_my_proposals(status='merged'):
            print('- %s' % mp.url)


print("""

Closed Proposals
================

Proposals can be closed without being merged for a number of reasons - a
similar change has already been applied, the change was rejected or the change
was merged without history being referenced (i.e. in the case of a
cherry-pick.
""")

for name, hoster_cls in hosters.items():
    for instance in hoster_cls.iter_instances():
        for mp in instance.iter_my_proposals(status='merged'):
            print('- %s' % mp.url)
