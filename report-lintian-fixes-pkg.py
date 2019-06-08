#!/usr/bin/python3

import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402

parser = argparse.ArgumentParser(prog='report-lintian-fixes-pkg')
parser.add_argument("directory")
args = parser.parse_args()
dir = os.path.abspath(args.directory)

if not os.path.exists(dir):
    os.mkdir(dir)

with open(os.path.join(dir, 'index.rst'), 'w') as indexf:
    indexf.write("""\
Package Index
=============

""")

    for (name, command, result_code, log_id, description,
         duration) in state.iter_last_runs(command='lintian-brush'):
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
