#!/usr/bin/python3

import argparse
import sys
import os

from jinja2 import Environment, FileSystemLoader, select_autoescape

sys.path.insert(0, os.path.dirname(__file__))

parser = argparse.ArgumentParser('report-state')
parser.add_argument('name', nargs='?', type=str, default=None)
args = parser.parse_args()

from janitor import state  # noqa: E402

env = Environment(
    loader=FileSystemLoader('templates'),
    autoescape=select_autoescape(['html', 'xml'])
)


proposals_by_status = {}
for url, status, package in state.iter_all_proposals(branch_name=args.name):
    proposals_by_status.setdefault(status, []).append(url)


template = env.get_template('merge-proposals.html')
sys.stdout.write(template.render(
        open_proposals=proposals_by_status.get('open', []),
        merged_proposals=proposals_by_status.get('merged', []),
        closed_proposals=proposals_by_status.get('closed', [])))
