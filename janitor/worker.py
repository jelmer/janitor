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
from debian.changelog import Changelog, Version
import distro_info
import json
import os
import subprocess
import sys

from breezy.config import GlobalStack
from breezy.errors import NoRoundtrippingSupport

from silver_platter.debian import (
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
    check_quilt_patches_apply,
    merge_upstream,
    refresh_quilt_patches,
    InconsistentSourceFormatError,
    InvalidFormatUpstreamVersion,
    NewUpstreamMissing,
    UnparseableChangelog,
    UpstreamAlreadyImported,
    UpstreamAlreadyMerged,
    UpstreamMergeConflicted,
    UpstreamBranchUnavailable,
    UpstreamBranchUnknown,
    PackageIsNative,
    PreviousVersionTagMissing,
    PristineTarError,
    QuiltError,
    UScanError,
)

from silver_platter.utils import (
    run_pre_check,
    run_post_check,
    PreCheckFailed,
    PostCheckFailed,
    open_branch,
    BranchMissing,
    BranchUnavailable,
)

from .fix_build import build_incrementally
from .build import (
    MissingChangesFile,
    SbuildFailure,
)
from .trace import (
    note,
    warning,
)


# Whether to trust packages enough to run code from them,
# e.g. when guessing repo location.
TRUST_PACKAGE = False


class SubWorker(object):

    def __init__(self, command, env):
        """Initialize a subworker.

        Args:
          command: List of command arguments
          env: Environment dictionary
        """

    def make_changes(self, local_tree, report_context, metadata,
                     base_metadata):
        """Make the actual changes to a tree.

        Args:
          local_tree: Tree to make changes to
          report_context: report context
          metadata: JSON Dictionary that can be used for storing results
          base_metadata: Optional JSON Dictionary with results of
            any previous runs this one is based on
        """
        raise NotImplementedError(self.make_changes)

    def build_suite(self):
        """Returns the name of the suite to build for."""
        raise NotImplementedError(self.build_suite)

    def build_version_suffix(self):
        raise NotImplementedError(self.build_version_suffix)


class LintianBrushWorker(SubWorker):
    """Janitor-specific Lintian Fixer."""

    def __init__(self, command, env):
        self.committer = env.get('COMMITTER')
        subparser = argparse.ArgumentParser(prog='lintian-brush')
        subparser.add_argument("tags", nargs='*')
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

    def make_changes(self, local_tree, report_context, metadata,
                     base_metadata):
        fixers = get_fixers(
            available_lintian_fixers(), tags=self.args.tags)

        with local_tree.lock_write():
            applied, failed = run_lintian_fixers(
                    local_tree, fixers,
                    committer=self.committer,
                    update_changelog=self.args.update_changelog,
                    compat_release=self.args.compat_release,
                    trust_package=TRUST_PACKAGE)

        if failed:
            for fixer_name, failure in failed.items():
                note('Fixer %r failed to run:', fixer_name)
                sys.stderr.write(failure.errors)

        metadata['applied'] = []
        if base_metadata:
            metadata['applied'].extend(base_metadata['applied'])
        for result, summary in applied:
            metadata['applied'].append({
                'summary': summary,
                'description': result.description,
                'fixed_lintian_tags': result.fixed_lintian_tags,
                'certainty': result.certainty})
        metadata['failed'] = {
            name: e.errors for (name, e) in failed.items()}
        metadata['add_on_only'] = not has_nontrivial_changes(
            applied, self.args.propose_addon_only)
        if base_metadata and not base_metadata['add_on_only']:
            metadata['add_on_only'] = False

        if not applied:
            raise WorkerFailure('nothing-to-do', 'no fixers to apply')
        else:
            tags = set()
            for entry in metadata['applied']:
                tags.update(entry['fixed_lintian_tags'])
        return 'Applied fixes for %r' % tags

    def build_suite(self):
        return 'lintian-fixes'

    def build_version_suffix(self):
        return 'jan+lint'


