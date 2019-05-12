Upstream Snapshots
==================

This repository contains packages from Debian unstable for which it is possible
to automatically merge in a new upstream snapshot (i.e. the latest revision in
Git).

These packages are currently only available for amd64.

.. warning::
   The packages in this repository were created automatically, without review
   from a human.

.. toctree::
   package-status

Using the repository
~~~~~~~~~~~~~~~~~~~~

To use the apt repository, run something like::

   echo deb https://janitor.debian.net/ fresh-snapshots/ > /etc/apt/sources.list.d/janitor.list
   gpg --recv-keys 6F915003D1998D6A
   gpg --export 6F915003D1998D6A | sudo apt-key add -
   apt update

The repository is marked as ``experimental``, meaning that apt won't
automatically update to packages in them unless explicitly requested to do so.
To e.g. install the version of *cifs-utils* that's in this repository, run::

   apt install -t fresh-snapshots cifs-utils

The packages are also versioned in such a way that if the new upstream version
gets uploaded to the official Debian APT repository, it will replace the package
in this archive.

Pinning
~~~~~~~

Optionally, if you want to track a given package so upgrades happen automatically,
add the following to ``/etc/apt/preferences`` (and see
`the documentation about apt preferences <https://wiki.debian.org/AptPreferences>`_)::

    Package: cifs-utils
    Pin: release a=fresh-snapshots
    Pin-Priority: 800

Package list
~~~~~~~~~~~~

The following source packages with new upstream snapshots merged are currently
available:

.. include:: package-list.rst
