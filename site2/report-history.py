#!/usr/bin/python3

import argparse
import os
import sys
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402
from janitor.site.rst import format_duration  # noqa: E402
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
        '<a href="/pkg/%s">%s</a>' % (package, package),
        command,
        '%s' % format_duration(times[1] - times[0]),
        ]
    if proposal_url:
        row.append('%s <a href="%s">Merge proposal</a>' %
                   (result_code, proposal_url))
    else:
        row.append('<a href="/pkg/%s/%s/">%s</a>' % (
            package, run_id, result_code))
    data.append(row)


template = env.get_template('history.html')
sys.stdout.write(template.render(count=args.limit, history=data))
