import pytest
from janitor.queue import Queue


async def test_get_buckets(con):
    queue = Queue(con)
    assert await queue.get_buckets() == []
