#!/usr/bin/python3

import argparse
import os
import sys
import time
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402
from janitor.site.rst import format_table, format_duration  # noqa: E402

parser = argparse.ArgumentParser('report-history')
parser.add_argument('--limit', type=int, help='Number of entries to display',
                    default=100)
args = parser.parse_args()


sys.stdout.write("""\
History
=======

""")

if args.limit:
    sys.stdout.write('Last %d runs:\n\n' % args.limit)


header = ['Package', 'Command', 'Duration', 'Result']
data = []
for (run_id, times, command, description, package, proposal_url,
        changes_filename, build_distro, result_code) in state.iter_runs(
                limit=args.limit):
    row = [
        '`%s <pkg/%s>`_' % (package, package),
        '`%s <pkg/%s/%s/>`_.' % (
            command, package, run_id),
        '%s' % format_duration(times[1] - times[0]),
        ]
    if proposal_url:
        row.append('%s `Merge proposal <%s>`_\n' %
                   (result_code, proposal_url))
    else:
        row.append(result_code or '')
    data.append(row)


format_table(sys.stdout, header, data)

sys.stdout.write("\n")
sys.stdout.write("*Last Updated: " + time.asctime() + "*\n")
