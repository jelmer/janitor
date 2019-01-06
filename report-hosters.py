#!/usr/bin/python3

import operator

from urllib.parse import urlparse

from breezy.plugins.debian.directory import vcs_field_to_bzr_url_converters

from silver_platter.debian.udd import UDD

udd = UDD.public_udd_mirror()
cursor = udd._conn.cursor()
cursor.execute(
    "SELECT source, vcs, url FROM vcswatch GROUP by 1, 2, 3")
hosts = {}
url_converters = dict(vcs_field_to_bzr_url_converters)
for row in cursor.fetchall():
    try:
        converter = url_converters[row[1]]
    except KeyError:
        continue
    host = urlparse(row[2])[1]
    hosts.setdefault(host, 0)
    hosts[host] += 1
for host, count in sorted(hosts.items(), key=operator.itemgetter(1), reverse=True):
    print('%-40s %d' % (host, count))
