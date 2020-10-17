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

Several cron jobs that run daily:

* the *package_metadata* syncer imports package metadata from UDD
* the *candidate* syncer determines candidates
* the *scheduler* determines what packages are ready for processing
  based on lintian, vcswatch and upstream data, and queues them.

Several permanently running jobs:

* the *publisher* proposes or pushes changes that have been successfully
  created and built previously, and which can provide VCS diffs
* the *runner* processes the queue, kicks off workers for
  each package and stores the results.
* one or more *workers* which are responsible for actual generating and
  building changes.
* an *archiver* combined with a *repository manager* (aptly) that takes
  care of managing the apt archives and publishes them; it
  also takes care of running e.g. debdiff or diffoscope
* a *site* job that renders the web site

There are no requirements that these jobs run on the same machine, but they are
expected to have secure network access to each other.

Every job runs a HTTP server to allow API requests and use of /metrics, for
prometheus monitoring.

Workers are fairly naive; they simply run a ``silver-platter`` subcommand
to create branches and they build the resulting branches. The runner
then fetches the results from each run and (if the run was successful)
uploads the .debs and optionally proposes a change.

The publisher is responsible for enforcing rate limiting, i.e. making sure
that there are no more than X pull requests open per maintainer.

Web site
========

The web site is served by the ``janitor.site`` module using jinja2 templates
from the ``janitor/site/templates/`` subdirectory.

Installation
============

The easiest way to set up a new instance of the Janitor is probably to use the
ansible playbooks at https://salsa.debian.org/jelmer/debian-janitor-ansible

Contributing
============

The easiest way to get started with contributing to the Janitor is to work on
identifying issues and adding fixers. There is
[a guide](https://salsa.debian.org/jelmer/lintian-brush/-/blob/master/doc/fixer-writing-guide.rst)
on identifying good candidates and writing fixers
in the lintian-brush repository.

Some of us hang out in the ``#debian-janitor`` IRC channel on OFTC (irc.oftc.net).
