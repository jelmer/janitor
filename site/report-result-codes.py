#!/usr/bin/python3

import argparse
import operator
import os
import sys

from jinja2 import Environment, FileSystemLoader, select_autoescape

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402

parser = argparse.ArgumentParser(prog='report-result-codes')
parser.add_argument(
    'path', type=str, default='result-codes', help='Output path')
args = parser.parse_args()

env = Environment(
    loader=FileSystemLoader('templates'),
    autoescape=select_autoescape(['html', 'xml'])
)

by_code = {}

for (source, command, result_code, log_id,
     description, duration) in state.iter_last_runs():
    by_code.setdefault(result_code, []).append(
        (source, command, log_id, description))

template = env.get_template('result-code.html')

for code, items in by_code.items():
    data = []
    for (source, command, log_id, description) in items:
        data.append((
            source,
            log_id,
            command,
            description))
    with open(os.path.join(args.path, '%s.html' % code), 'w') as f:
        f.write(template.render(code=code, entries=data))


with open(os.path.join(args.path, 'index.html'), 'w') as f:
    template = env.get_template('result-code-index.html')

    data = sorted(
        [[name, len(by_code[name])] for name in by_code],
        key=operator.itemgetter(1), reverse=True)
    f.write(template.render(result_codes=data))
