#!/usr/bin/python3

import argparse
from debian.changelog import Version
import os
import sys
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state, udd  # noqa: E402
from janitor.site.rst import format_table  # noqa: E402

parser = argparse.ArgumentParser(prog='report-apt-repo')
parser.add_argument("suite")
args = parser.parse_args()


def get_unstable_versions(present):
    unstable = {}
    if present:
        for package in udd.UDD.public_udd_mirror().get_source_packages(
                packages=list(present), release='sid'):
            unstable[package.name] = Version(package.version)
    return unstable


def gather_package_list_table():
    present = {}
    for source, version in state.iter_published_packages(args.suite):
        present[source] = Version(version)

    unstable = get_unstable_versions(present)

    header = ['Package', 'Version', 'Upstream Version in Unstable']
    data = []
    for source in sorted(present):
        data.append(
            (source, present[source].upstream_version,
             unstable[source].upstream_version
             if source in unstable else ''))

    return (header, data)


def gather_package_status_table():
    header = [
        'Package',
        'Version in Unstable',
        'New Upstream Version',
        ]
    present = {}
    for (source, command, build_version, result_code, context,
         start_time, log_id) in state.iter_last_successes(args.suite):
        present[source] = (
            build_version, result_code, context, start_time, log_id)
    unstable = get_unstable_versions(present)
    data = []
    for source in sorted(present):
        (version, result_code, context, start_time, log_id) = present[source]
        if version:
            new_field = "`%s </pkg/%s/%s>`_" % (
                Version(version).upstream_version, source, log_id)
        else:
            new_field = "`%s </pkg/%s/%s>`_" % (result_code, source, log_id)
        data.append(
            (source,
             '`%s <%s>`_' % (
                 unstable[source].upstream_version
                 if source in unstable else '',
                 'https://packages.debian.org/sid/%s' % source),
             new_field,
             ))

    return (header, data)


with open(os.path.join(args.suite, 'package-list.rst'), 'w') as f:
    (header, data) = gather_package_list_table()
    format_table(f, header, data)


with open(os.path.join(args.suite, 'package-status.rst'), 'w') as f:
    (header, data) = gather_package_status_table()
    format_table(f, header, data)


with open(os.path.join(args.suite, 'apt-stats.rst'), 'w') as f:
    i = 0
    for source, version in state.iter_published_packages(args.suite):
        i += 1
    f.write('There are *%d* packages in this repository.' % i)
