#!/usr/bin/python3

import operator

import matplotlib.pyplot as plt

from urllib.parse import urlparse

from breezy.plugins.debian.directory import vcs_field_to_bzr_url_converters

from silver_platter.debian.udd import UDD

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

udd = UDD.public_udd_mirror()
cursor = udd._conn.cursor()
cursor.execute(
    "SELECT source, vcs, url FROM vcswatch GROUP by 1, 2, 3")
hosters = {}
url_converters = dict(vcs_field_to_bzr_url_converters)
for row in cursor.fetchall():
    try:
        converter = url_converters[row[1]]
    except KeyError:
        continue
    host = urlparse(row[2])[1]
    host = host.split(':')[0]
    if '@' in host:
        host = host.split('@')[1]
    host = renames.get(host, host)
    hosters.setdefault(host, 0)
    hosters[host] += 1

ordered_hosters = list(sorted(
    hosters.items(), key=operator.itemgetter(1),
    reverse=True))

with open('hosters.csv', 'w') as f:
    f.write('hoster,repo_count\n')
    for host, count in ordered_hosters:
        f.write('%s,%d\n' % (host, count))

ordered_hosters = (
    ordered_hosters[:5] + [
        ('other', sum(map(operator.itemgetter(1), ordered_hosters[5:])))])

labels = list(map(operator.itemgetter(0), ordered_hosters))
sizes = list(map(operator.itemgetter(1), ordered_hosters))

fig1, ax1 = plt.subplots()
ax1.pie(sizes, labels=None, autopct='%1.1f%%',
        pctdistance=1.2)
ax1.axis('equal')
ax1.legend(loc=3, labels=labels)

plt.savefig('hosters.png')
