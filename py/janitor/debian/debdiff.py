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

from .._common import debdiff  # type: ignore

debdiff_is_empty = debdiff.debdiff_is_empty  # type: ignore
filter_boring = debdiff.filter_boring  # type: ignore
section_is_wdiff = debdiff.section_is_wdiff  # type: ignore
markdownify_debdiff = debdiff.markdownify_debdiff  # type: ignore
htmlize_debdiff = debdiff.htmlize_debdiff  # type: ignore
DebdiffError = debdiff.DebdiffError  # type: ignore
run_debdiff = debdiff.run_debdiff  # type: ignore
