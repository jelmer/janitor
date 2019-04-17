#!/usr/bin/python3

import os
import sys
import time
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402


sys.stdout.write("""\
Packages
========

""")


for (name, ) in state.iter_packages():
    sys.stdout.write(
        '- `%s <%s>`_\n' % (name, name))

sys.stdout.write("\n")
sys.stdout.write("*Last Updated: " + time.asctime() + "*\n")
