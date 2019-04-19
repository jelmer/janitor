Apt repositories
================

For each changes that the janitor makes successfully, it publishes the
resulting Debian package.

To use the apt repository, e.g. for lintian fixes, run::

   echo deb https://janitor.debian.net/apt lintian-fixes/ > /etc/apt/sources.list.d/janitor.list
   gpg --recv-keys 6F915003D1998D6A
   gpg --export 6F915003D1998D6A | sudo apt-key add -
   apt update

The repositories are marked as ``experimental``, meaning that apt won't
automatically update to packages in them unless explicitly requested to do so.
To e.g. install the version of *offlineimap* that's in the **lintian-fixes** repository, run::

   apt install -t lintian-fixes offlineimap
