#!/usr/bin/python
# Copyright (C) 2022 Jelmer Vernooij <jelmer@jelmer.uk>
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

import gpg
from aiohttp.test_utils import make_mocked_request

from janitor.publish import credentials_request


async def test_credentials_no_ssh_dir(tmp_path, monkeypatch):
    # /credentials returns an empty ssh_keys list when ~/.ssh doesn't exist.
    monkeypatch.setenv("HOME", str(tmp_path))
    request = make_mocked_request(
        "GET", "/credentials", app={"gpg": gpg.Context(armor=True)}
    )
    resp = await credentials_request(request)
    assert resp.status == 200
