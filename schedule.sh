#!/bin/bash
export PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush:$(pwd)/silver-platter:$(pwd)/breezy"
./udd-package-metadata.py | python3 -m janitor.package_metadata "$@"
python3 -m janitor.candidates "$@"
python3 -m janitor.schedule --policy=policy.conf "$@"
