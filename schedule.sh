#!/bin/bash
PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush:$(pwd)/silver-platter" ./schedule-lintian-fixes.py --policy=policy.conf "$@"
PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush:$(pwd)/silver-platter" ./schedule-new-upstreams.py --policy=policy.conf "$@"
