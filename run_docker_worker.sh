#!/bin/bash -e

docker run -v ${JANITOR_CREDENTIALS}:/credentials.json eu.gcr.io/debian-janitor/worker --credentials=/credentials.json "$@"
