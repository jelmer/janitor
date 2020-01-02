This repository contains the setup for the "Debian Janitor" bot. It contains
the specific configuration & infrastructure for the instance running on
janitor.debian.net. Any code that is more generic should probably be
in either ``silver-platter``, ``lintian-brush`` or ``breezy``.

To change what packages the janitor considers for merge proposals,
edit the [policy file](policy.conf).

Philosophy
==========

There are some straightforward changes to Debian packages that can be made
using scripting. The janitor's job is to opportunistically make those changes
when it is certain it can do so with a high confidence, and to back off
otherwise.

Design
======

The janitor is made up out of multiple components:

* the *udd* syncer imports package metadata from UDD
* the *scheduler* determines what packages are ready for processing
  based on lintian and upstream data, and queues them.
* the *publisher* proposes or pushes changes that have been successfully
  created and built previously
* the *runner* processes the queue, kicks off workers for
  each package and stores the results.
* one or more *workers* which are responsible for actual generating and
  building changes.
* an *archiver* combined with a *repository manager* (aptly) that takes
  care of managing the apt archives and publishes them

Workers are fairly naive; they simply run a ``silver-platter`` subcommand
to create branches and they build the resulting branches. The runner
then fetches the results from each run and (if the run was successful)
uploads the .debs and optionally proposes a change.

The publisher is responsible for enforcing rate limiting, i.e. making sure
that there are no more than X pull requests open per maintainer.

Web site
========

The web site is generated using jinja2 templates from the ``site/``
subdirectory.
