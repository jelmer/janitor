#!/bin/bash
PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush:$(pwd)/silver-platter" python3 -m janitor.runner --pre-check "test ! -f debian/control.in" "$@"
