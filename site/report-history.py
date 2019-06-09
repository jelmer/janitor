#!/usr/bin/python3

import argparse
import os
import sys
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402
from janitor.site import format_duration  # noqa: E402
from jinja2 import Environment, FileSystemLoader, select_autoescape  # noqa: E402
env = Environment(
    loader=FileSystemLoader('templates'),
    autoescape=select_autoescape(['html', 'xml'])
)

parser = argparse.ArgumentParser('report-history')
parser.add_argument('--limit', type=int, help='Number of entries to display',
                    default=100)
args = parser.parse_args()


header = ['Package', 'Command', 'Duration', 'Result']
data = []
for (run_id, times, command, description, package, proposal_url,
        changes_filename, build_distro, result_code,
        branch_name) in state.iter_runs(limit=args.limit):
    row = [
        package,
        command,
        times[1] - times[0],
        run_id,
        result_code,
        proposal_url,
        ]
    data.append(row)


template = env.get_template('history.html')
sys.stdout.write(template.render(count=args.limit, history=data, format_duration=format_duration))
