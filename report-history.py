#!/usr/bin/python3

import os
import sys
import time
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402


sys.stdout.write("""\
History
=======

""")


for (run_id, times, command, description, package, proposal_url,
        changes_filename, build_distro, result_code) in state.iter_runs():
    sys.stdout.write(
        '- `%(package)s <pkg/%(package)s>`_: '
        'Run `%(command)s <pkg/%(package)s/%(run_id)s/>`_.' %
        {'run_id': run_id,
         'package': package,
         'command': command.split(' ')[0]})
    if result_code:
        sys.stdout.write(' => %s' % result_code)
    sys.stdout.write('\n')
    if proposal_url:
        sys.stdout.write(
            '  `Merge proposal <%(proposal_url)s>`_\n' %
            {'proposal_url': proposal_url})
    sys.stdout.write('\n')

sys.stdout.write("\n")
sys.stdout.write("*Last Updated: " + time.asctime() + "*\n")
