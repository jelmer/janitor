#!/usr/bin/python3

import argparse
import os
import sys

from debian.changelog import Version
from jinja2 import Environment, FileSystemLoader, select_autoescape

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state, udd  # noqa: E402

env = Environment(
    loader=FileSystemLoader('templates'),
    autoescape=select_autoescape(['html', 'xml'])
)


parser = argparse.ArgumentParser(prog='report-apt-repo')
parser.add_argument("suite")
args = parser.parse_args()


def get_unstable_versions(present):
    unstable = {}
    if present:
        for package in udd.UDD.public_udd_mirror().get_source_packages(
                packages=list(present), release='sid'):
            unstable[package.name] = Version(package.version)
    return unstable


def gather_package_list():
    present = {}
    for source, version in state.iter_published_packages(args.suite):
        present[source] = Version(version)

    unstable = get_unstable_versions(present)

    for source in sorted(present):
        yield (
            source,
            present[source].upstream_version,
            unstable[source].upstream_version
            if source in unstable else '')


template = env.get_template(args.suite + '.html')
sys.stdout.write(template.render(packages=gather_package_list()))
