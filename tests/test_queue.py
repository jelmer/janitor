from janitor.queue import Queue


async def test_get_buckets(con):
    queue = Queue(con)
    assert await queue.get_buckets() == []


async def test_add(con):
    queue = Queue(con)
    await con.execute("INSERT INTO codebase (name) VALUES ('foo')")
    assert await queue.add(codebase="foo", campaign="bar", command="true") == (1, 'default')
    queue_item, vcs_info = await queue.next_item()
    assert queue_item.codebase == 'foo'
    assert queue_item.campaign == 'bar'


async def test_double_add(con):
    queue = Queue(con)
    await con.execute("INSERT INTO codebase (name) VALUES ('foo')")
    assert await queue.add(codebase="foo", campaign="bar", command="true") == (1, 'default')
    assert await queue.add(codebase="foo", campaign="bar", command="true") == (1, 'default')
