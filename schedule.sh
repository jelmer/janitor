#!/bin/bash
export PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush:$(pwd)/silver-platter:$(pwd)/breezy"
./upstream-codebases.py | python3 -m janitor.package_metadata --distribution=upstream "$@" 
./udd-package-metadata.py | python3 -m janitor.package_metadata --distribution=unstable "$@"
(
   python3 ./unchanged-candidates.py
   python3 ./upstream-unchanged-candidates.py
   python3 ./scrub-obsolete-candidates.py
   python3 ./cme-candidates.py
   python3 ./lintian-fixes-candidates.py
   python3 ./fresh-releases-candidates.py
   python3 ./fresh-snapshots-candidates.py
   python3 ./multi-arch-candidates.py
   python3 ./orphan-candidates.py
   python3 ./mia-candidates.py
   python3 ./uncommitted-candidates.py
   python3 ./debianize-candidates.py
) | python3 -m janitor.candidates "$@"
python3 -m janitor.schedule "$@"
