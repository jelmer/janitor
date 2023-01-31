import importlib.resources
from typing import Type

import asyncpg
import asyncpg_engine
import pytest
import testing.postgresql

from janitor.state import init_types

pytest_plugins = ["asyncpg_engine", "aiohttp"]


@pytest.fixture()
async def postgres_url():
    with testing.postgresql.Postgresql() as postgresql:
        conn = await asyncpg.connect(postgresql.url())
        try:
            with importlib.resources.open_text('janitor', 'state.sql') as f:
                await conn.execute(f.read())
            with importlib.resources.open_text('janitor.debian', 'debian.sql') as f:
                await conn.execute(f.read())
        finally:
            await conn.close()
        yield postgresql.url()


class JanitorEngine(asyncpg_engine.Engine):

    @staticmethod
    async def _set_codecs(con: asyncpg.Connection) -> None:
        await init_types(con)


@pytest.fixture()
def asyncpg_engine_cls() -> Type[JanitorEngine]:
    return JanitorEngine


async def test_returns_janitor_engine(db: JanitorEngine) -> None:
    assert isinstance(db, JanitorEngine)
