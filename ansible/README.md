This directory contains a simple ansible collection with roles that are
relevant for running janitor instances.

playbooks for specific instances can be found here:

 * https://salsa.debian.org/jelmer/debian-janitor-ansible
 * https://gitlab.com/kalilinux/internal/janitor.kali.org

The various roles of the Janitor can be set up on different hosts or the same host.

The only externally facing job is the janitor-site. All other jobs should not
be externally accessible. A wireguard network is one way of achieving this.

After installation, you'll need to manually log in to the janitor user on the
publisher and log in to the relevant hoster sites by running "svp login". E.g.::

    svp login https://github.com/
    svp login https://gitlab.com/
    svp login https://salsa.debian.org/

Roles
=====

* janitor-db: The postgresql database
* janitor-irc-notify: Notifies on IRC when merge proposals are merged
* janitor-maintenance: Regular importing of package metadata and candidates
* janitor-mastodon-notify: Notifies on Mastodon when merge proposals are merged
* janitor-prometheus: Prometheus setup for all janitor jobs
* janitor-publish: VCS manager; keeps cache of packaging branches and holds
    results. Needs ample disk space.
* janitor-runner: Processing coordinator
* janitor-site: User-facing site, including external API
* janitor-worker: The actual worker (modifies & builds packages)

Debian-specific roles
---------------------

* janitor-archive: Archive management; stores built debs and can provide
    debdiffs/diffoscope diffs. Needs ample disk space.
* janitor-auto-upload: Automatically upload new builds using dput
