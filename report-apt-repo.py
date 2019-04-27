#!/usr/bin/python3

import argparse
from debian.changelog import Version
import os
import sys
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state, udd  # noqa: E402

parser = argparse.ArgumentParser(prog='report-apt-repo')
parser.add_argument("suite")
args = parser.parse_args()


def format_rst_table(f, header, data):
    def separator(lengths):
        for i, length in enumerate(lengths):
            if i > 0:
                f.write(' ')
            f.write('=' * length)
        f.write('\n')
    lengths = [
        max([len(str(x[i])) for x in [header] + data])
        for i in range(len(header))]
    separator(lengths)
    for i, (column, length) in enumerate(zip(header, lengths)):
        if i > 0:
            f.write(' ')
        f.write(column + (' ' * (length - len(column))))
    f.write('\n')
    separator(lengths)
    for row in data:
        for i, (column, length) in enumerate(zip(row, lengths)):
            if i > 0:
                f.write(' ')
            f.write(str(column) + (' ' * (length - len(str(column)))))
        f.write('\n')
    separator(lengths)


present = {}

for source, version in state.iter_published_packages(args.suite):
    present[source] = Version(version)

unstable = {}
if present:
    for package in udd.UDD.public_udd_mirror().get_source_packages(
            packages=list(present), release='sid'):
        unstable[package.name] = Version(package.version)

header = ['Package', 'Version', 'Upstream Version in Unstable',
          'New Upstream Version']
data = []

for source in sorted(present):
    data.append(
        (source, present[source],
         present[source].upstream_version,
         unstable[source].upstream_version
         if source in unstable else ''))


with open(os.path.join(args.suite, 'package-list.rst'), 'w') as f:
    format_rst_table(f, header, data)
