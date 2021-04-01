#!/usr/bin/python3

import asyncpg


async def get_published_packages(conn: asyncpg.Connection, suite):
    return await conn.fetch("""
select distinct on (package.name) package.name, debian_build.version,
archive_version from debian_build
left join package on package.name = debian_build.source
where debian_build.distribution = $1 and not package.removed
order by package.name, debian_build.version desc
""", suite)
