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
from contextlib import contextmanager
from datetime import datetime
from debian.changelog import Changelog, Version, ChangelogCreateError
import distro_info
import json
import os
import subprocess
import sys
import traceback
from typing import Callable, Dict, List, Optional, Any, Type, Iterator, Tuple

import breezy
from breezy import osutils
from breezy.config import GlobalStack
from breezy.transport import Transport
from breezy.workingtree import WorkingTree

import silver_platter
from silver_platter.debian import (
    MissingUpstreamTarball,
    Workspace,
    pick_additional_colocated_branches,
    control_files_in_root,
    control_file_present,
)
from silver_platter.debian.changer import (
    ChangerError,
    DebianChanger,
    )
from silver_platter.debian.lintian import (
    available_lintian_fixers,
    get_fixers,
    run_lintian_fixers,
    has_nontrivial_changes,
    DEFAULT_ADDON_FIXERS,
    DEFAULT_MINIMUM_CERTAINTY,
    calculate_value as lintian_brush_calculate_value,
)
from silver_platter.proposal import Hoster
from lintian_brush.config import Config as LintianBrushConfig

from silver_platter.utils import (
    full_branch_url,
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
    build_once,
    MissingChangesFile,
    SbuildFailure,
)
from .dist import (
    create_dist_schroot,
    DetailedDistCommandFailed,
    UnidentifiedError,
    )

from .trace import (
    note,
    warning,
)
from .vcs import (
    BranchOpenFailure,
    open_branch_ext,
    )


# Whether to trust packages enough to run code from them,
# e.g. when guessing repo location.
TRUST_PACKAGE = False


DEFAULT_DIST_COMMAND = os.path.join(os.path.dirname(__file__), '..', 'dist.py')
DEFAULT_BUILD_COMMAND = 'sbuild -A -s -v'


class SubWorkerResult(object):

    def __init__(
            self, description: Optional[str], value: Optional[int],
            auxiliary_branches: Optional[List[str]] = None,
            tags: Optional[List[str]] = None):
        self.description = description
        self.value = value
        self.auxiliary_branches = auxiliary_branches
        self.tags = tags

    @classmethod
    def from_changer_result(cls, result):
        return cls(
            tags=result.tags,
            auxiliary_branches=result.auxiliary_branches,
            description=result.description,
            value=result.value)


class SubWorker(object):

    name: str

    def __init__(self, command: List[str], env: Dict[str, str]) -> None:
        """Initialize a subworker.

        Args:
          command: List of command arguments
          env: Environment dictionary
        """

    def make_changes(self, local_tree: WorkingTree, subpath: str,
                     report_context: Callable[[str], None],
                     metadata, base_metadata) -> SubWorkerResult:
        """Make the actual changes to a tree.

        Args:
          local_tree: Tree to make changes to
          report_context: report context
          metadata: JSON Dictionary that can be used for storing results
          base_metadata: Optional JSON Dictionary with results of
            any previous runs this one is based on
          subpath: Path in the branch where the package resides
        Returns:
          SubWorkerResult
        """
        raise NotImplementedError(self.make_changes)


class ChangerWorker(SubWorker):

    changer_cls: Type[DebianChanger]

    def __init__(self, command, env):
        self.committer = env.get('COMMITTER')
        self.changer = self.changer_cls()
        subparser = argparse.ArgumentParser(
            prog=self.name, parents=[common_parser])
        self.changer.setup_parser(subparser)
        self.args = subparser.parse_args(command)

    def make_changes(self, local_tree, subpath, report_context, metadata,
                     base_metadata):
        try:
            result = self.changer.make_changes(
                local_tree, subpath=subpath, committer=self.committer,
                update_changelog=False)
        except ChangerError as e:
            raise WorkerFailure(e.category, e.summary)

        return SubWorkerResult.from_changer_result(result=result)


common_parser = argparse.ArgumentParser(add_help=False)
common_parser.add_argument(
    '--no-update-changelog', action="store_false", default=None,
    dest="update_changelog", help="do not update the changelog")
