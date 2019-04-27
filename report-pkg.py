#!/usr/bin/python3

import argparse
from debian.deb822 import Changes
import os
import sys
import time
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402
from janitor.build import (
    changes_filename,
    get_build_architecture,
)  # noqa: E402
from janitor.trace import warning  # noqa: E402

FAIL_BUILD_LOG_TAIL = 15

parser = argparse.ArgumentParser(prog='report-pkg')
parser.add_argument("directory")
args = parser.parse_args()
dir = args.directory


def changes_get_binaries(changes_path):
    with open(changes_path, "r") as cf:
        changes = Changes(cf)
        return changes['Binary'].split(' ')


def include_console_log(f, log_path, lines=None):
    f.write('.. literalinclude:: %s\n' % os.path.basename(log_path))
    f.write('  :language: console\n')
    f.write('  :linenos:\n')
    if lines:
        f.write('  :lines: ')
        if lines[0] is not None:
            f.write('%d' % lines[0])
        f.write('-')
        if lines[1] is not None:
            f.write('%d' % lines[1])
        f.write('\n')
    f.write('\n')


def include_console_log_tail(f, log_path, tail):
    with open(log_path, 'r') as logf:
        linecount = logf.read().count('\n')
    if linecount > tail:
        include_console_log(f, log_path, lines=(linecount-tail, None))
    else:
        include_console_log(f, log_path)


def include_build_log_failure(f, log_path, length):
    build_end = None
    linecount = 0
    with open(log_path, 'r') as logf:
        for i, l in enumerate(logf, 1):
            if l.startswith('Build finished at '):
                build_end = i
            if l.startswith('Fail-Stage: '):
                include_console_log(f, log_path, (build_end-length, build_end))
                return
            linecount += 1

    include_console_log_tail(f, log_path, (linecount-length, None))


if not os.path.isdir(dir):
    os.mkdir(dir)


runs_by_pkg = {}

for run in state.iter_runs():
    (run_id, (start_time, finish_time), command, description,
        package_name, merge_proposal_url, build_version,
        build_distro, result_code) = run

    runs_by_pkg.setdefault(package_name, []).append(run)

    run_dir = os.path.join(dir, package_name, run_id)
    os.makedirs(run_dir, exist_ok=True)

    kind = command.split(' ')[0]
    with open(os.path.join(run_dir, 'index.rst'), 'w') as g:
        g.write('Run of %s for %s\n' % (kind, package_name))
        g.write('============' + (len(kind) + len(package_name)) * '=' + '\n')

        g.write('* Package: `%s <..>`_\n' % package_name)
        g.write('* Start time: %s\n' % start_time)
        g.write('* Finish time: %s\n' % finish_time)
        g.write('* Run time: %s\n' % (finish_time - start_time))
        g.write('* Description: %s\n' % description)
        if build_version:
            changes_name = changes_filename(
                package_name, build_version,
                get_build_architecture())
            g.write('* Changes filename: `%s '
                    '<../../../%s/%s>`_\n'
                    % (changes_name, build_distro, changes_name))
        g.write('\n')
        g.write('Try this locally::\n\n\t')
        # TODO(jelmer): Don't put lintian-fixer specific code here
        svp_args = command.split(' ')
        if svp_args[0] == 'lintian-brush':
            g.write('debian-svp lintian-brush %s %s' % (
                package_name, ' '.join(
                    ['--fixers=%s' % f for f in svp_args[1:]])))
        elif svp_args[0] == 'new-upstream':
            g.write('debian-svp new-upstream')
            g.write(' '.join(svp_args[1:]))
            g.write(' ' + package_name)
        else:
            raise AssertionError
        g.write('\n\n')
        build_log_path = 'build.log'
        worker_log_path = 'worker.log'
        if build_version:
            changes_name = changes_filename(
                package_name, build_version,
                get_build_architecture())
            changes_path = os.path.join(
                "../public_html", build_distro, changes_name)
            if not os.path.exists(changes_path):
                warning('Missing changes path %r', changes_path)
            else:
                g.write('Install this package (if you have the ')
                g.write('`apt repository <../../../>`_ enabled) '
                        'by running one of::\n\n')
                for binary in changes_get_binaries(changes_path):
                    g.write(
                        '\tapt install -t upstream-releases %s\n' %
                        binary)
                    g.write('\tapt install %s=%s\n' % (
                            binary, build_version))
            g.write('\n\n')
        elif os.path.exists(os.path.join(run_dir, build_log_path)):
            include_build_log_failure(
                g, os.path.join(run_dir, build_log_path),
                FAIL_BUILD_LOG_LEN)
        else:
            include_console_log_tail(
                g, os.path.join(run_dir, worker_log_path))
        if os.path.exists(os.path.join(run_dir, build_log_path)):
            g.write('`Full build log <%s>`_\n' %
                    build_log_path)
        elif os.path.exists(
                os.path.join(run_dir, worker_log_path)):
            g.write('`Full worker log <%s>`_\n' %
                    worker_log_path)
        g.write("\n")
        g.write("*Last Updated: " + time.asctime() + "*\n")


merge_proposals = {}
for package, url, status in state.iter_proposals():
    merge_proposals.setdefault(package, []).append((url, status))


with open(os.path.join(dir, 'index.rst'), 'w') as indexf:
    indexf.write("""\
Package Index
=============

""")

    for (name, maintainer_email, branch_url) in state.iter_packages():
        indexf.write(
            '- `%s <%s>`_\n' % (name, name))

        pkg_dir = os.path.join(dir, name)
        if not os.path.isdir(pkg_dir):
            os.mkdir(pkg_dir)

        with open(os.path.join(pkg_dir, 'index.rst'), 'w') as f:
            f.write('%s\n' % name)
            f.write('%s\n' % ('=' * len(name)))
            f.write(
                '* `QA Page <https://tracker.debian.org/pkg/%s>`_\n' % name)
            f.write('* Maintainer email: %s\n' % maintainer_email)
            f.write('* Branch URL: `%s <%s>`_\n' % (branch_url, branch_url))
            f.write('\n')

            f.write('Recent merge proposals\n')
            f.write('----------------------\n')
            for merge_proposal_url, status in merge_proposals.get(name, []):
                f.write('* `merge proposal <%s>`_\n' % merge_proposal_url)
            f.write('\n')

            f.write('Recent package builds\n')
            f.write('---------------------\n')
            runs = list(state.iter_runs(name))
            for (run_id, (start_time, finish_time), command, description,
                    package_name, merge_proposal_url, build_version,
                    build_distro, result_code) in runs_by_pkg.get(name, []):
                if build_version is None:
                    continue
                f.write('* %s (for %s)' % (build_version, build_distro))
                if result_code:
                    f.write(' => %s' % result_code)
                f.write('\n')
            f.write('\n')

            f.write('Recent runs\n')
            f.write('-----------\n')

            for (run_id, (start_time, finish_time), command, description,
                    package_name, merge_proposal_url, build_version,
                    build_distro, result_code) in runs_by_pkg.get(name, []):
                kind = command.split(' ')[0]
                f.write('* `%s: %s <%s/>`_' % (
                    finish_time.isoformat(timespec='minutes'), kind, run_id))
                if merge_proposal_url:
                    f.write(' (`merge proposal <%s>`_)' % merge_proposal_url)
                if result_code:
                    f.write(' => %s' % result_code)
                f.write('\n')

    indexf.write("\n")
    indexf.write("*Last Updated: " + time.asctime() + "*\n")
