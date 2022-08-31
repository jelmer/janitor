#!/bin/bash
DEPS="lintian-brush silver-platter breezy dulwich breezy-debian debmutate ognibuild upstream-ontologist buildlog-consultant"
for NAME in $DEPS
do
    pushd $NAME
    if [ "$NAME" = "breezy" ] ; then
	    git pull origin master
	    git push origin HEAD:janitor
    elif [ "$NAME" = "ognibuild" ]; then
        git pull origin main
    else
       git pull origin master
    fi
    popd
done
git commit -m "Update dependencies." $DEPS
