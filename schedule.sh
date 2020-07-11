#!/bin/bash
export PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush:$(pwd)/silver-platter:$(pwd)/breezy"
./udd-package-metadata.py | python3 -m janitor.package_metadata "$@"
(
   python3 ./unchanged-candidates.py
   python3 ./lintian-fixes-candidates.py
   python3 ./fresh-releases-candidates.py
   python3 ./fresh-snapshots-candidates.py
   python3 ./multi-arch-candidates.py
   python3 ./orphan-candidates.py
   python3 ./uncommitted-candidates.py
) | python3 -m janitor.candidates "$@"
python3 -m janitor.schedule --policy=policy.conf "$@"
