#!/bin/bash
if [ ! -z "$@" ]; then
    TODO="$@"
else
    TODO="base site runner publish archive differ worker vcs_store postgres irc_notify mastodon_notify xmpp_notify"
fi
for NAME in $TODO
do
   docker build -t eu.gcr.io/debian-janitor/$NAME -f Dockerfile_$NAME .
   docker push eu.gcr.io/debian-janitor/$NAME
done
