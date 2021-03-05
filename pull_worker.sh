#!/bin/bash -e

WD=$(realpath $(dirname $0))

export BRZ_PLUGINS_AT=debian@$WD/breezy-debian
export SBUILD_CONFIG=${SBUILD_CONFIG:-$WD/sbuildrc}
export AUTOPKGTEST=$WD/autopkgtest-wrapper

export PYTHONPATH=$WD/ognibuild:$WD:$WD/breezy:$WD/silver-platter:$WD/lintian-brush:$WD/dulwich:$WD/debmutate:$WD/python-debian/lib:$WD/upstream-ontologist:$WD/buildlog-consultant

if [ -n "${JANITOR_CREDENTIALS}" ];
then
   python3 -m janitor.pull_worker --credentials="${JANITOR_CREDENTIALS}" "$@"
else
   python3 -m janitor.pull_worker "$@"
fi
