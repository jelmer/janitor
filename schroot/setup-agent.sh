#!/bin/bash

INCLUDE="cython3 devscripts python3-aiohttp python3-all-dev python3-certifi python3-configobj python3-debian python3-dulwich python3-distro-info python3-fastimport python3-iniparse python3-launchpadlib python3-levenshtein python3-paramiko python3-patiencediff python3-pkginfo python3-pyinotify python3-ruamel.yaml python3-setuptools python3-six python3-subunit python3-testtools python3-urllib3 sbuild gnome-pkg-tools postgresql-server-dev-all lintian dos2unix gpg libdebhelper-perl git"

if [ ! -d /srv/chroot/jenkins ]; then
sudo chroot /srv/chroot/jenkins apt install $INCLUDE
