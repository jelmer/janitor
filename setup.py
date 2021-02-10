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

setup(
    name="debian-janitor",
    author="Jelmer Vernooij",
    author_email="jelmer@jelmer.uk",
    url="https://salsa.debian.org/jelmer/debian-janitor",
    description="Manager for automatic VCS changes",
    version="0.0.1",
    license="GNU GPL v2 or later",
    keywords="debian git bzr vcs github gitlab launchpad",
    packages=[
        "janitor",
        "janitor.tests",
        "janitor.site",
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
        ],
    },
    test_suite="janitor.tests.test_suite",
    install_requires=[
        "lintian-brush",
        "breezy",
        "jinja2",
    ],
)
