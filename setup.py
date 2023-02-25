#!/usr/bin/python3
from setuptools import setup
from setuptools_protobuf import Protobuf

setup(protobufs=[
    Protobuf('janitor/config.proto', mypy=True),
])
