This repository contains the setup for a "Janitor" bot. This is basically
a platform for managing large-scale automated code improvements on
top of [silver-platter](https://github.com/jelmer/silver-platter).

Any code that is not related to the platform but to actually making changes
should probably live in either ``silver-platter``, ``breezy`` or a
specific codemod (such as [lintian-brush](https://salsa.debian.org/jelmer/lintian-brush)).

There are currently several instances of the Janitor running. For their configuration, see:

* [Debian Janitor](https://janitor.debian.net/) - Setup at https://salsa.debian.org/jelmer/janitor.debian.net
* [Kali Janitor](https://janitor.kali.org/) - Configuration repository is private
* [Upstream Janitor aka ``Scruffy``](https://www.scruffy.dev/) - Setup at https://github.com/scruffy-team/scruffy

Philosophy
==========

There are some straightforward changes to code that can be made
using scripting. The janitor's job is to opportunistically make those changes
when it is certain it can do so with a high confidence, and to back off
otherwise.

The janitor continuously tries to run changes on the set of repositories it
knows about. It tries to be clever about scheduling those operations that
are more likely to yield results and be published (i.e. merged or pushed).

Design
======

The janitor is made up out of multiple components. The majority of these
are not Debian-specific. The janitor is built on top of
[silver-platter](https://github.com/jelmer/silver-platter) and relies
on that project for most of the grunt work.

There are several cron jobs that run daily:

* the *package_metadata* syncer imports package metadata from UDD
* the *candidate* syncer determines candidates

Several permanently running jobs:

* the *publisher* proposes or pushes changes that have been successfully
  created and built previously, and which can provide VCS diffs
* the *vcs store* manages and stores VCS repositories (git, bzr) [optional]
* the [ognibuild](https://github.com/jelmer/ognibuild) dep server is used to
  resolve missing dependencies
* the *runner* processes the queue, kicks off workers for
  each package and stores the results.
* one or more *workers* which are responsible for actual generating and
  building changes.
* an *archiver* that takes care of managing the apt archives and publishes them
* a *site* job that renders the web site
* the *differ* takes care of running e.g. debdiff or diffoscope between binary runs

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

Worker
======
The actual changes are made by various codemod scripts that implement
the [silver-platter protocol](https://github.com/jelmer/silver-platter/blob/master/codemod-protocol.rst).

Web site
========

The web site is served by the ``janitor.site`` module using jinja2 templates
from the ``janitor/site/templates/`` subdirectory.

Installation
============

There are two common ways of deploying a new janitor instance.

 * On top of kubernetes (see the configuration for the Debian & Upstream janitor)
 * Using e.g. ansible and/or a venv

Docker
------

Several docker images are provided

 * ghcr.io/jelmer/janitor/base - base image, essentially debian:testing-slim with some additional packages installed
 * ghcr.io/jelmer/janitor/archive - APT archive generator
 * ghcr.io/jelmer/janitor/differ - diffoscope/debdiff generator
 * ghcr.io/jelmer/janitor/publish - VCS publisher
 * ghcr.io/jelmer/janitor/runner - Queue management & Run handling
 * ghcr.io/jelmer/janitor/site - Web site & public API
 * ghcr.io/jelmer/janitor/git_store - storage for Git
 * ghcr.io/jelmer/janitor/bzr_store - storage for Bazaar
 * ghcr.io/jelmer/janitor/worker - Base for workers

The notifiers/ directory contains a couple of convenience scripts for sending
notification of accepted merge proposals and pushes.

 * ghcr.io/jelmer/janitor/irc_notify - IRC notification bot
 * ghcr.io/jelmer/janitor/mastodon_notify - Mastodon notification bot
 * ghcr.io/jelmer/janitor/matrix_notify - Mastodon notification bot
 * ghcr.io/jelmer/janitor/xmpp_notify - XMPP Notification Bot

Contributing
============

See CONTRIBUTING.md for instructions on e.g. setting up
a development environment.

If you're interested in working on adding another campaign, see
[adding-a-new-campaign](devnotes/adding-a-new-campaign.rst).

Some of us hang out in the ``#debian-janitor`` IRC channel on OFTC
(irc.oftc.net) or
[#debian-janitor:matrix.debian.social](https://matrix.to/#/#debian-janitor:matrix.debian.social).
