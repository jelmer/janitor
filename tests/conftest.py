from collections.abc import AsyncGenerator
from pathlib import Path

import asyncpg
import pytest_asyncio
import testing.postgresql

from janitor.state import create_pool

pytest_plugins = ["aiohttp"]

_SCHEMA_DIR = Path(__file__).resolve().parent.parent / "schema"


@pytest_asyncio.fixture()
async def db():
    with testing.postgresql.Postgresql() as postgresql:
        conn = await asyncpg.connect(postgresql.url())
        try:
            await conn.execute((_SCHEMA_DIR / "state.sql").read_text())
            await conn.execute((_SCHEMA_DIR / "debian" / "debian.sql").read_text())
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
