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

if not os.path.isdir(dir):
    os.mkdir(dir)

with open(os.path.join(dir, 'index.rst'), 'w') as indexf:
    indexf.write("""\
Packages
========

""")


    for (pkg_id, name, maintainer_email, branch_url) in state.iter_packages():
        indexf.write(
            '- `%s <%s>`_\n' % (name, name))

        pkg_dir = os.path.join(dir, name)
        if not os.path.isdir(pkg_dir):
            os.mkdir(pkg_dir)

        with open(os.path.join(pkg_dir, 'index.rst'), 'w') as f:
            f.write('%s\n' % name)
            f.write('%s\n' % ('=' * len(name)))
            f.write('* `QA Page <https://tracker.debian.org/pkg/%s>`_\n' % name)
            f.write('* Maintainer email: %s\n' % maintainer_email)
            f.write('* Branch URL: `%s <%s>`_\n' % (branch_url, branch_url))
            f.write('\n')

            f.write('Recent merge proposals\n')
            f.write('----------------------\n')
            for merge_proposal_url in state.iter_proposals(name):
                f.write(' * `merge proposal <%s>`_\n' % merge_proposal_url)

            f.write('\n')

            f.write('Recent runs\n')
            f.write('-----------\n')

            for run_id, (start_time, finish_time), command, description, package_name, merge_proposal_url in state.iter_runs(name):
                f.write('* `%s <%s/>`_' % (command, run_id))
                if merge_proposal_url:
                    f.write(' (`merge proposal <%s>`_)' % merge_proposal_url)
                f.write('\n')

                run_dir = os.path.join(pkg_dir, run_id)
                if not os.path.isdir(run_dir):
                    os.mkdir(run_dir)

                with open(os.path.join(run_dir, 'index.rst'), 'w') as g:
                    g.write('Run %s\n' % run_id)
                    g.write('====' + len(run_id) * '=' + '\n')

                    g.write('* Package: `%s <..>`_\n' % package_name)
                    g.write('* Start time: %s\n' % start_time)
                    g.write('* Finish time: %s\n' % finish_time)
                    g.write('* Run time: %s\n' % (finish_time - start_time))
                    g.write('* Command run::\n\n    %s\n' % command)
                    g.write('* Try this locally::\n\n')
                    # TODO(jelmer): Don't put lintian-fixer specific code here
                    svp_args = command.split(' ')
                    assert svp_args[0] == 'lintian-brush'
                    g.write('    debian-svp lintian-brush %s %s\n' % (
                        name,
                        ' '.join(['--fixers=%s' % f for f in svp_args[1:]])))
                    g.write('\n')
                    g.write('%s\n' % description)
                    g.write('\n')
                    g.write('.. literalinclude:: ../logs/%s/build.log\n' % run_id)
                    g.write('   :linenos:\n')
                    g.write('   :language: shell\n')
                    g.write("\n")
                    g.write("*Last Updated: " + time.asctime() + "*\n")

    indexf.write("\n")
    indexf.write("*Last Updated: " + time.asctime() + "*\n")
