#!/usr/bin/python3

import os
import sys

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402
from janitor.build import worker_failure_from_sbuild_log  # noqa: E402
from janitor.trace import note  # noqa: E402


for package, log_id, result_code, description in state.iter_build_failures():
    build_log_path = os.path.join('site', 'pkg', package, log_id, 'build.log')
    failure = worker_failure_from_sbuild_log(build_log_path)
    if failure.stage:
        new_code = 'build-failed-stage-%s' % failure.stage
    else:
        new_code = 'build-failed'
    if new_code != result_code or description != failure.description:
        state.update_run_result(log_id, new_code, failure.description)
        note('Updated %r, %r => %r, %r', result_code, description,
             new_code, failure.description)
