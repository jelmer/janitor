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

import os
import socket
import subprocess
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
    )
from silver_platter.debian.schedule import schedule

from breezy import (
    errors,
    )

from breezy.branch import Branch
from breezy.trace import note

from breezy.plugins.propose.propose import (
    NoSuchProject,
    UnsupportedHoster,
    )

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
                    default=['file-contains-trailing-whitespace'])
parser.add_argument('--pre-check',
                    help='Command to run to check whether to process package.',
                    type=str)
parser.add_argument('--post-check',
                    help='Command to run to check package before pushing.',
                    type=str)
parser.add_argument('--build-verify',
                    help='Build package to verify it.', action='store_true')
parser.add_argument('--shuffle',
                    help='Shuffle order in which packages are processed.',
                    action='store_true')
parser.add_argument('--refresh',
                    help='Discard old branch and apply fixers from scratch.',
                    action='store_true')
parser.add_argument('--log-dir',
                    help='Directory to store logs in.',
                    type=str, default='public_html/pkg')
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


dry_run = args.dry_run

fixer_scripts = {}
for fixer in available_lintian_fixers():
    for tag in fixer.lintian_tags:
        fixer_scripts[tag] = fixer

possible_transports = []
possible_hosters = []

todo = schedule(
    args.lintian_log, args.policy, args.propose_addon_only, args.packages,
    args.fixers, args.shuffle)

subparser = argparse.ArgumentParser(prog='lintian-brush')
subparser.add_argument("fixers", nargs='*')
subparser.add_argument(
    '--no-update-changelog', action="store_false", default=None,
    dest="update_changelog", help="do not update the changelog")
subparser.add_argument(
    '--update-changelog', action="store_true", dest="update_changelog",
    help="force updating of the changelog", default=None)


class JanitorLintianFixer(LintianFixer):
    """Janitor-specific Lintian Fixer."""

    def __init__(self, pkg, fixers, update_changelog,
                 pre_check=None, post_check=None, propose_addon_only=None,
                 committer=None, log_id=None):
        super(JanitorLintianFixer, self).__init__(
            pkg, fixers=fixers, update_changelog=update_changelog,
            pre_check=pre_check, post_check=post_check,
            propose_addon_only=propose_addon_only, committer=committer)
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


for (vcs_url, mode, env, command) in todo:
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
                note('%r: post-check failed, skipping', pkg)
                return False
        if args.build_verify:
            with open(os.path.join(log_path, 'build.log'), 'w') as f:
                subprocess.check_call(
                    'brz bd --builder=\'sbuild -v\'', shell=True,
                    cwd=local_tree.basedir,
                    stdout=f, stderr=f)
        return True

    note('Processing: %s', pkg)

    try:
        main_branch = Branch.open(
                vcs_url, possible_transports=possible_transports)
    except socket.error:
        note('%s: ignoring, socket error', pkg)
    except errors.NotBranchError as e:
        note('%s: Branch does not exist: %s', pkg, e)
    except errors.UnsupportedProtocol:
        note('%s: Branch available over unsupported protocol', pkg)
    except errors.ConnectionError as e:
        note('%s: %s', pkg, e)
    except errors.PermissionDenied as e:
        note('%s: %s', pkg, e)
    except errors.InvalidHttpResponse as e:
        note('%s: %s', pkg, e)
    except errors.TransportError as e:
        note('%s: %s', pkg, e)
    else:
        branch_changer = JanitorLintianFixer(
                pkg, fixers=[fixer_scripts[fixer] for fixer in subargs.fixers],
                update_changelog=subargs.update_changelog,
                pre_check=pre_check, post_check=post_check,
                propose_addon_only=args.propose_addon_only,
                committer=committer, log_id=log_id)
        try:
            proposal, is_new = propose_or_push(
                    main_branch, "lintian-fixes", branch_changer, mode,
                    possible_transports=possible_transports,
                    possible_hosters=possible_hosters,
                    refresh=args.refresh)
        except UnsupportedHoster:
            note('%s: Hoster unsupported', pkg)
            continue
        except NoSuchProject as e:
            note('%s: project %s was not found', pkg, e.project)
            continue
        except BuildFailedError:
            note('%s: build failed', pkg)
            continue
        except MissingUpstreamTarball:
            note('%s: unable to find upstream source', pkg)
            continue
        except errors.PermissionDenied as e:
            note('%s: %s', pkg, e)
            continue
        except PostCheckFailed as e:
            note('%s: %s', pkg, e)
            continue
        else:
            if proposal:
                tags = set()
                for result, unused_summary in branch_changer.applied:
                    tags.update(result.fixed_lintian_tags)
                if is_new:
                    note('%s: Proposed fixes %r: %s', pkg, tags, proposal.url)
                elif tags:
                    note('%s: Updated proposal %s with fixes %r', pkg,
                         proposal.url, tags)
                else:
                    note('%s: No new fixes for proposal %s', pkg, proposal.url)
