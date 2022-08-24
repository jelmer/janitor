#!/bin/bash -e

WD=$(realpath $(dirname $0))

export BRZ_PLUGINS_AT=debian@$WD/breezy-debian
export SBUILD_CONFIG=${SBUILD_CONFIG:-$WD/sbuildrc}
export AUTOPKGTEST=$WD/autopkgtest-wrapper

export PATH=$WD/debmutate/scripts:$WD/breezy-debian/scripts:$PATH
export PYTHONPATH=$WD/ognibuild:$WD:$WD/breezy:$WD/silver-platter:$WD/lintian-brush:$WD/dulwich:$WD/debmutate:$WD/python-debian/lib:$WD/upstream-ontologist:$WD/buildlog-consultant

python3 -m janitor.worker --tee "$@"
