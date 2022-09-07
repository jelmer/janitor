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


def override_launchpad_consumer_name():
    from breezy.forge import ForgeLoginRequired
    from breezy.plugins.launchpad import lp_api
    from launchpadlib.launchpad import Launchpad
    from launchpadlib.credentials import RequestTokenAuthorizationEngine

    class LoginRequiredAuthorizationEngine(RequestTokenAuthorizationEngine):

        def make_end_user_authorize_token(self, credentials, request_token):
            raise ForgeLoginRequired(self.web_root)


    def connect_launchpad(base_url, timeout=None, proxy_info=None,
                          version=Launchpad.DEFAULT_VERSION):
        cache_directory = lp_api.get_cache_directory()
        credential_store = lp_api.BreezyCredentialStore()
        authorization_engine = LoginRequiredAuthorizationEngine(
            base_url, consumer_name='Janitor')
        return Launchpad.login_with(
            'Janitor', base_url, cache_directory, timeout=timeout,
            credential_store=credential_store,
            authorization_engine=authorization_engine,
            version=version)


    lp_api.connect_launchpad = connect_launchpad
