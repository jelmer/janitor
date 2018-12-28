#!/bin/sh
BRZ_PLUGINS_AT=propose@$(pwd)/.plugins/propose PYTHONPATH="$PYTHONPATH:$(pwd)/lintian-brush" ./silver-platter/propose-lintian-fixes.py --pre-check "test ! -f debian/control.in" --policy=policy.conf "$@"
