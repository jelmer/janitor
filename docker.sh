#!/bin/bash
for NAME in base site runner publish archive differ worker vcs_store postgres irc_notify
do
   docker build -t eu.gcr.io/debian-janitor/$NAME -f Dockerfile_$NAME .
   docker push eu.gcr.io/debian-janitor/$NAME
done
