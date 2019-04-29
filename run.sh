#!/bin/bash
PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush:$(pwd)/silver-platter" python3 -m janitor.runner "$@"
