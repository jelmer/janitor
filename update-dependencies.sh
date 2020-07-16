#!/bin/bash
DEPS="lintian-brush silver-platter breezy dulwich breezy-debian python-debian debmutate"
for NAME in $DEPS
do
    pushd $NAME
    git pull
    popd
done
git commit -m "Update dependencies." $DEPS