class NewUpstreamWorker(SubWorker):

    def __init__(self, command, env):
        self.committer = env.get('COMMITTER')
        subparser = argparse.ArgumentParser(prog='new-upstream')
        subparser.add_argument(
            '--snapshot',
            help='Merge a new upstream snapshot rather than a release',
            action='store_true')
        self.args = subparser.parse_args(command)

    def make_changes(self, local_tree, report_context, metadata,
                     base_metadata):
        # Make sure that the quilt patches applied in the first place..
        with local_tree.lock_write():
            try:
                result = merge_upstream(
                    tree=local_tree, snapshot=self.args.snapshot,
                    committer=self.committer, trust_package=TRUST_PACKAGE)
            except UpstreamAlreadyImported as e:
                report_context(e.version)
                metadata['upstream_version'] = e.version
                error_description = (
                    "Upstream version %s already imported." % (e.version))
                raise WorkerFailure(
                    'upstream-already-imported', error_description)
            except UpstreamAlreadyMerged as e:
                error_description = (
                    "Last upstream version %s already merged." % e.version)
                error_code = 'nothing-to-do'
                report_context(e.version)
                metadata['upstream_version'] = e.version
                raise WorkerFailure(error_code, error_description)
            except NewUpstreamMissing:
                error_description = "Unable to find new upstream source."
                error_code = 'new-upstream-missing'
                raise WorkerFailure(error_code, error_description)
            except UpstreamBranchUnavailable:
                error_description = "The upstream branch was unavailable."
                error_code = 'upstream-branch-unavailable'
                raise WorkerFailure(error_code, error_description)
            except UpstreamMergeConflicted as e:
                error_description = "Upstream version %s conflicted." % (
                    e.version)
                error_code = 'upstream-merged-conflicts'
                report_context(e.version)
                metadata['upstream_version'] = e.version
                raise WorkerFailure(error_code, error_description)
            except PreviousVersionTagMissing as e:
                error_description = (
                     "Previous upstream version %s missing (tag: %s)" %
                     (e.version, e.tag_name))
                error_code = 'previous-upstream-missing'
                raise WorkerFailure(error_code, error_description)
            except PristineTarError as e:
                error_description = ('Error from pristine-tar: %s' % e)
                error_code = 'pristine-tar-error'
                raise WorkerFailure(error_code, error_description)
            except UpstreamBranchUnknown:
                error_description = (
                    'The location of the upstream branch is unknown.')
                error_code = 'upstream-branch-unknown'
                raise WorkerFailure(error_code, error_description)
            except PackageIsNative:
                error_description = (
                    'Package is native; unable to merge upstream.')
                error_code = 'native-package'
                raise WorkerFailure(error_code, error_description)
            except NoRoundtrippingSupport:
                error_description = (
                    'Unable to import upstream repository into '
                    'packaging repository.')
                error_code = 'roundtripping-error'
                raise WorkerFailure(error_code, error_description)
            except InconsistentSourceFormatError as e:
                error_description = str(e)
                error_code = 'inconsistent-source-format'
                raise WorkerFailure(error_code, error_description)
            except InvalidFormatUpstreamVersion as e:
                error_description = (
                        'Invalid format upstream version: %r',
                        e.version)
                error_code = 'invalid-upstream-version-format'
                raise WorkerFailure(error_code, error_description)
            except UnparseableChangelog as e:
                error_description = str(e)
                error_code = 'unparseable-changelog'
                raise WorkerFailure(error_code, error_description)
            except UScanError as e:
                error_description = str(e)
                error_code = 'uscan-error'
                raise WorkerFailure(error_code, error_description)

            if local_tree.has_filename('debian/patches/series'):
                try:
                    refresh_quilt_patches(local_tree, committer=self.committer)
                except QuiltError as e:
                    error_description = (
                        "An error (%d) occurred refreshing quilt patches: "
                        "%s%s" % (e.retcode, e.stderr, e.extra))
                    error_code = 'quilt-refresh-error'
                    raise WorkerFailure(error_code, error_description)

            report_context(result.new_upstream_version)
            metadata['old_upstream_version'] = result.old_upstream_version
            metadata['upstream_version'] = result.new_upstream_version
            if result.upstream_branch:
                metadata['upstream_branch_url'] = (
                    result.upstream_branch.user_url)
                metadata['upstream_branch_browse'] = (
                    result.upstream_branch_browse)
            return "Merged new upstream version %s" % (
                result.new_upstream_version)

    def build_suite(self):
        if self.args.snapshot:
            return 'fresh-snapshots'
        else:
            return 'fresh-releases'

    def build_version_suffix(self):
        if self.args.snapshot:
            return 'jan+nus'
        else:
            return 'jan+nur'


