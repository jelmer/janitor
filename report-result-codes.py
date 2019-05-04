#!/usr/bin/python3

import argparse
import operator
import os
import sys
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402
from janitor.site.rst import format_table  # noqa: E402

parser = argparse.ArgumentParser(prog='report-result-codes')
parser.add_argument(
    'path', type=str, default='result-codes', help='Output path')
args = parser.parse_args()

by_code = {}

for (source, command, result_code, log_id,
     description) in state.iter_last_runs():
    by_code.setdefault(result_code, []).append(
        (source, command, log_id, description))


for code, items in by_code.items():
    data = []
    header = ['Package', 'Command', 'Description']
    for (source, command, log_id, description) in items:
        data.append((
            '`%s </pkg/%s/>`_' % (source, source),
            command,
            '`%s </pkg/%s/%s>`_' % (description, source, log_id)))
    with open(os.path.join(args.path, '%s-list.rst' % code), 'w') as f:
        format_table(f, header, data)


with open(os.path.join(args.path, 'index.rst'), 'w') as f:
    header = ['Code', 'Count']

    def label(n):
        if os.path.exists(os.path.join(args.path, "%s.rst" % n)):
            return "`%s <%s.html>`_" % (n, n)
        else:
            return "`%s <%s-list.html>`_" % (n, n)
    data = sorted(
        [[label(name), len(by_code[name])] for name in by_code],
        key=operator.itemgetter(1), reverse=True)
    format_table(f, header, data)
