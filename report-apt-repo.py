#!/usr/bin/python3

import argparse
import os
import sys
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402

parser = argparse.ArgumentParser(prog='report-apt-repo')
parser.add_argument("suite")
args = parser.parse_args()

os.makedirs(args.suite, exist_ok=True)

sys.stdout.write("""\
Package Index
=============

""")

for source, version in state.iter_published_packages(
        args.suite):
    sys.stdout.write('* %s %s\n' % (source, version))
