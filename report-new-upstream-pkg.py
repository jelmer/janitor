#!/usr/bin/python3

import argparse
import asyncio
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402

parser = argparse.ArgumentParser(prog='report-new-upstream-pkg')
parser.add_argument('--snapshot', action='store_true', help='Snapshot')
parser.add_argument("directory")
args = parser.parse_args()
if args.snapshot:
    command = 'new-upstream --snapshot'
else:
    command = 'new-upstream'
dir = os.path.abspath(args.directory)

if not os.path.exists(dir):
    os.mkdir(dir)

loop = asyncio.get_event_loop()

with open(os.path.join(dir, 'index.rst'), 'w') as indexf:
    indexf.write("""\
Package Index
=============

""")

    for (name, command, result_code, log_id, description,
         duration) in loop.run_until_complete(
                 state.iter_last_runs(command=command)):
        indexf.write(
            '- `%s <%s>`_\n' % (name, name))

        pkg_dir = os.path.join(dir, name)
        if not os.path.exists(pkg_dir):
            os.mkdir(pkg_dir)

        with open(os.path.join(pkg_dir, 'index.rst'), 'w') as f:
            f.write('%s\n' % name)
            f.write('=' * len(name) + '\n')
            f.write('`Original build </pkg/%s/%s/>`_\n' %
                    (name, log_id))
