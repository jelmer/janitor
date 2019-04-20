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
                f.write('* `merge proposal <%s>`_\n' % merge_proposal_url)

            f.write('\n')

            f.write('Recent runs\n')
            f.write('-----------\n')

            for (run_id, (start_time, finish_time), command, description,
                    package_name, merge_proposal_url, changes_filename,
                    build_distro) in state.iter_runs(name):
                kind = command.split(' ')[0]
                f.write('* `%s: %s <%s/>`_' % (
                    finish_time.isoformat(timespec='minutes'), kind, run_id))
                if merge_proposal_url:
                    f.write(' (`merge proposal <%s>`_)' % merge_proposal_url)
                f.write('\n')

                run_dir = os.path.join(pkg_dir, run_id)
                if not os.path.isdir(run_dir):
                    os.mkdir(run_dir)

                with open(os.path.join(run_dir, 'index.rst'), 'w') as g:
                    g.write('Run of %s for %s\n' % (kind, package_name))
                    g.write('====' + len(run_id) * '=' + '\n')

                    g.write('* Package: `%s <..>`_\n' % package_name)
                    g.write('* Start time: %s\n' % start_time)
                    g.write('* Finish time: %s\n' % finish_time)
                    g.write('* Run time: %s\n' % (finish_time - start_time))
                    g.write('* Description: %s\n' % description)
                    if changes_filename:
                        g.write('* Changes filename: %s\n' % changes_filename)
                    g.write('\n')
                    g.write('Command run::\n\n\t%s\n\n' % command)
                    g.write('Try this locally::\n\n\t')
                    # TODO(jelmer): Don't put lintian-fixer specific code here
                    svp_args = command.split(' ')
                    if svp_args[0] == 'lintian-brush':
                        g.write('debian-svp lintian-brush %s %s' % (
                            name, ' '.join(
                                ['--fixers=%s' % f for f in svp_args[1:]])))
                    elif svp_args[0] == 'new-upstream':
                        g.write(('debian-svp new-upstream %s' % name)
                                + ' '.join(svp_args[1:]))
                    else:
                        raise AssertionError
                    if changes_filename:
                        g.write('Fetch the package::\n\n')
                        g.write(
                            '\tdget https://janitor.debian.net/apt/%s/%s\n' %
                            (build_distro, changes_filename))
                        g.write('\n')
                        g.write('Install this package (if you have the ')
                        g.write('`apt repository <../../apt/>`_ enabled) '
                                'by running one of::\n\n')
                        g.write('\tapt install -t upstream-releases %s\n' %
                                package_name)
                        g.write('\tapt install %s=%s\n' % (
                                package_name, changes_filename.split('_')[1]))
                        g.write('\n\n')
                    g.write('\n\n')
                    g.write('.. literalinclude:: ../logs/%s/build.log\n' %
                            run_id)
                    g.write('   :linenos:\n')
                    g.write('   :caption: build.log\n')
                    g.write('   :language: shell\n')
                    g.write("\n")
                    g.write("*Last Updated: " + time.asctime() + "*\n")

    indexf.write("\n")
    indexf.write("*Last Updated: " + time.asctime() + "*\n")
