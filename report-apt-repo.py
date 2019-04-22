#!/usr/bin/python3

import argparse
from debian.changelog import Version
import os
import sys
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402

parser = argparse.ArgumentParser(prog='report-apt-repo')
parser.add_argument("suite")
args = parser.parse_args()

present = {}

for source, version in state.iter_published_packages(args.suite):
    present[source] = version

unstable = {}
for package in state.get_source_packages(
        packages=set(present), release='sid'):
    unstable[package.name] = package.version

for source in sorted(present):
    sys.stdout.write(
        '* %s %s' %
        source, Version(present[source]).upstream_version)
    if source in unstable:
        sys.stdout.write(' (%s in unstable)' % (unstable[source], ))
    sys.stdout.write('\n')
