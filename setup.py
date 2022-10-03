#!/usr/bin/python3
# Copyright (C) 2019 Jelmer Vernooij
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

from setuptools import setup


debian_requires = [
    "python-apt@git+https://salsa.debian.org/apt-team/python-apt",
    "python_debian",
    'debmutate@git+https://salsa.debian.org/jelmer/debmutate',
    'silver-platter[debian]@git+https://github.com/jelmer/silver-platter',
    'ognibuild[debian,dep-server]@git+https://github.com/jelmer/ognibuild',
    "brz-debian@git+https://github.com/breezy-team/breezy-debian",
    # "brz-debian@bzr+https://code.launchpad.net/brz-debian",
]

setup(
    name="janitor",
    author="Jelmer Vernooij",
    author_email="jelmer@jelmer.uk",
    url="https://github.com/jelmer/janitor",
    description="Manager for automatic VCS changes",
    version="0.0.1",
    license="GNU GPL v2 or later",
    keywords="debian git bzr vcs github gitlab launchpad",
    packages=[
        "janitor",
        "janitor.tests",
        "janitor.site",
        "janitor_client",
    ],
    classifiers=[
        "Development Status :: 3 - Alpha",
        "License :: OSI Approved :: GNU General Public License (GPL)",
        "Programming Language :: Python :: 3.6",
        "Programming Language :: Python :: 3.7",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: Implementation :: CPython",
        "Programming Language :: Python :: Implementation :: PyPy",
        "Operating System :: POSIX",
        "Topic :: Software Development :: Version Control",
    ],
    entry_points={
        "console_scripts": [
            "janitor-runner=janitor.run:main",
            "janitor-worker=janitor.worker:main",
            "janitor-publisher=janitor.publish:main",
            "janitor-apt=janitor.debian.archive:main",
        ],
    },
    extras_require={
        'dev': [
            "flake8",
            "djlint",
            "mypy",
            "testtools",
            "pytest",
            "pytest-cov",
            "mypy-protobuf",
            "python-subunit",
            "types-PyYAML",
            "types-protobuf",
        ] + debian_requires,
        'debian': debian_requires,
        'gcp': ['gcloud-aio-storage', 'google-cloud-logging'],
        'git': ['klaus', 'aiohttp-wsgi'],
        'bzr': ['loggerhead'],
    },
    install_requires=[
        "aiohttp",
        "aiohttp_jinja2",
        "aioredis<2.0",
        "aiozipkin",
        "asyncpg",
        "backoff",
        "pygments",
        "lintian-brush@git+https://salsa.debian.org/jelmer/lintian-brush",
        "breezy[cext,git,launchpad,workspace,pgp]@git+https://github.com/breezy-team/breezy",
        # "breezy@bzr+https://code.launchpad.net/brz",
        "jinja2",
        "ognibuild@git+https://github.com/jelmer/ognibuild",
        "buildlog-consultant@git+https://github.com/jelmer/buildlog-consultant",
        "upstream-ontologist@git+https://github.com/jelmer/upstream-ontologist",
        "silver-platter@git+https://github.com/jelmer/silver-platter",
        "aiohttp-openmetrics",
    ],
)
