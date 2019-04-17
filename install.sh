#!/bin/bash
sudo apt install brz lintian-brush brz-debian protobuf-compiler python3-github python3-gitlab python3-launchpadlib
gpg --recv-keys 6F915003D1998D6A
gpg --export 6F915003D1998D6A > ~/debian-janitor/janitor.gpg

