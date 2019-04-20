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
import subprocess
import uuid

from breezy.branch import Branch
from breezy import (
    errors,
)
from breezy.trace import (
    note,
    warning,
)
from breezy.plugins.propose.propose import (
    NoSuchProject,
    UnsupportedHoster,
)


from silver_platter.debian import (
    propose_or_push,
    BuildFailedError,
    MissingUpstreamTarball,
)
from silver_platter.debian.lintian import (
    available_lintian_fixers,
    PostCheckFailed,
    LintianFixer,
    create_mp_description,
    get_fixers,
    parse_mp_description,
    DEFAULT_ADDON_FIXERS,
)
from silver_platter.debian.upstream import (
    NewUpstreamMerger,
)

from janitor.build import (
    build,
    predict_changes_filename,
    add_dummy_changelog_entry,
)  # noqa: E402


JANITOR_BLURB = """
This merge proposal was created automatically by the Janitor bot
(https://janitor.debian.net/).

You can follow up to this merge proposal as you normally would.
"""


LOG_BLURB = """
Build and test logs for this branch can be found at
https://janitor.debian.net/pkg/%(package)s/%(log_id)s/.
"""


def strip_janitor_blurb(text):
    return text[text.index(JANITOR_BLURB):]


def add_janitor_blurb(text, env):
    text += JANITOR_BLURB
    if env['log_id']:
        text += (LOG_BLURB % env)
    return text


class JanitorLintianFixer(LintianFixer):
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

    def __init__(self, pkg, fixers, update_changelog, compat_release,
                 pre_check=None, post_check=None, propose_addon_only=None,
                 committer=None, log_id=None):
        super(JanitorLintianFixer, self).__init__(
            pkg, fixers=fixers, update_changelog=update_changelog,
            compat_release=compat_release, pre_check=pre_check,
            post_check=post_check, propose_addon_only=propose_addon_only,
            committer=committer)
        self._log_id = log_id

    def get_proposal_description(self, existing_proposal):
        if existing_proposal:
            existing_description = existing_proposal.get_description()
            existing_description = strip_janitor_blurb(existing_description)
            existing_lines = parse_mp_description(existing_description)
        else:
            existing_lines = []
        return add_janitor_blurb(create_mp_description(
            existing_lines + [l for r, l in self.applied]), {
                'package': self._pkg, 'log_id': self._log_id})

    def describe(self, result):
        tags = set()
        for brush_result, unused_summary in self.applied:
            tags.update(brush_result.fixed_lintian_tags)
        if result.merge_proposal:
            if result.is_new:
                return 'Proposed fixes %r' % tags
            elif tags:
                return 'Updated proposal with fixes %r' % tags
            else:
                return 'No new fixes for proposal'
        else:
            if tags:
                return 'Pushed fixes %r' % tags
            else:
                return 'Nothing to do.'


class JanitorNewUpstreamMerger(NewUpstreamMerger):

    @staticmethod
    def setup_argparser(subparser):
        subparser.add_argument(
            '--snapshot',
            help='Merge a new upstream snapshot rather than a release',
            action='store_true')

    def describe(self, result):
        if result.merge_proposal:
            if result.is_new:
                return (
                    'Created merge proposal %s merging new '
                    'upstream version %s.' % (
                        result.merge_proposal.url,
                        self._upstream_version))
            else:
                return 'Updated merge proposal %s for upstream version %s.' % (
                    result.merge_proposal.url, self._upstream_version)
        return 'Did nothing.'


class WorkerResult(object):

    def __init__(self, pkg, log_id, start_time, finish_time, description,
                 proposal_url=None, is_new=None):
        self.package = pkg
        self.log_id = log_id
        self.start_time = start_time
        self.finish_time = finish_time
        self.description = description
        self.proposal_url = proposal_url
        self.is_new = is_new


debian_info = distro_info.DebianDistroInfo()


lintian_subparser = argparse.ArgumentParser(prog='lintian-brush')
JanitorLintianFixer.setup_argparser(lintian_subparser)

new_upstream_subparser = argparse.ArgumentParser(prog='new-upstream')
JanitorNewUpstreamMerger.setup_argparser(new_upstream_subparser)


