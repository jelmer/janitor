Adding a new suite
==================

Create a new mutators script
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

At the core of every suite is a script that can make changes
to a version controlled branch.

This script will be executed in a version controlled checkout of
a source package, and can make changes to the package as it sees fit.
See `this blog post <https://www.jelmer.uk/silver-platter-intro.html>`_ for more
information about creating mutator scripts.

You can test the script independently by running silver-platter, e.g.

``./debian-svp apply --command=myscript --dry-run --diff`` (from a checkout)
or

``./debian-svp run --command=myscript --dry-run --diff package-name``

Add configuration for the suite
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

In janitor.conf, add a section for the suite. E.g.::

    suite {
      name: "some-name"
      branch_name: "some-name"
      debian_build {
        archive_description: "Description for use in apt"
        build_suffix: "suf"
      }
    }

In policy.conf, add a default stanza::

    policy {
      suite {
       name: "some-name"  # This is the name of the suite
       command: "some-name"  # This is the mutator script to run
       publish { mode: propose }  # Default publishing mode
      }
    }

Add script for finding candidates
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Add a script that can gather candidates for the new suite. This script should
be run regularly to find new candidates to schedule, with its output fed into
``python3 -m janitor.candidates``. The easiest way to do this is to add
the script to ``schedule.sh``

See janitor/candidates.proto for the textproto schema of the output.

Add site (optional)
~~~~~~~~~~~~~~~~~~~

Add a website for the new suite under ``janitor/site``
