#!/bin/sh
PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush:$(pwd)/silver-platter" python3 -m janitor.vcs_mirror --delay=30 "$@"
