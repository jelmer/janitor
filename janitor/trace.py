#!/usr/bin/python
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

__all__ = [
    "mutter",
    "warning",
]

import sys

from breezy.ui import (
    NoninteractiveUIFactory,
    NullOutputStream,
)


class JanitorUIFactory(NoninteractiveUIFactory):
    """UI Factory implementation for the janitor."""

    def note(self, msg):
        sys.stdout.write(msg + "\n")

    def get_username(self, prompt, **kwargs):
        return None

    def get_password(self, prompt=u"", **kwargs):
        return None

    def _make_output_stream_explicit(self, encoding, encoding_type):
        return NullOutputStream(encoding)

    def show_error(self, msg):
        sys.stderr.write("error: %s\n" % msg)

    def show_message(self, msg):
        self.note(msg)

    def show_warning(self, msg):
        sys.stderr.write("warning: %s\n" % msg)


import breezy  # noqa: E402

breezy.ui.ui_factory = JanitorUIFactory()
if not breezy._global_state:
    breezy.initialize(setup_ui=False)
    import breezy.ui

from breezy.trace import mutter, warning  # noqa: E402
