Upstream Releases
=================

This repository contains packages from Debian unstable for which it is possible
to automatically merge in a new upstream version.

These packages are currently only available for amd64.

Using the repository
~~~~~~~~~~~~~~~~~~~~

To use the apt repository, run something like::

   echo deb https://janitor.debian.net/apt upstream-releases/ > /etc/apt/sources.list.d/janitor.list
   gpg --recv-keys 6F915003D1998D6A
   gpg --export 6F915003D1998D6A | sudo apt-key add -
   apt update

The repository is marked as ``experimental``, meaning that apt won't
automatically update to packages in them unless explicitly requested to do so.
To e.g. install the version of *cifs-utils* that's in this repository, run::

   apt install -t lintian-fixes offlineimap

Pinning
~~~~~~~

Optionally, if you want to track a given package from one of the janitor's
repositories, add the following to ``/etc/apt/preferences`` (and see
`the documentation about apt preferences <https://wiki.debian.orgAptPreferences>`_)::

    Package: offlineimap
    Pin: release a=upstream-releases
    Pin-Priority: 800

Package list
~~~~~~~~~~~~

The following source packages are currently available:

.. include:: package-list.rst
