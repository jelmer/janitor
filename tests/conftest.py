import importlib.resources
from typing import AsyncGenerator

import asyncpg
import pytest_asyncio

import testing.postgresql

from janitor.state import create_pool

pytest_plugins = ["aiohttp"]


@pytest_asyncio.fixture()
async def db():
    with testing.postgresql.Postgresql() as postgresql:
        conn = await asyncpg.connect(postgresql.url())
        try:
            with importlib.resources.open_text('janitor', 'state.sql') as f:
                await conn.execute(f.read())
            with importlib.resources.open_text('janitor.debian', 'debian.sql') as f:
                await conn.execute(f.read())
        finally:
            await conn.close()

        db = await create_pool(postgresql.url())

        yield db

        await db.close()


@pytest_asyncio.fixture()
async def con(db: asyncpg.Pool) -> AsyncGenerator[asyncpg.Connection, None]:
    async with db.acquire() as con:
        yield con


async def test_db_returns_janitor_db(db) -> None:
    assert isinstance(db, asyncpg.Pool)
