#!/bin/bash -e
TODO="$@"
TODO=${TODO:-base site runner publish archive worker git_store bzr_store irc_notify mastodon_notify xmpp_notify differ}
for NAME in $TODO
do
   SHA=$(git rev-parse HEAD)
   buildah build -t ghcr.io/jelmer/janitor/$NAME:latest -t ghcr.io/jelmer/janitor/$NAME:$SHA -f Dockerfile_$NAME .
   buildah push ghcr.io/jelmer/janitor/$NAME:latest
   buildah push ghcr.io/jelmer/janitor/$NAME:$SHA
done
