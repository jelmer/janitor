#!/usr/bin/python3

import asyncpg
import operator


async def stats_by_result_codes(conn: asyncpg.Connection, suite=None):

