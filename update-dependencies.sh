#!/bin/bash
DEPS="lintian-brush silver-platter breezy"
for NAME in $DEPS
do
    pushd $NAME
    git pull origin master
    popd
done
git commit -m "Update dependencies." $DEPS
