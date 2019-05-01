#!/usr/bin/python3

import argparse
import operator
import os
import sys
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402
from janitor.site import format_rst_table  # noqa: E402

parser = argparse.ArgumentParser(prog='report-result-codes')
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
    with open('result-codes/%s-list.rst' % code, 'w') as f:
        format_rst_table(f, header, data)


with open('result-codes/index.rst', 'w') as f:
    header = ['Code', 'Count']
    data = sorted(
        [[name, len(by_code[name])] for name in by_code],
        key=operator.itemgetter(1))
    format_rst_table(f, header, data)
