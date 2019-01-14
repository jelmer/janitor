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

import distro_info

from datetime import datetime
import os
import socket
import subprocess
import sys
import uuid

import silver_platter   # noqa: F401
from silver_platter.debian import (
    propose_or_push,
    BuildFailedError,
    MissingUpstreamTarball,
    )
from silver_platter.debian.lintian import (
    LintianFixer,
    PostCheckFailed,
    available_lintian_fixers,
    create_mp_description,
    parse_mp_description,
    DEFAULT_ADDON_FIXERS,
    )
from silver_platter.debian.schedule import schedule_udd

from breezy import (
    errors,
    )

from breezy.branch import Branch
from breezy.trace import (
    note,
    warning,
    )

from breezy.plugins.propose.propose import (
    NoSuchProject,
    UnsupportedHoster,
    hosters,
    )

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state

import argparse
parser = argparse.ArgumentParser(prog='propose-lintian-fixes')
parser.add_argument("packages", nargs='*')
parser.add_argument('--lintian-log',
                    help="Path to lintian log file.", type=str,
                    default=None)
parser.add_argument("--fixers",
                    help="Fixers to run.", type=str, action='append')
parser.add_argument("--policy",
                    help="Policy file to read.", type=str,
                    default='policy.conf')
parser.add_argument("--dry-run",
                    help="Create branches but don't push or propose anything.",
                    action="store_true", default=False)
parser.add_argument('--propose-addon-only',
                    help='Fixers that should be considered add-on-only.',
                    type=str, action='append',
                    default=DEFAULT_ADDON_FIXERS)
parser.add_argument('--pre-check',
                    help='Command to run to check whether to process package.',
                    type=str)
parser.add_argument('--post-check',
                    help='Command to run to check package before pushing.',
                    type=str)
parser.add_argument('--verify-command',
                    help='Build package to verify it.', type=str,
                    default='brz bd --builder=\'sbuild -v\'')
parser.add_argument('--shuffle',
                    help='Shuffle order in which packages are processed.',
                    action='store_true')
parser.add_argument('--refresh',
                    help='Discard old branch and apply fixers from scratch.',
                    action='store_true')
parser.add_argument('--log-dir',
                    help='Directory to store logs in.',
                    type=str, default='public_html/pkg')
parser.add_argument(
    '--max-mps-per-maintainer',
    default=5,
    type=int, help='Maximum number of open merge proposals per maintainer.')
args = parser.parse_args()

JANITOR_BLURB = """
This merge proposal was created automatically by the Janitor bot
(https://janitor.debian.net/).

You can follow up to this merge proposal as you normally would.
"""

EXTRA_BLURB = """
To stop further merge proposals, reply "opt out".
"""

LOG_BLURB = """
Build and test logs for this branch can be found at
https://janitor.debian.net/pkg/%(package)s/logs/%(log_id)s.
"""


def strip_janitor_blurb(text):
    return text[text.index(JANITOR_BLURB):]


def add_janitor_blurb(text, env):
    text += JANITOR_BLURB
    if env['log_id']:
        text += (LOG_BLURB % env)
    return text


fixer_scripts = {}
for fixer in available_lintian_fixers():
    for tag in fixer.lintian_tags:
        fixer_scripts[tag] = fixer

available_fixers = set(fixer_scripts)
if args.fixers:
    available_fixers = available_fixers.intersection(set(args.fixers))

open_proposals = []
for name, hoster_cls in hosters.items():
    for instance in hoster_cls.iter_instances():
        open_proposals.extend(instance.iter_my_proposals(status='open'))

open_mps_per_maintainer = {}
for proposal in open_proposals:
    maintainer_email = state.get_maintainer_email(proposal.url)
    if maintainer_email is None:
        warning('No maintainer email known for %s', proposal.url)
    open_mps_per_maintainer.setdefault(maintainer_email, 0)
    open_mps_per_maintainer[maintainer_email] += 1

possible_transports = []
possible_hosters = []

todo = schedule_udd(
    args.policy, args.propose_addon_only, args.packages,
    available_fixers, args.shuffle)

subparser = argparse.ArgumentParser(prog='lintian-brush')
subparser.add_argument("fixers", nargs='*')
subparser.add_argument(
    '--no-update-changelog', action="store_false", default=None,
    dest="update_changelog", help="do not update the changelog")
subparser.add_argument(
    '--update-changelog', action="store_true", dest="update_changelog",
    help="force updating of the changelog", default=None)

debian_info = distro_info.DebianDistroInfo()


class JanitorLintianFixer(LintianFixer):
    """Janitor-specific Lintian Fixer."""

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


class JanitorResult(object):

    def __init__(self, pkg, log_id, start_time, finish_time, description,
                 proposal_url=None):
        self.package = pkg
        self.log_id = log_id
        self.start_time = start_time
        self.finish_time = finish_time
        self.description = description
        self.proposal_url = proposal_url


