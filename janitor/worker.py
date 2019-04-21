#!/usr/bin/python3
# Copyright (C) 2018 Jelmer Vernooij <jelmer@jelmer.uk>
#
# This program is free software; you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation; either version 2 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program; if not, write to the Free Software
# Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA

import argparse
from datetime import datetime
import distro_info
import json
import os

from silver_platter.debian import (
    BuildFailedError,
    MissingUpstreamTarball,
    Workspace,
)
from silver_platter.debian.lintian import (
    available_lintian_fixers,
    get_fixers,
    run_lintian_fixers,
    has_nontrivial_changes,
    DEFAULT_ADDON_FIXERS,
)
from silver_platter.debian.upstream import (
    merge_upstream,
    UpstreamAlreadyImported,
)

from silver_platter.utils import (
    run_pre_check,
    run_post_check,
    PostCheckFailed,
    open_branch,
    BranchUnavailable,
)

from .build import (
    build,
    add_dummy_changelog_entry,
    get_latest_changelog_version,
    changes_filename,
    get_build_architecture,
)
from .trace import (
    note,
    warning,
)


class LintianBrushWorker(object):
    """Janitor-specific Lintian Fixer."""

    build_version_suffix = 'janitor+lintian'
    build_suite = 'lintian-fixes'

    def __init__(self, command, env):
        self.committer = env.get('COMMITTER')
        subparser = argparse.ArgumentParser(prog='lintian-brush')
        subparser.add_argument("fixers", nargs='*')
        subparser.add_argument(
            '--no-update-changelog', action="store_false", default=None,
            dest="update_changelog", help="do not update the changelog")
        subparser.add_argument(
            '--update-changelog', action="store_true", dest="update_changelog",
            help="force updating of the changelog", default=None)
        subparser.add_argument(
            '--compat-release', type=str, default=debian_info.stable(),
            help='Oldest Debian release to be compatible with.')
        subparser.add_argument(
            '--propose-addon-only',
            help='Fixers that should be considered add-on-only.',
            type=str, action='append', default=DEFAULT_ADDON_FIXERS)
        self.args = subparser.parse_args(command)

    def make_changes(self, local_tree):
        # TODO(jelmer): 'fixers' is wrong; it's actually tags.
        fixers = get_fixers(
            available_lintian_fixers(), tags=self.args.fixers)

        with local_tree.lock_write():
            self.applied, self.failed = run_lintian_fixers(
                    local_tree, fixers,
                    committer=self.committer,
                    update_changelog=self.args.update_changelog,
                    compat_release=self.args.compat_release)
        if self.failed:
            note('some fixers failed to run: %r', self.failed)

        if not self.applied:
            note('no fixers to apply')

    def result(self):
        return {
            'applied': [
                {'summary': summary,
                 'description': result.description,
                 'fixed_lintian_tags': result.fixed_lintian_tags,
                 'certainty': result.certainty}
                for result, summary in self.applied],
            'failed': self.failed,
            'add_on_only': not has_nontrivial_changes(
                self.applied, self.args.propose_addon_only),
        }


class NewUpstreamWorker(object):

    build_suite = 'upstream-releases'
    build_version_suffix = 'janitor+newupstream'

    def __init__(self, command, env):
        subparser = argparse.ArgumentParser(prog='new-upstream')
        subparser.add_argument(
            '--snapshot',
            help='Merge a new upstream snapshot rather than a release',
            action='store_true')
        self.args = subparser.parse_args(command)
        self.upstream_version = None

    def make_changes(self, local_tree):
        try:
            self.upstream_version = merge_upstream(
                tree=local_tree, snapshot=self.args.snapshot)
        except UpstreamAlreadyImported as e:
            note('Last upstream version %s already imported' % e.version)
            self.upstream_version = e.version

    def result(self):
        return {'upstream_version': self.upstream_version}


class WorkerResult(object):

    def __init__(self, description, build_distribution=None,
                 build_version=None, changes_filename=None):
        self.description = description
        self.build_version = build_version
        self.build_distribution = build_distribution
        self.changes_filename = changes_filename


class WorkerFailure(Exception):
    """Worker processing failed."""


debian_info = distro_info.DebianDistroInfo()


