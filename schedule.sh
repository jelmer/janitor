#!/bin/bash
PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush:$(pwd)/silver-platter" ./schedule-lintian-fixes.py --policy=policy.conf "$@"
PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush:$(pwd)/silver-platter" ./schedule-new-upstream-releases.py --policy=policy.conf "$@"
PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush:$(pwd)/silver-platter" ./schedule-new-upstream-snapshots.py --policy=policy.conf "$@"
