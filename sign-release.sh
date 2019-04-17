#!/bin/bash
set -e

rm -f Release.gpg.tmp InRelease.tmp
echo "$PASSPHRASE" | gpg --no-tty --batch --detach-sign -o Release.gpg.tmp "$1"
mv Release.gpg.tmp Release.gpg
echo "$PASSPHRASE" | gpg --no-tty --batch --clearsign -o InRelease.tmp "$1"
mv InRelease.tmp InRelease

