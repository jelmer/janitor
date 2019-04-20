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
import os
import socket

from breezy.branch import Branch
from breezy import (
    errors,
)
from breezy.trace import (
    note,
    warning,
)

from silver_platter.debian import (
    BuildFailedError,
    MissingUpstreamTarball,
    Workspace,
)
from silver_platter.debian.lintian import (
    available_lintian_fixers,
    get_fixers,
    LintianFixer,
    DEFAULT_ADDON_FIXERS,
)
from silver_platter.debian.upstream import (
    NewUpstreamMerger,
)

from silver_platter.utils import (
    run_pre_check,
    run_post_check,
    PostCheckFailed,
)

from janitor.build import (
    build,
    add_dummy_changelog_entry,
    get_latest_changelog_version,
    changes_filename,
    get_build_architecture,
)


class JanitorLintianFixer(object):
    """Janitor-specific Lintian Fixer."""

    @staticmethod
    def setup_argparser(subparser):
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


class JanitorNewUpstreamMerger(object):

    @staticmethod
    def setup_argparser(subparser):
        subparser.add_argument(
            '--snapshot',
            help='Merge a new upstream snapshot rather than a release',
            action='store_true')


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


lintian_subparser = argparse.ArgumentParser(prog='lintian-brush')
JanitorLintianFixer.setup_argparser(lintian_subparser)

new_upstream_subparser = argparse.ArgumentParser(prog='new-upstream')
JanitorNewUpstreamMerger.setup_argparser(new_upstream_subparser)


def process_package(vcs_url, env, command, output_directory,
                    build_command=None, pre_check_command=None,
                    post_check_command=None, possible_transports=None,
                    possible_hosters=None, resume_branch_url=None):
    pkg = env['PACKAGE']
    committer = env['COMMITTER']
    # TODO(jelmer): sort out this mess:
    if command[0] == 'lintian-brush':
        subargs = lintian_subparser.parse_args(command[1:])
        build_version_suffix = 'janitor+lintian'
        build_suite = 'lintian-fixes'
    elif command[0] == 'new-upstream':
        subargs = new_upstream_subparser.parse_args(command[1:])
        build_version_suffix = 'janitor+newupstream'
        build_suite = 'upstream-releases'
    else:
        raise WorkerFailure('unknown subcommand %s' % command[0])
    assert pkg is not None
    assert output_directory is not None
    output_directory = os.path.abspath(output_directory)

    note('Processing: %s', pkg)

    try:
        main_branch = Branch.open(
            vcs_url, possible_transports=possible_transports)
        if resume_branch_url:
            resume_branch = Branch.open(
                resume_branch_url,
                possible_transports=possible_transports)
        else:
            resume_branch = None
    except socket.error:
        raise WorkerFailure('ignoring, socket error')
    except errors.NotBranchError as e:
        raise WorkerFailure('Branch does not exist: %s' % e)
    except errors.UnsupportedProtocol:
        raise WorkerFailure('Branch available over unsupported protocol')
    except errors.ConnectionError as e:
        raise WorkerFailure(str(e))
    except errors.PermissionDenied as e:
        raise WorkerFailure(str(e))
    except errors.InvalidHttpResponse as e:
        raise WorkerFailure(str(e))
    except errors.TransportError as e:
        raise WorkerFailure(str(e))

    with Workspace(main_branch, resume_branch=resume_branch) as ws:
        run_pre_check(ws.local_tree, pre_check_command)

        if command[0] == 'lintian-brush':
            # TODO(jelmer): 'fixers' is wrong; it's actually tags.
            fixers = get_fixers(
                available_lintian_fixers(), tags=subargs.fixers)
            branch_changer = LintianFixer(
                pkg, fixers=fixers,
                update_changelog=subargs.update_changelog,
                compat_release=subargs.compat_release,
                propose_addon_only=subargs.propose_addon_only,
                committer=committer)
        elif command[0] == 'new-upstream':
            branch_changer = NewUpstreamMerger(
                subargs.snapshot)

        branch_changer.make_changes(ws.local_tree)

        if not ws.changes_since_main():
            raise WorkerResult('Nothing to do.')

        try:
            run_post_check(ws.local_tree, post_check_command, ws.orig_revid)
        except PostCheckFailed as e:
            note('%s: post-check failed')
            raise WorkerFailure(str(e))

        if build_command:
            add_dummy_changelog_entry(
                ws.local_tree.basedir, build_version_suffix, build_suite,
                'Build for debian-janitor apt repository.')
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
        default='sbuild -v')

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
