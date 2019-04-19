Apt repositories
================

For each changes that the janitor makes successfully, it publishes the
resulting Debian package.

To use the apt repository, e.g. for lintian fixes, run::

   echo deb https://janitor.debian.net/apt lintian-fixes/ > /etc/apt/sources.list.d/janitor.list
   gpg --recv-keys 6F915003D1998D6A
   gpg --export 6F915003D1998D6A | sudo apt-key add -
   apt update

Available sections are:

* ``lintian-fixes``: fixes created by `lintian-brush
  <https://packages.debian.org/lintian-brush>`_. These are probably most useful
  if you are the maintainer.
* ``upstream-releases``: builds for new upstream releases
* ``upstream-snapshots``: builds for new upstream snapshots (i.e. recent revisions)

The repositories are marked as ``experimental``, meaning that apt won't
automatically update to packages in them unless explicitly requested to do so.
To e.g. install the version of *offlineimap* that's in the **lintian-fixes** repository, run::

   apt install -t lintian-fixes offlineimap

Pinning
~~~~~~~

Optionally, if you want to track a given package from one of the janitor's
repositories, add the following to ``/etc/apt/preferences`` (and see
`the documentation about apt preferences <https://wiki.debian.orgAptPreferences>`_)::

    Package: offlineimap
    Pin: release a=upstream-releases
    Pin-Priority: 800
