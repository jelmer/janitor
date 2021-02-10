#!/usr/bin/python3
# Copyright (C) 2019 Jelmer Vernooij <jelmer@jelmer.uk>
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

import asyncio
from io import BytesIO, StringIO
import os
import json
import sys

from .trace import note

from breezy.patches import (
    iter_hunks,
    InsertLine,
    RemoveLine,
    ContextLine,
)


class DiffoscopeError(Exception):
    """An error occurred while running diffoscope."""


def filter_boring_udiff(udiff, old_version, new_version, display_version):
    old_version = old_version.encode("utf-8")
    new_version = new_version.encode("utf-8")
    display_version = display_version.encode("utf-8")
    lines = iter([line.encode("utf-8") for line in udiff.splitlines(True)])
    hunks = []
    oldlines = []
    newlines = []
    for hunk in iter_hunks(lines, allow_dirty=False):
        for line in hunk.lines:
            if isinstance(line, RemoveLine):
                line.contents = line.contents.replace(old_version, display_version)
                oldlines.append(line.contents)
            elif isinstance(line, InsertLine):
                line.contents = line.contents.replace(new_version, display_version)
                newlines.append(line.contents)
            elif isinstance(line, ContextLine):
                oldlines.append(line.contents)
                newlines.append(line.contents)
        hunks.append(hunk)
    if oldlines == newlines:
        return None
    return "".join([hunk.as_bytes().decode("utf-8", "replace")])


def filter_boring_detail(detail, old_version, new_version, display_version):
    if detail["unified_diff"] is not None:
        detail["unified_diff"] = filter_boring_udiff(
            detail["unified_diff"], old_version, new_version, display_version
        )
    detail["source1"] = detail["source1"].replace(old_version, display_version)
    detail["source2"] = detail["source2"].replace(new_version, display_version)
    if detail.get("details"):
        i = 0
        for subdetail in list(detail["details"]):
            if not filter_boring_detail(
                subdetail, old_version, new_version, display_version
            ):
                del detail["details"][i]
                continue
            i += 1
    if not detail.get("unified_diff") and not detail.get("details"):
        return False
    return True


def filter_boring(diff, old_version, new_version, old_suite, new_suite):
    display_version = new_version.rsplit("~", 1)[0]
    # Changes file differences
    BORING_FIELDS = ["Date", "Distribution", "Version"]
    i = 0
    for detail in list(diff["details"]):
        if detail["source1"] in BORING_FIELDS and detail["source2"] in BORING_FIELDS:
            del diff["details"][i]
            continue
        if detail["source1"].endswith(".buildinfo") and detail["source2"].endswith(
            ".buildinfo"
        ):
            del diff["details"][i]
            continue
        if not filter_boring_detail(detail, old_version, new_version, display_version):
            del diff["details"][i]
            continue
        i += 1


def filter_irrelevant(diff):
    diff["source1"] = os.path.basename(diff["source1"])
    diff["source2"] = os.path.basename(diff["source2"])


async def format_diffoscope(
    root_difference, content_type, title, jquery_url=None, css_url=None
):
    if content_type == "application/json":
        return json.dumps(root_difference)
    from diffoscope.readers.json import JSONReaderV1

    root_difference = JSONReaderV1().load_rec(root_difference)
    if content_type == "text/html":
        from diffoscope.presenters.html.html import HTMLPresenter

        p = HTMLPresenter()
        old_stdout = sys.stdout
        sys.stdout = f = StringIO()
        old_argv = sys.argv
        sys.argv = title.split(" ")
        try:
            p.output_html("-", root_difference, jquery_url=jquery_url, css_url=css_url)
        finally:
            sys.stdout = old_stdout
            sys.argv = old_argv
        return f.getvalue()
    if content_type == "text/markdown":
        from diffoscope.presenters.markdown import MarkdownTextPresenter

        out = []

        def printfn(t=""):
            out.append(t + "\n")

        p = MarkdownTextPresenter(printfn)
        p.start(root_difference)
        return "".join(out)
    if content_type == "text/plain":
        from diffoscope.presenters.text import TextPresenter

        out = []

        def printfn(t=""):
            out.append(t + "\n")

        p = TextPresenter(printfn, False)
        p.start(root_difference)
        return "".join(out)
    raise AssertionError("unknown content type %r" % content_type)


async def _run_diffoscope(old_binary, new_binary, preexec_fn=None):
    args = ["diffoscope", "--json=-", "--exclude-directory-metadata=yes"]
    args.extend([old_binary, new_binary])
    stdout = BytesIO()
    note("running %r", args)
    p = await asyncio.create_subprocess_exec(
        *args,
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        preexec_fn=preexec_fn
    )
    stdout, stderr = await p.communicate(b"")
    if p.returncode == 0:
        return None
    if p.returncode != 1:
        raise DiffoscopeError(stderr.decode(errors="replace"))
    try:
        return json.loads(stdout.decode("utf-8"))
    except json.JSONDecodeError as e:
        raise DiffoscopeError("Error parsing JSON: %s" % e)


async def run_diffoscope(old_binaries, new_binaries, preexec_fn=None):
    ret = {
        "diffoscope-json-version": 1,
        "source1": "old version",
        "source2": "new version",
        "unified_diff": None,
        "details": [],
    }

    for (old_name, old_path), (new_name, new_path) in zip(old_binaries, new_binaries):
        sub = await _run_diffoscope(old_path, new_path, preexec_fn=preexec_fn)
        if sub is None:
            continue
        sub["source1"] = old_name
        sub["source2"] = new_name
        del sub["diffoscope-json-version"]
        ret["details"].append(sub)
    return ret