class WorkerResult(object):

    def __init__(self, description, build_distribution=None,
                 build_version=None, changes_filename=None):
        self.description = description
        self.build_version = build_version
        self.build_distribution = build_distribution
        self.changes_filename = changes_filename


class WorkerFailure(Exception):
    """Worker processing failed."""

    def __init__(self, code, description):
        self.code = code
        self.description = description


def tree_set_changelog_version(tree, build_version):
    cl = Changelog(tree.get_file('debian/changelog'))
    if cl.version > Version(build_version):
        return
    cl.set_version(build_version)
    with open(tree.abspath('debian/changelog'), 'wb') as f:
        cl.write_to_open_file(f)


debian_info = distro_info.DebianDistroInfo()


def process_package(vcs_url, env, command, output_directory,
                    metadata, build_command=None, pre_check_command=None,
                    post_check_command=None, possible_transports=None,
                    possible_hosters=None, resume_branch_url=None,
                    cached_branch_url=None, tgz_repo=False,
                    last_build_version=None,
                    resume_subworker_result=None):
    pkg = env['PACKAGE']

    metadata['package'] = pkg
    metadata['command'] = command

    # TODO(jelmer): sort out this mess:
    if command[0] == 'lintian-brush':
        subworker_cls = LintianBrushWorker
    elif command[0] == 'new-upstream':
        subworker_cls = NewUpstreamWorker
    else:
        raise WorkerFailure(
            'unknown-subcommand',
            'unknown subcommand %s' % command[0])
    subworker = subworker_cls(command[1:], env)
    build_suite = subworker.build_suite()
    assert pkg is not None

    note('Processing: %s', pkg)

    try:
        main_branch = open_branch(
            vcs_url, possible_transports=possible_transports)
    except BranchUnavailable as e:
        raise WorkerFailure('worker-branch-unavailable', str(e))
    except BranchMissing as e:
        raise WorkerFailure('worker-branch-missing', str(e))

    if cached_branch_url:
        try:
            cached_branch = open_branch(
                cached_branch_url,
                possible_transports=possible_transports)
        except (BranchMissing, BranchUnavailable) as e:
            warning('Cached branch URL unavailable: %s', e)
            cached_branch = None
    else:
        cached_branch = None

    if resume_branch_url:
        try:
            resume_branch = open_branch(
                resume_branch_url,
                possible_transports=possible_transports)
        except BranchUnavailable as e:
            raise WorkerFailure('worker-resume-branch-unavailable', str(e))
        except BranchMissing as e:
            raise WorkerFailure('worker-resume-branch-missing', str(e))
    else:
        resume_branch = None

    with Workspace(main_branch, resume_branch=resume_branch,
                   cached_branch=cached_branch,
                   path=os.path.join(output_directory, pkg)) as ws:
        metadata['main_branch_revision'] = (
            ws.main_branch.last_revision().decode())

        if not ws.local_tree.has_filename('debian/control'):
            raise WorkerFailure(
                'missing-control-file',
                'missing control file: debian/control')

        try:
            run_pre_check(ws.local_tree, pre_check_command)
        except PreCheckFailed as e:
            raise WorkerFailure('pre-check-failed', str(e))

        try:
            check_quilt_patches_apply(ws.local_tree)
        except QuiltError as e:
            error_description = (
                "An error (%d) occurred running quilt in the original tree: "
                "%s%s" % (e.retcode, e.stderr, e.extra))
            raise WorkerFailure('before-quilt-error', error_description)

        metadata['subworker'] = {}

        def provide_context(c):
            metadata['context'] = c

        if ws.resume_branch is None:
            # If the resume branch was discarded for whatever reason, then we
            # don't need to pass in the subworker result.
            resume_subworker_result = None

        description = subworker.make_changes(
            ws.local_tree, provide_context, metadata['subworker'],
            resume_subworker_result)

        if not ws.changes_since_main():
            raise WorkerFailure('nothing-to-do', 'Nothing to do.')

        if not ws.changes_since_resume():
            raise WorkerFailure('nothing-new-to-do', 'Nothing new to do.')

        try:
            run_post_check(ws.local_tree, post_check_command, ws.orig_revid)
        except PostCheckFailed as e:
            raise WorkerFailure('post-check-failed', str(e))

        if build_command:
            if last_build_version:
                # Update the changelog entry with the previous build version;
                # This allows us to upload incremented versions for subsequent
                # runs.
                tree_set_changelog_version(
                    ws.local_tree, last_build_version)
            try:
                (changes_name, cl_version) = build_incrementally(
                    ws.local_tree, '~' + subworker.build_version_suffix(),
                    build_suite, output_directory,
                    build_command, committer=env.get('COMMITTER'))
            except MissingUpstreamTarball:
                raise WorkerFailure(
                    'build-missing-upstream-source',
                    'unable to find upstream source')
            except MissingChangesFile as e:
                raise WorkerFailure(
                    'build-missing-changes',
                    'Expected changes path %s does not exist.' % e.filename)
            except SbuildFailure as e:
                if e.error is not None:
                    code = '%s-%s' % (e.stage, e.error.kind)
                elif e.stage is not None:
                    code = 'build-failed-stage-%s' % e.stage
                else:
                    code = 'build-failed'
                raise WorkerFailure(code, e.description)
            note('Built %s', changes_name)
        else:
            build_suite = None
            changes_name = None
            cl_version = None

        if tgz_repo:
            subprocess.check_call(
                ['tar', 'czf', pkg + '.tgz', pkg],
                cwd=output_directory)
        else:
            ws.defer_destroy()
        return WorkerResult(
            description,
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
        '--resume-result-path', type=str,
        help=('Path to a JSON file with the results for '
              'the last run on the resumed branch.'))
    parser.add_argument(
        '--last-build-version', type=str,
        help='Version of the last built Debian package in this suite.')
    parser.add_argument(
        '--cached-branch-url', type=str,
        help='URL of cached branch to start from.')
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
        default='sbuild -A -s -v -d$DISTRIBUTION --build-dep-resolver=aspcud')
    parser.add_argument(
        '--tgz-repo',
        help='Whether to create a tgz of the VCS repo.',
        action='store_true')

    parser.add_argument('command', nargs=argparse.REMAINDER)

    args = parser.parse_args(argv)
    if args.branch_url is None:
        parser.print_usage()
        return 1

    output_directory = os.path.abspath(args.output_directory)

    global_config = GlobalStack()
    global_config.set('branch.fetch_tags', True)

    if args.resume_result_path:
        with open(args.resume_result_path, 'r') as f:
            resume_subworker_result = json.load(f)
    else:
        resume_subworker_result = None

    metadata = {}
    start_time = datetime.now()
    metadata['start_time'] = start_time.isoformat()
    try:
        result = process_package(
            args.branch_url, os.environ,
            args.command, output_directory, metadata,
            build_command=args.build_command, pre_check_command=args.pre_check,
            post_check_command=args.post_check,
            resume_branch_url=args.resume_branch_url,
            cached_branch_url=args.cached_branch_url,
            tgz_repo=args.tgz_repo,
            last_build_version=args.last_build_version,
            resume_subworker_result=resume_subworker_result)
    except WorkerFailure as e:
        metadata['code'] = e.code
        metadata['description'] = e.description
        note('Worker failed: %s', e.description)
        return 0
    except BaseException as e:
        metadata['code'] = 'worker-exception'
        metadata['description'] = str(e)
        raise
    else:
        metadata['code'] = None
        metadata['description'] = result.description
        metadata['changes_filename'] = result.changes_filename
        metadata['build_version'] = (
            str(result.build_version)
            if result.build_version else None)
        metadata['build_distribution'] = result.build_distribution
        note('%s', result.description)
        if result.changes_filename is not None:
            note('Built %s.', result.changes_filename)
        return 0
    finally:
        finish_time = datetime.now()
        note('Elapsed time: %s', finish_time - start_time)
        metadata['finish_time'] = finish_time.isoformat()
        metadata['duration'] = (finish_time - start_time).seconds
        with open(os.path.join(output_directory, 'result.json'), 'w') as f:
            json.dump(metadata, f, indent=2)


if __name__ == '__main__':
    sys.exit(main())
