#!/usr/bin/python3

import os
import sys

sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402
from janitor.worker import worker_failure_from_sbuild_log  # noqa: E402

for package, log_id, result_code, description in state.iter_build_failures():
    build_log_path = os.path.join('pkg', package, log_id, 'build.log')
    failure = worker_failure_from_sbuild_log(build_log_path)
    if failure.code != result_code or description != failure.description:
        state.update_run_result(log_id, failure.code, failure.description)
