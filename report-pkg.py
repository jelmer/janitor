#!/usr/bin/python3

import os
import sys
import time
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402

import argparse

parser = argparse.ArgumentParser(prog='report-pkg')
parser.add_argument("directory")
args = parser.parse_args()
dir = args.directory

if not os.isdir(dir):
    os.mkdir(dir)

with open(os.path.join(dir, 'index.rst'), 'w') as indexf:
    indexf.write("""\
Packages
========

""")


for (name, maintainer_email, branch_url) in state.iter_packages():
    indexf.write(
        '- `%s <%s>`_\n' % (name, name))

    pkg_dir = os.path.join(dir, name)
    if not os.isdir(pkg_dir)
        os.mkdir(pkg_dir)

    with open(os.path.join(pkg_dir, 'index.rst'), 'w') as f:
        f.write('Package %s `QA Page <https://tracker.debian.org/pkg/%s>`_\n' % (name, name))
        f.write('Maintainer email: %s\n' % maintainer_email)
        f.write('Branch URL: %s\n' % branch_url)
        f.write('\n')


indexf.write("\n")
indexf.write("*Last Updated: " + time.asctime() + "*\n")
