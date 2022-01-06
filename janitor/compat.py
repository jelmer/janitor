#!/usr/bin/python3
# Copyright (C) 2021 Jelmer Vernooij <jelmer@jelmer.uk>
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

"""Backwards compatibility."""

import functools
import shlex

# Backwards compatibility for python < 3.8
try:
    shlex_join = shlex.join  # type: ignore
except AttributeError:
    def shlex_join(args):
        return ' '.join(shlex.quote(arg) for arg in args)


try:
    from asyncio import to_thread  # type: ignore
except ImportError:  # python < 3.8
    from asyncio import events
    import contextvars

    async def to_thread(func, *args, **kwargs):  # type: ignore
        loop = events.get_running_loop()
        ctx = contextvars.copy_context()
        func_call = functools.partial(ctx.run, func, *args, **kwargs)
        return await loop.run_in_executor(None, func_call)

