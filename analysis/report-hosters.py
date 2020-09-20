#!/usr/bin/python3

import asyncio
import operator
from typing import Dict
from urllib.parse import urlparse

import matplotlib.pyplot as plt

from silver_platter.debian import vcs_field_to_bzr_url_converters

from janitor.udd import UDD

renames = {
    'launchpad.net': 'launchpad',
    'code.launchpad.net': 'launchpad',
    'bazaar.launchpad.net': 'launchpad',
    'git.launchpad.net': 'launchpad',
    'anonscm.debian.org': 'alioth',
    'git.debian.org': 'alioth',
    'bzr.debian.org': 'alioth',
    'hg.debian.org': 'alioth',
    'svn.debian.org': 'alioth',
    'alioth.debian.org': 'alioth',
    'salsa.debian.org': 'salsa',
    'git.code.sf.net': 'sourceforge',
    'hg.code.sf.net': 'sourceforge',
    'svn.code.sf.net': 'sourceforge',
}

loop = asyncio.get_event_loop()
udd = loop.run_until_complete(UDD.public_udd_mirror())
all = loop.run_until_complete(udd._conn.fetch(
    "SELECT sources.source, vcs, url FROM sources "
    "LEFT JOIN vcswatch ON vcswatch.source = sources.source "
    "WHERE release = 'sid' GROUP by 1, 2, 3"))
hosters: Dict[str, int] = {}
url_converters = dict(vcs_field_to_bzr_url_converters)
for source, vcs, url in all:
    if vcs is None:
        name = 'no vcs'
    else:
        try:
            converter = url_converters[vcs]
        except KeyError:
            name = 'unknown vcs %s' % vcs
        else:
            url = converter(url)
            host = urlparse(url)[1]
            host = host.split(':')[0]
            if '@' in host:
                host = host.split('@')[1]
            host = renames.get(host, host)
            name = '%s (%s)' % (host, vcs)
    hosters.setdefault(name, 0)
    hosters[name] += 1

ordered_hosters = list(sorted(
    hosters.items(), key=operator.itemgetter(1),
    reverse=True))

with open('hosters.csv', 'w') as f:
    f.write('hoster,repo_count\n')
    for host, count in ordered_hosters:
        f.write('%s,%d\n' % (host, count))

ordered_hosters = (
    ordered_hosters[:10] + [
        ('other', sum(map(operator.itemgetter(1), ordered_hosters[5:])))])

labels = list(map(operator.itemgetter(0), ordered_hosters))
sizes = list(map(operator.itemgetter(1), ordered_hosters))

fig1, ax1 = plt.subplots()
ax1.pie(sizes, labels=None, autopct='%1.1f%%',
        pctdistance=1.2)
ax1.axis('equal')
ax1.legend(loc=3, labels=labels)

plt.savefig('hosters.png')