common_parser.add_argument(
    '--update-changelog', action="store_true", dest="update_changelog",
    help="force updating of the changelog", default=None)


class OrphanWorker(ChangerWorker):

    name = 'orphan'
    from silver_platter.debian.orphan import OrphanChanger
    changer_cls = OrphanChanger


class LintianBrushWorker(SubWorker):
    """Janitor-specific Lintian Fixer."""

    name = 'lintian-brush'

    def __init__(self, command, env):
        from lintian_brush import (
            SUPPORTED_CERTAINTIES,
            )
        self.committer = env.get('COMMITTER')
        subparser = argparse.ArgumentParser(
            prog='lintian-brush', parents=[common_parser])
        subparser.add_argument("tags", nargs='*')
        subparser.add_argument(
            '--exclude', action='append', type=str,
            help="Exclude fixer.")
        subparser.add_argument(
            '--compat-release', type=str, default=None,
            help='Oldest Debian release to be compatible with.')
        subparser.add_argument(
            '--propose-addon-only',
            help='Fixers that should be considered add-on-only.',
            type=str, action='append', default=DEFAULT_ADDON_FIXERS)
        subparser.add_argument(
            '--allow-reformatting', default=None, action='store_true',
            help='Whether to allow reformatting.')
        subparser.add_argument(
            '--minimum-certainty',
            type=str,
            choices=SUPPORTED_CERTAINTIES,
            default=None,
            help=argparse.SUPPRESS)
        self.args = subparser.parse_args(command)

    def make_changes(self, local_tree, subpath, report_context, metadata,
                     base_metadata):
        from lintian_brush import (
            version_string as lintian_brush_version_string,
            )
        fixers = get_fixers(
            available_lintian_fixers(), tags=self.args.tags,
            exclude=self.args.exclude)

        compat_release = self.args.compat_release
        allow_reformatting = self.args.allow_reformatting
        minimum_certainty = self.args.minimum_certainty
        try:
            cfg = LintianBrushConfig.from_workingtree(local_tree, '')
        except FileNotFoundError:
            pass
        else:
            if compat_release is None:
                compat_release = cfg.compat_release()
            allow_reformatting = cfg.allow_reformatting()
            minimum_certainty = cfg.minimum_certainty()
        if compat_release is None:
            compat_release = debian_info.stable()
        if allow_reformatting is None:
            allow_reformatting = False
        if minimum_certainty is None:
            minimum_certainty = DEFAULT_MINIMUM_CERTAINTY

        with local_tree.lock_write():
            if control_files_in_root(local_tree, subpath):
                raise WorkerFailure(
                    'control-files-in-root',
                    'control files live in root rather than debian/ '
                    '(LarstIQ mode)')

            try:
                overall_result = run_lintian_fixers(
                        local_tree, fixers,
                        committer=self.committer,
                        update_changelog=self.args.update_changelog,
                        compat_release=compat_release,
                        minimum_certainty=minimum_certainty,
                        allow_reformatting=allow_reformatting,
                        trust_package=TRUST_PACKAGE,
                        net_access=True, subpath=subpath,
                        opinionated=False,
                        diligence=10)
            except ChangelogCreateError as e:
                raise WorkerFailure(
                    'changelog-create-error',
                    'Error creating changelog entry: %s' % e)

        if overall_result.failed_fixers:
            for fixer_name, failure in overall_result.failed_fixers.items():
                note('Fixer %r failed to run:', fixer_name)
                sys.stderr.write(str(failure))

        metadata['versions'] = {
            'lintian-brush': lintian_brush_version_string,
            'silver-platter': silver_platter.version_string,
            'breezy': breezy.version_string,
            }
        metadata['applied'] = []
        if base_metadata:
            metadata['applied'].extend(base_metadata['applied'])
        for result, summary in overall_result.success:
            metadata['applied'].append({
                'summary': summary,
                'description': result.description,
                'fixed_lintian_tags': result.fixed_lintian_tags,
                'revision_id': result.revision_id.decode('utf-8'),
                'certainty': result.certainty})
        metadata['failed'] = {
            name: str(e) for (name, e) in overall_result.failed_fixers.items()}
        metadata['add_on_only'] = not has_nontrivial_changes(
            overall_result.success, self.args.propose_addon_only)
        if base_metadata and not base_metadata['add_on_only']:
            metadata['add_on_only'] = False

        if not overall_result.success:
            raise WorkerFailure('nothing-to-do', 'no fixers to apply')

        tags = set()
        for entry in metadata['applied']:
            tags.update(entry['fixed_lintian_tags'])
        value = lintian_brush_calculate_value(tags)
        return SubWorkerResult(
            description='Applied fixes for %r' % tags,
            value=value, tags=[])


