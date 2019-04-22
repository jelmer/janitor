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

present = {}

for source, version in state.iter_published_packages(args.suite):
    present[source] = Version(version)

unstable = {}
if present:
    for package in udd.UDD.public_udd_mirror().get_source_packages(
            packages=list(present), release='sid'):
        unstable[package.name] = Version(package.version)

for source in sorted(present):
    sys.stdout.write(
        '* %s %s' %
        (source, present[source].upstream_version))
    if source in unstable:
        sys.stdout.write(' (%s in unstable)' % (
            unstable[source].upstream_version, ))
    sys.stdout.write('\n')
