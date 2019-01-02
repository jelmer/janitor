#!/usr/bin/python3

from breezy import urlutils
from breezy.plugins.propose.propose import hosters

print("Open Merge Proposals")
print("====================")

for name, hoster_cls in hosters.items():
    for instance in hoster_cls.iter_instances():
        for mp in instance.iter_my_proposals():
            print('- %s' % mp.url)