class NewUpstreamWorker(ChangerWorker):

    name = 'new-upstream'
    from silver_platter.debian.upstream import (
        NewUpstreamChanger,
        )

    class changer_cls(NewUpstreamChanger):

        def create_dist_from_command(self, tree, package, version, target_dir):
            from silver_platter.debian.upstream import DistCommandFailed
            try:
                return create_dist_schroot(
                    tree, subdir=package, target_dir=target_dir,
                    packaging_tree=tree, chroot=self.args.chroot)
            except DetailedDistCommandFailed:
                raise
            except UnidentifiedError as e:
                traceback.print_exc()
                lines = [line for line in e.lines if line]
                if e.secondary:
                    raise DistCommandFailed(e.secondary[1])
                elif len(lines) == 1:
                    raise DistCommandFailed(lines[0])
                else:
                    raise DistCommandFailed(
                        'command %r failed with unidentified error '
                        '(return code %d)' % (e.argv, e.retcode))
            except Exception as e:
                traceback.print_exc()
                raise DistCommandFailed(str(e))

    def make_changes(self, local_tree, subpath, report_context, metadata,
                     base_metadata):
        try:
            return NewUpstreamWorker.make_changes(
                self,
                local_tree, subpath, report_context, metadata, base_metadata)
        except DetailedDistCommandFailed as e:
            error_code = 'dist-' + e.error.kind
            error_description = str(e.error)
            raise WorkerFailure(error_code, error_description)


class JustBuildWorker(SubWorker):

    name = 'just-build'

    def __init__(self, command, env):
        subparser = argparse.ArgumentParser(
            prog='just-build', parents=[common_parser])
        subparser.add_argument(
            '--revision', type=str,
            help='Specific revision to build.')
        self.args = subparser.parse_args(command)

    def make_changes(self, local_tree, subpath, report_context, metadata,
                     base_metadata):
        if self.args.revision:
            local_tree.update(revision=self.args.revision.encode('utf-8'))
        if control_files_in_root(local_tree, subpath):
            raise WorkerFailure(
                'control-files-in-root',
                'control files live in root rather than debian/ '
                '(LarstIQ mode)')
        return SubWorkerResult(None, None)


class UncommittedWorker(ChangerWorker):

    from silver_platter.debian.uncommitted import UncommittedChanger
    name = 'import-upload'
    changer_cls = UncommittedChanger


class ScrubObsoleteWorker(ChangerWorker):
    from silver_platter.debian.scrub_obsolete import ScrubObsoleteChanger

    name = 'scrub-obsolete'
    changer_cls = ScrubObsoleteChanger


class CMEWorker(ChangerWorker):

    from silver_platter.debian.cme import CMEChanger
    name = 'cme-fix'
    changer_cls = CMEChanger


class MultiArchHintsWorker(ChangerWorker):

    from silver_platter.debian.multiarch import MultiArchHintsChanger
    name = 'apply-multiarch-fixes'
    changer_cls = MultiArchHintsChanger


class WorkerResult(object):

    def __init__(
            self, description: Optional[str],
            value: Optional[int],
            changes_filename: Optional[str] = None) -> None:
        self.description = description
        self.value = value
        self.changes_filename = changes_filename


class WorkerFailure(Exception):
    """Worker processing failed."""

    def __init__(self, code: str, description: str) -> None:
        self.code = code
        self.description = description