def process_package(vcs_url, mode, env, command):
    pkg = env['PACKAGE']
    committer = env['COMMITTER']
    subargs = subparser.parse_args(command[1:])
    log_id = str(uuid.uuid4())
    log_path = os.path.join(
        os.path.abspath(args.log_dir), pkg, 'logs', log_id)
    os.makedirs(log_path)

    if args.pre_check:
        def pre_check(local_tree):
            try:
                subprocess.check_call(
                        args.pre_check, shell=True, cwd=local_tree.basedir)
            except subprocess.CalledProcessError:
                note('%r: pre-check failed, skipping', pkg)
                return False
            return True
    else:
        pre_check = None

    def post_check(local_tree, since_revid):
        if args.post_check:
            try:
                subprocess.check_call(
                    args.post_check, shell=True, cwd=local_tree.basedir,
                    env={'SINCE_REVID': since_revid})
            except subprocess.CalledProcessError:
                note('%s: post-check failed, skipping', pkg)
                return False
        if args.verify_command:
            with open(os.path.join(log_path, 'build.log'), 'w') as f:
                try:
                    subprocess.check_call(
                        args.verify_command, shell=True,
                        cwd=local_tree.basedir,
                        stdout=f, stderr=f)
                except subprocess.CalledProcessError:
                    note('%s: build failed, skipping', pkg)
                    return False
        return True

    note('Processing: %s (mode: %s)', pkg, mode)
    start_time = datetime.now()

    try:
        main_branch = Branch.open(
                vcs_url, possible_transports=possible_transports)
    except socket.error:
        return JanitorResult(pkg, log_id, start_time, datetime.now(), 'ignoring, socket error')
    except errors.NotBranchError as e:
        return JanitorResult(pkg, log_id, start_time, datetime.now(), 'Branch does not exist: %s' % e)
    except errors.UnsupportedProtocol:
        return JanitorResult(
            pkg, log_id, start_time, datetime.now(), 'Branch available over unsupported protocol')
    except errors.ConnectionError as e:
        return JanitorResult(pkg, log_id, start_time, datetime.now(), str(e))
    except errors.PermissionDenied as e:
        return JanitorResult(pkg, log_id, start_time, datetime.now(), str(e))
    except errors.InvalidHttpResponse as e:
        return JanitorResult(pkg, log_id, start_time, datetime.now(), str(e))
    except errors.TransportError as e:
        return JanitorResult(pkg, log_id, start_time, datetime.now(), str(e))
    else:
        if subargs.fixers:
            fixers = subargs.fixers
        else:
            fixers = available_fixers
        branch_changer = JanitorLintianFixer(
                pkg, fixers=[fixer_scripts[fixer] for fixer in fixers],
                update_changelog=subargs.update_changelog,
                compat_release=debian_info.stable(),
                pre_check=pre_check, post_check=post_check,
                propose_addon_only=args.propose_addon_only,
                committer=committer, log_id=log_id)
        try:
            result = propose_or_push(
                    main_branch, "lintian-fixes", branch_changer, mode,
                    possible_transports=possible_transports,
                    possible_hosters=possible_hosters,
                    refresh=args.refresh, dry_run=args.dry_run)
        except UnsupportedHoster:
            return JanitorResult(
                pkg, log_id, start_time, datetime.now(), 'Hosted unsupported.')
        except NoSuchProject as e:
            return JanitorResult(
                pkg, log_id, start_time, datetime.now(),
                'project %s was not found' % e.project)
        except BuildFailedError:
            return JanitorResult(
                pkg, log_id, start_time, datetime.now(), 'build failed')
        except MissingUpstreamTarball:
            return JanitorResult(
                pkg, log_id, start_time, datetime.now(),
                'unable to find upstream source')
        except errors.PermissionDenied as e:
            return JanitorResult(
                pkg, log_id, start_time, datetime.now(), str(e))
        except PostCheckFailed as e:
            return JanitorResult(
                pkg, log_id, start_time, datetime.now(), str(e))
        else:
            tags = set()
            for brush_result, unused_summary in branch_changer.applied:
                tags.update(brush_result.fixed_lintian_tags)
            if result.merge_proposal:
                if result.is_new:
                    return JanitorResult(
                        pkg, log_id, start_time, datetime.now(),
                        'Proposed fixes %r' % tags,
                        proposal_url=result.merge_proposal.url)
                elif tags:
                    return JanitorResult(
                        pkg, log_id, start_time, datetime.now(),
                        'Updated proposal with fixes %r' % tags,
                        proposal_url=result.merge_proposal.url)
                else:
                    return JanitorResult(
                        pkg, log_id, start_time, datetime.now(),
                        'No new fixes for proposal',
                        proposal_url=result.merge_proposal.url)
            else:
                if tags:
                    return JanitorResult(
                        pkg, log_id, start_time, datetime.now(),
                        'Pushed fixes %r' % tags)
                else:
                    return JanitorResult(
                        pkg, log_id, start_time, datetime.now(),
                        'Nothing to do.')


for (vcs_url, mode, env, command) in todo:
    maintainer_email = env['MAINTAINER_EMAIL']
    if (args.max_mps_per_maintainer and
            open_mps_per_maintainer.get(maintainer_email, 0)
            >= args.max_mps_per_maintainer):
        warning(
            'Skipping %s, maximum number of open merge proposals reached '
            'for maintainer %s', env['PACKAGE'], maintainer_email)
        continue
    if mode == "attempt-push" and "salsa.debian.org/debian/" in vcs_url:
        # Make sure we don't accidentally push to unsuspecting collab-maint
        # repositories, even if debian-janitor becomes a member of "debian"
        # in the future.
        mode = "propose"
    result = process_package(vcs_url, mode, env, command)
    if result.proposal_url:
        note('%s: %s: %s', result.package, result.description,
             result.proposal_url)
    else:
        note('%s: %s', result.package, result.description)
    state.store_run(
        result.log_id, env['PACKAGE'], vcs_url, env['MAINTAINER_EMAIL'],
        result.start_time, result.finish_time, command,
        result.description, result.proposal_url)
    open_mps_per_maintainer.setdefault(maintainer_email, 0)
    open_mps_per_maintainer[maintainer_email] += 1
