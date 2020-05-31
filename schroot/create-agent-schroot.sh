#!/bin/bash

sudo sbuild-createchroot --chroot-prefix jenkins --include=eatmydata,ccache,gnupg $SUITE /srv/chroot/jenkins http://deb.debian.org/debian --extra-repository="deb http://deb.debian.org/debian $SUITE-backports main"

sudo chroot /srv/chroot/jenkins ./setup-agent.sh