def process_package(vcs_url, env, command, output_directory,
                    build_command=None, pre_check_command=None,
                    post_check_command=None, possible_transports=None,
                    possible_hosters=None, resume_branch_url=None):
    pkg = env['PACKAGE']
    # TODO(jelmer): sort out this mess:
    if command[0] == 'lintian-brush':
        subworker = LintianBrushWorker(command[1:], env)
    elif command[0] == 'new-upstream':
        subworker = NewUpstreamWorker(command[1:], env)
    else:
        raise WorkerFailure('unknown subcommand %s' % command[0])
    build_suite = subworker.build_suite
    assert pkg is not None
    assert output_directory is not None
    output_directory = os.path.abspath(output_directory)

    note('Processing: %s', pkg)

    try:
        main_branch = open_branch(
            vcs_url, possible_transports=possible_transports)
    except BranchUnavailable as e:
        raise WorkerFailure(str(e))

    if resume_branch_url:
        try:
            resume_branch = open_branch(
                resume_branch_url,
                possible_transports=possible_transports)
        except BranchUnavailable as e:
            raise WorkerFailure(str(e))
    else:
        resume_branch = None

    with Workspace(main_branch, resume_branch=resume_branch,
                   path=os.path.join(output_directory, pkg)) as ws:
        if not ws.local_tree.has_filename('debian/control'):
            raise WorkerFailure('missing control file')

        run_pre_check(ws.local_tree, pre_check_command)

        subworker.make_changes(ws.local_tree)
        with open(os.path.join(output_directory, 'result.json'), 'w') as f:
            json.dump(subworker.result(), f)

        if not ws.changes_since_main():
            return WorkerResult('Nothing to do.')

        try:
            run_post_check(ws.local_tree, post_check_command, ws.orig_revid)
        except PostCheckFailed as e:
            note('%s: post-check failed')
            raise WorkerFailure(str(e))

        if build_command:
            add_dummy_changelog_entry(
                ws.local_tree.basedir, subworker.build_version_suffix,
                build_suite, 'Build for debian-janitor apt repository.')
            with open(os.path.join(output_directory, 'build.log'), 'w') as f:
                try:
                    build(ws.local_tree, outf=f, build_command=build_command,
                          result_dir=output_directory,
                          distribution=build_suite)
                except BuildFailedError:
                    raise WorkerFailure('build failed')
                except MissingUpstreamTarball:
                    raise WorkerFailure('unable to find upstream source')

            (cl_package, cl_version) = get_latest_changelog_version(
                ws.local_tree)
            changes_name = changes_filename(
                cl_package, cl_version, get_build_architecture())
            changes_path = os.path.join(
                output_directory, changes_name)
            if not os.path.exists(changes_path):
                warning('Expected changes path %s does not exist.',
                        changes_path)
                build_suite = None
                changes_name = None
                cl_version = None
            else:
                note('Built %s', changes_path)
        else:
            build_suite = None
            changes_name = None
            cl_version = None

        ws.defer_destroy()
        return WorkerResult(
            'Success',
            build_distribution=build_suite,
            changes_filename=changes_name,
            build_version=cl_version)


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog='janitor-worker',
        formatter_class=argparse.ArgumentDefaultsHelpFormatter)
    parser.add_argument(
        '--output-directory', type=str,
        help='Output directory', default='.')
    parser.add_argument(
        '--branch-url', type=str,
        help='URL of branch to build.')
    parser.add_argument(
        '--resume-branch-url', type=str,
        help='URL of resume branch to continue on (if any).')
    parser.add_argument(
        '--pre-check',
        help='Command to run to check whether to process package.',
        type=str)
    parser.add_argument(
        '--post-check',
        help='Command to run to check package before pushing.',
        type=str, default=None)
    parser.add_argument(
        '--build-command',
        help='Build package to verify it.', type=str,
        default='sbuild -v -d$DISTRIBUTION')

    parser.add_argument('command', nargs=argparse.REMAINDER)

    args = parser.parse_args(argv)
    if args.branch_url is None:
        parser.print_usage()
        return 1
    start_time = datetime.now()
    try:
        result = process_package(
            args.branch_url, os.environ,
            args.command, output_directory=args.output_directory,
            build_command=args.build_command, pre_check_command=args.pre_check,
            post_check_command=args.post_check,
            resume_branch_url=args.resume_branch_url)
    except WorkerFailure as e:
        note('Elapsed time: %s', datetime.now() - start_time)
        note('Worker failed: %s', e)
        return 1
    else:
        note('Elapsed time: %s', datetime.now() - start_time)
        note('%s', result.description)
        if result.changes_filename is not None:
            note('Built %s.', result.changes_filename)
        return 0


if __name__ == '__main__':
    import sys
    sys.exit(main())
