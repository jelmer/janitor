#!/bin/bash -e
TODO="$@"
TODO=${TODO:-base site runner publish archive differ worker vcs_store irc_notify mastodon_notify xmpp_notify}
for NAME in $TODO
do
   SHA=$(git rev-parse HEAD)
   docker build -t ghcr.io/jelmer/janitor/$NAME:latest -t ghcr.io/jelmer/janitor/$NAME:$SHA -f Dockerfile_$NAME .
   docker push ghcr.io/jelmer/janitor/$NAME:latest
   docker push ghcr.io/jelmer/janitor/$NAME:$SHA
done
