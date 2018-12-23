#!/bin/sh -x
# TODO(jelmer): Create a Debian package for this
if [ ! -d .plugins ]; then
    mkdir .plugins
    brz branch lp:brz-propose .plugins/propose
fi
brz pull -d ~/src/brz-propose/trunk https://code.breezy-vcs.org/brz-propose/trunk --overwrite
if [ ! -d lintian-brush ]; then
    brz branch https://salsa.debian.org/jelmer/lintian-brush
else
    brz pull -d lintian-brush
fi
if [ ! -d silver-platter ]; then
    brz branch bzr+ssh://rhonwyn/srv/bzr/silver-platter/trunk silver-platter
else
    brz pull -d silver-platter
fi
