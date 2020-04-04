#!/bin/bash -e
PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush:$(pwd)/silver-platter:$(pwd)/breezy" python3 -m janitor.runner "$@"