def tree_set_changelog_version(
        tree: WorkingTree, build_version: Version, subpath: str) -> None:
    cl_path = osutils.pathjoin(subpath, 'debian/changelog')
    with tree.get_file(cl_path) as f:
        cl = Changelog(f)
    if Version(str(cl.version) + '~') > build_version:
        return
    cl.version = build_version
    with open(tree.abspath(cl_path), 'w') as f:
        cl.write_to_open_file(f)


debian_info = distro_info.DebianDistroInfo()


# TODO(jelmer): Just invoke the silver-platter subcommand
SUBWORKERS = {
    swc.name: swc for swc in [
        LintianBrushWorker,
        NewUpstreamWorker,
        JustBuildWorker,
        MultiArchHintsWorker,
        OrphanWorker,
        UncommittedWorker,
        CMEWorker,
        ScrubObsoleteWorker,
        ]}


@contextmanager
def process_package(vcs_url: str, subpath: str, env: Dict[str, str],
                    command: List[str], output_directory: str,
                    metadata: Any, build_command: Optional[str] = None,
                    pre_check_command: Optional[str] = None,
                    post_check_command: Optional[str] = None,
                    possible_transports: Optional[List[Transport]] = None,
                    possible_hosters: Optional[List[Hoster]] = None,
                    resume_branch_url: Optional[str] = None,
                    cached_branch_url: Optional[str] = None,
                    last_build_version: Optional[Version] = None,
                    build_distribution: Optional[str] = None,
                    build_suffix: Optional[str] = None,
                    resume_subworker_result: Any = None
                    ) -> Iterator[Tuple[Workspace, WorkerResult]]:
    pkg = env['PACKAGE']

    metadata['command'] = command

    subworker_cls: Type[SubWorker]
    try:
        subworker_cls = SUBWORKERS[command[0]]
    except KeyError:
        raise WorkerFailure(
            'unknown-subcommand',
            'unknown subcommand %s' % command[0])
    subworker = subworker_cls(command[1:], env)

    note('Opening branch at %s', vcs_url)
    try:
        main_branch = open_branch_ext(
            vcs_url, possible_transports=possible_transports)
    except BranchOpenFailure as e:
        raise WorkerFailure('worker-%s' % e.code, e.description)

    if cached_branch_url:
        try:
            cached_branch = open_branch(
                cached_branch_url,
                possible_transports=possible_transports)
        except BranchMissing as e:
            note('Cached branch URL %s missing: %s', cached_branch_url, e)
            cached_branch = None
        except BranchUnavailable as e:
            warning('Cached branch URL %s unavailable: %s',
                    cached_branch_url, e)
            cached_branch = None
        else:
            note('Using cached branch %s', full_branch_url(cached_branch))
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
            note('Resuming from branch %s', full_branch_url(resume_branch))
    else:
        resume_branch = None

    with Workspace(
            main_branch, resume_branch=resume_branch,
            cached_branch=cached_branch,
            path=os.path.join(output_directory, pkg),
            additional_colocated_branches=(
                pick_additional_colocated_branches(main_branch))) as ws:
        if ws.local_tree.has_changes():
            if list(ws.local_tree.iter_references()):
                raise WorkerFailure(
                    'requires-nested-tree-support',
                    'Missing support for nested trees in Breezy.')
            raise AssertionError

        metadata['revision'] = metadata['main_branch_revision'] = (
            ws.main_branch.last_revision().decode())

        if not control_file_present(ws.local_tree, subpath):
            if ws.local_tree.has_filename(
                    os.path.join(subpath, 'debian', 'debcargo.toml')):
                # debcargo packages are fine too
                pass
            else:
                raise WorkerFailure(
                    'missing-control-file',
                    'missing control file: debian/control')

        try:
            run_pre_check(ws.local_tree, pre_check_command)
        except PreCheckFailed as e:
            raise WorkerFailure('pre-check-failed', str(e))

        metadata['subworker'] = {}

        def provide_context(c):
            metadata['context'] = c

        if ws.resume_branch is None:
            # If the resume branch was discarded for whatever reason, then we
            # don't need to pass in the subworker result.
            resume_subworker_result = None

        try:
            subworker_result = subworker.make_changes(
                ws.local_tree, subpath, provide_context, metadata['subworker'],
                resume_subworker_result)
        except WorkerFailure as e:
            if (e.code == 'nothing-to-do' and
                    resume_subworker_result is not None):
                e = WorkerFailure('nothing-new-to-do', e.description)
                raise e
            else:
                raise
        finally:
            metadata['revision'] = (
                ws.local_tree.branch.last_revision().decode())

        if command[0] != 'just-build':
            if not ws.changes_since_main():
                raise WorkerFailure('nothing-to-do', 'Nothing to do.')

            if ws.resume_branch and not ws.changes_since_resume():
                raise WorkerFailure('nothing-to-do', 'Nothing new to do.')

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
                    ws.local_tree, last_build_version, subpath)

            source_date_epoch = ws.local_tree.branch.repository.get_revision(
                ws.main_branch.last_revision()).timestamp
            try:
                if not build_suffix:
                    (changes_name, cl_version) = build_once(
                        ws.local_tree, build_distribution, output_directory,
                        build_command, subpath=subpath,
                        source_date_epoch=source_date_epoch)
                else:
                    (changes_name, cl_version) = build_incrementally(
                        ws.local_tree, '~' + build_suffix,
                        build_distribution, output_directory,
                        build_command, committer=env.get('COMMITTER'),
                        subpath=subpath, source_date_epoch=source_date_epoch)
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
                    if e.stage:
                        code = '%s-%s' % (e.stage, e.error.kind)
                    else:
                        code = e.error.kind
                elif e.stage is not None:
                    code = 'build-failed-stage-%s' % e.stage
                else:
                    code = 'build-failed'
                raise WorkerFailure(code, e.description)
            note('Built %s', changes_name)
        else:
            changes_name = None

        wr = WorkerResult(
            subworker_result.description, subworker_result.value,
            changes_filename=changes_name)
        yield ws, wr


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
        help='Version of the last built Debian package.')
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
        '--subpath', type=str,
        help='Path in the branch under which the package lives.',
        default='')
    parser.add_argument(
        '--build-command',
        help='Build package to verify it.', type=str,
        default=DEFAULT_BUILD_COMMAND)
    parser.add_argument(
        '--tgz-repo',
        help='Whether to create a tgz of the VCS repo.',
        action='store_true')
    parser.add_argument(
        '--build-distribution', type=str, help='Build distribution.')
    parser.add_argument('--build-suffix', type=str, help='Build suffix.')

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
        with process_package(
                args.branch_url, args.subpath, os.environ,
                args.command, output_directory, metadata,
                build_command=args.build_command,
                pre_check_command=args.pre_check,
                post_check_command=args.post_check,
                resume_branch_url=args.resume_branch_url,
                cached_branch_url=args.cached_branch_url,
                build_distribution=args.build_distribution,
                build_suffix=args.build_suffix,
                last_build_version=args.last_build_version,
                resume_subworker_result=resume_subworker_result
                ) as (ws, result):
            if args.tgz_repo:
                subprocess.check_call(
                    ['tar', 'czf', os.environ['PACKAGE'] + '.tgz',
                     os.environ['PACKAGE']],
                    cwd=output_directory)
            else:
                ws.defer_destroy()
    except WorkerFailure as e:
        metadata['code'] = e.code
        metadata['description'] = e.description
        note('Worker failed (%s): %s', e.code, e.description)
        return 0
    except BaseException as e:
        metadata['code'] = 'worker-exception'
        metadata['description'] = str(e)
        raise
    else:
        metadata['code'] = None
        metadata['value'] = result.value
        metadata['description'] = result.description
        note('%s', result.description)
        if result.changes_filename is not None:
            note('Built %s.', result.changes_filename)
        return 0
    finally:
        finish_time = datetime.now()
        note('Elapsed time: %s', finish_time - start_time)
        with open(os.path.join(output_directory, 'result.json'), 'w') as f:
            json.dump(metadata, f, indent=2)


if __name__ == '__main__':
    sys.exit(main())
