#!/bin/bash
PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush:$(pwd)/silver-platter:$(pwd)/breezy" python3 -m janitor.udd "$@"
PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush:$(pwd)/silver-platter:$(pwd)/breezy" python3 -m janitor.schedule --policy=policy.conf "$@"
