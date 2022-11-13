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

__all__ = [
    "Config",
    "Campaign",
    "read_config",
    "get_campaign_config",
]

from google.protobuf import text_format  # type: ignore

from . import config_pb2

Config = config_pb2.Config
Campaign = config_pb2.Campaign


def read_config(f):
    return text_format.Parse(f.read(), config_pb2.Config())


def get_distribution(config: Config, name: str) -> config_pb2.Distribution:
    for d in config.distribution:
        if d.name == name:
            return d
    raise KeyError(name)


def get_campaign_config(config: config_pb2.Config, name: str
                        ) -> config_pb2.Campaign:
    for c in config.campaign:
        if c.name == name:
            return c
    raise KeyError(name)


def setup_redis(app):
    from redis.asyncio import Redis

    async def connect_redis(app):
        app['redis'] = Redis.from_url(app['config'].redis_location)

    async def disconnect_redis(app):
        await app['redis'].close()

    app.on_startup.append(connect_redis)
    app.on_cleanup.append(disconnect_redis)


def setup_gpg(app):
    import tempfile
    import gpg
    async def start_gpg_context(app):
        gpg_home = tempfile.TemporaryDirectory()
        gpg_home.__enter__()
        gpg_context = gpg.Context(home_dir=gpg_home.name)
        app['gpg'] = gpg_context.__enter__()

        async def cleanup_gpg(app):
            gpg_context.__exit__(None, None, None)
            gpg_home.__exit__(None, None, None)

        app.on_cleanup.append(cleanup_gpg)
    app.on_startup.append(start_gpg_context)


def setup_postgres(app):
    from .state import create_pool
    async def connect_postgres(app):
        database = await state.create_pool(app['config'].database_location)
        app.database = database
        app['pool'] = database

    app.on_startup.append(connect_postgres)


def setup_logfile_manager(app, trace_configs=None):
    from .logs import get_log_manager
    async def startup_logfile_manager(app):
        app.logfile_manager = get_log_manager(
            app['config'].logs_location, trace_configs=trace_configs)

    app.on_startup.append(startup_logfile_manager)


def setup_artifact_manager(app, trace_configs=None):
    from janitor.artifacts import get_artifact_manager

    async def startup_artifact_manager(app):
        app['artifact_manager'] = get_artifact_manager(
            app['config'].artifact_location, trace_configs=trace_configs)
        await app['artifact_manager'].__aenter__()

    async def turndown_artifact_manager(app):
        await app['artifact_manager'].__aexit__(None, None, None)

    app.on_startup.append(startup_artifact_manager)
    app.on_cleanup.append(turndown_artifact_manager)


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument('config_file', type=str, help='Configuration file to read')
    args = parser.parse_args()
    with open(args.config_file, 'r') as f:
        config = read_config(f)