def process_package(vcs_url, mode, env, command, output_directory,
                    incoming=None, dry_run=False, refresh=False,
                    build_command=None, pre_check_command=None,
                    post_check_command=None, possible_transports=None,
                    possible_hosters=None):
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
        raise AssertionError('unknown subcommand %s' % command[0])
    log_id = str(uuid.uuid4())
    assert pkg is not None
    assert output_directory is not None
    log_path = os.path.join(output_directory, pkg, 'logs', log_id)
    os.makedirs(log_path)

    if pre_check_command:
        def pre_check(local_tree):
            try:
                subprocess.check_call(
                    pre_check_command, shell=True, cwd=local_tree.basedir)
            except subprocess.CalledProcessError:
                note('%r: pre-check failed, skipping', pkg)
                return False
            return True
    else:
        pre_check = None

    def post_check(local_tree, since_revid):
        if post_check_command:
            try:
                subprocess.check_call(
                    post_check_command, shell=True, cwd=local_tree.basedir,
                    env={'SINCE_REVID': since_revid})
            except subprocess.CalledProcessError:
                note('%s: post-check failed, skipping', pkg)
                return False
        if build_command:
            add_dummy_changelog_entry(
                local_tree.basedir, build_version_suffix, build_suite,
                'Build for debian-janitor apt repository.')
            with open(os.path.join(log_path, 'build.log'), 'w') as f:
                try:
                    build(local_tree, outf=f, build_command=build_command,
                          incoming=incoming, distribution=build_suite)
                except BuildFailedError:
                    note('%s: build failed, skipping', pkg)
                    return False
            changes_filename = predict_changes_filename(local_tree)
            changes_path = os.path.join(incoming, changes_filename)
            note('Changes file: %s / %s', changes_path, build_suite)
            if not os.path.exists(changes_path):
                warning('Expected changes path %s does not exist.',
                        changes_path)
        return True

    note('Processing: %s (mode: %s)', pkg, mode)
    start_time = datetime.now()

    try:
        main_branch = Branch.open(
            vcs_url, possible_transports=possible_transports)
    except socket.error:
        return WorkerResult(
            pkg, log_id, start_time, datetime.now(), 'ignoring, socket error')
    except errors.NotBranchError as e:
        return WorkerResult(
            pkg, log_id, start_time, datetime.now(),
            'Branch does not exist: %s' % e)
    except errors.UnsupportedProtocol:
        return WorkerResult(
            pkg, log_id, start_time, datetime.now(),
            'Branch available over unsupported protocol')
    except errors.ConnectionError as e:
        return WorkerResult(pkg, log_id, start_time, datetime.now(), str(e))
    except errors.PermissionDenied as e:
        return WorkerResult(pkg, log_id, start_time, datetime.now(), str(e))
    except errors.InvalidHttpResponse as e:
        return WorkerResult(pkg, log_id, start_time, datetime.now(), str(e))
    except errors.TransportError as e:
        return WorkerResult(pkg, log_id, start_time, datetime.now(), str(e))
    else:
        if command[0] == 'lintian-brush':
            # TODO(jelmer): 'fixers' is wrong; it's actually tags.
            fixers = get_fixers(
                available_lintian_fixers(), tags=subargs.fixers)
            branch_changer = JanitorLintianFixer(
                pkg, fixers=fixers,
                update_changelog=subargs.update_changelog,
                compat_release=subargs.compat_release,
                pre_check=pre_check, post_check=post_check,
                propose_addon_only=subargs.propose_addon_only,
                committer=committer, log_id=log_id)
            branch_name = "lintian-fixes"
        elif command[0] == 'new-upstream':
            branch_changer = JanitorNewUpstreamMerger(
                subargs.snapshot, post_check=post_check,
                pre_check=pre_check)
            branch_name = "new-upstream"
        if mode == 'build-only':
            dry_run = True
            mode = 'propose'
        try:
            result = propose_or_push(
                main_branch, branch_name, branch_changer, mode,
                possible_transports=possible_transports,
                possible_hosters=possible_hosters,
                refresh=refresh, dry_run=dry_run)
        except UnsupportedHoster:
            return WorkerResult(
                pkg, log_id, start_time, datetime.now(), 'Hosted unsupported.')
        except NoSuchProject as e:
            return WorkerResult(
                pkg, log_id, start_time, datetime.now(),
                'project %s was not found' % e.project)
        except BuildFailedError:
            return WorkerResult(
                pkg, log_id, start_time, datetime.now(), 'build failed')
        except MissingUpstreamTarball:
            return WorkerResult(
                pkg, log_id, start_time, datetime.now(),
                'unable to find upstream source')
        except errors.PermissionDenied as e:
            return WorkerResult(
                pkg, log_id, start_time, datetime.now(), str(e))
        except PostCheckFailed as e:
            return WorkerResult(
                pkg, log_id, start_time, datetime.now(), str(e))
        else:
            description = branch_changer.describe(result)
            return WorkerResult(
                pkg, log_id, start_time, datetime.now(),
                description)


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
        "--dry-run",
        help="Create branches but don't push or propose anything.",
        action="store_true", default=False)
    parser.add_argument(
        '--refresh', action="store_true",
        help='Refresh branch (discard current branch) and '
        'create from scratch')
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
    parser.add_argument(
        '--mode',
        help='Mode for pushing',
        choices=['push', 'attempt-push', 'propose', 'build-only'],
        default="propose", type=str)

    parser.add_argument('command', nargs=argparse.REMAINDER)

    args = parser.parse_args(argv)
    if args.branch_url is None:
        parser.print_usage()
        return 1
    result = process_package(
        args.branch_url, args.mode, os.environ,
        args.command, output_directory=args.output_directory,
        incoming=args.output_directory, dry_run=args.dry_run,
        refresh=args.refresh, build_command=args.build_command,
        pre_check_command=args.pre_check,
        post_check_command=args.post_check)
    if result.proposal_url:
        note('%s: %s: %s', result.package, result.description,
             result.proposal_url)
    else:
        note('%s: %s', result.package, result.description)


if __name__ == '__main__':
    import sys
    sys.exit(main())
