#!/usr/bin/python3

import os
import sys
import time
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state


sys.stdout.write("""\
History
=======

""")



for run_id, times, command, description, package, proposal_url in state.iter_runs():
    sys.stdout.write(
        '- `%(package)s <https://packages.debian.org/%(package)s>`_: '
        'Run `%(run_id)s <pkg/%(package)s/logs/%(run_id)s>`_.\n' %
        {'run_id': run_id, 'package': package})
    sys.stdout.write('  %s\n' % description)
    if proposal_url:
        sys.stdout.write(
            '  `Merge proposal <%(proposal_url)s>`_\n' %
            {'proposal_url': proposal_url})
    sys.stdout.write('\n')

sys.stdout.write("\n")
sys.stdout.write("*Last Updated: " + time.asctime() + "*\n")
