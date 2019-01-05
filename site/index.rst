The Debian Janitor
==================

The Debian janitor is a project to automatically propose fixes for common and trivial to fix problems in Debian packages.

It finds repositories associated with packages in the Debian archive, runs a set of standard modifications over them, builds the resulting package and then creates a pull request with the resulting changes.

The ultimate goal is to reduce the overhead for simple archive-wide changes to the point that maintainers just have to click *_Merge_* in a web UI.

At the moment, Debian Janitor is available as an opt-in service.

.. toctree::
   :maxdepth: 1

   history
   status
   credentials

FAQ
***

.. contents:: Contents:
   :local:

How do I opt in?
~~~~~~~~~~~~~~~~

Propose a change to the `policy <https://salsa.debian.org/jelmer/debian-janitor/blob/master/policy.conf>`_ that adds an entry for the relevant maintainer or package.

Alternatively, you can send me an e-mail (`jelmer@debian.org <mailto:jelmer@debian.org>`_).

I don’t find this useful. How do I stop it?
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

You can send me a short message with the maintainer e-mail address (this can be a team or an individual) for which to stop the merge proposals. You can reach me via one of the following mediums:

* E-mail: `jelmer@debian.org <mailto:jelmer@debian.org>`_
* on IRC: *jelmer* on *irc.oftc.net*
* XMPP: *jelmer@jelmer.uk*

It would be great if you can also give me an idea of what specifically you don’t appreciate about these merge proposals, and if there’s anything I can do to improve them.

Alternatively, propose a change to the `policy <https://salsa.debian.org/jelmer/debian-janitor/blob/master/policy.conf>`_ on Salsa.

This is great. How do I get it to automatically push improvements to my repository?
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Simply give the bot commit access to your repository, and it will push fixes rather than proposing them.

How are repositories located?
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The bot uses the Vcs-Git and Vcs-Bzr fields in source packages in unstable to locate repositories.

What repositories are supported?
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Repositories on the following hosting platforms are supported:

* `GitLab <https://www.gitlab.com/>`_ (currently supported instances: `GitLab.com <https://gitlab.com/>`_, `salsa.debian.org <https://salsa.debian.org/>`_)
* `GitHub <https://github.com/>`_
* `Launchpad <https://launchpad.net/>`_ (both `Bazaar <https://bazaar-vcs.org>`_ and `Git <https://git-scm.com>`_ repositories)

Work is under way to also support Mercurial. Subversion support may also be an option, though I have yet to work out what the equivalent of pull requests in Subversion would be.

What kind of changes are made?
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The janitor currently proposes changes that can be made by the `lintian-brush <https://salsa.debian.org/jelmer/lintian-brush>`_ tool. This includes fixes for the following issues flagged by lintian:

.. include:: lintian-brush-tags.txt

The bot is proposing an incorrect change. Where do I report this?
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The bot honors lintian overrides, and will not propose fixes for issues reported by lintian that have been overridden.

For issues with a fix that the bot has proposed, please just follow up on the merge proposal.

The bot is doing something else wrong. Where do I report this?
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Please report issues in the `Debian Janitor <https://salsa.debian.org/jelmer/debian-janitor>`_ project on Salsa.

How do I run the fixers locally?
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

To run the collection of fixer scripts on a locally checked out package, simply run *lintian-brush*::

    $ apt install lintian-brush
    $ lintian-brush

This will report the fixers that were run and automatically commit the changes to the local repository.

How can I contribute more fixer scripts?
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

You can contribute scripts to the *lintian-brush* repository at `https://salsa.debian.org/jelmer/lintian-brush <https://salsa.debian.org/jelmer/lintian-brush>`_. See the lintian-brush `README file <https://salsa.debian.org/jelmer/lintian-brush/blob/master/README.md>`_ for details.

Consider also adding relevant lintian tags to `lintian upstream <https://salsa.debian.org/lintian>`_. This allows silver-platter’s infrastructure to recognize which repositories it needs to process.

What technologies are used?
~~~~~~~~~~~~~~~~~~~~~~~~~~~

`Lintian <https://lintian.debian.org/>`_ is responsible for finding the issues
in packages.

`UDD <https://wiki.debian.org/UltimateDebianDatabase/>`_ is used to find package VCS URLs and to retrieve the lintian results.

`Breezy <https://www.breezy-vcs.org/>`_ provides abstractions over the version control system (Git, Bazaar, Mercurial, Subversion) and the supported hosting platforms (GitHub, GitLab, Launchpad).

`Lintian-brush <https://salsa.debian.org/jelmer/lintian-brush>`_ is responsible for actually making changes to the packages.

`Silver-Platter <https://jelmer.uk/code/silver-platter>`_ ties this all together; it trawls UDD to find packages that are affected by lintian tags that lintian-brush knows how to fix, clones the packaging branches, invokes lintian-brush and pushes back or creates merge proposals.

* :ref:`genindex`
* :ref:`search`
