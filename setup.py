#!/usr/bin/python3
from setuptools import setup
import setuptools.command.build
setuptools.command.build.build.sub_commands.insert(
    0, ('build_proto', lambda x: True))
setup()
