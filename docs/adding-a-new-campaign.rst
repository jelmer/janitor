Adding a new campaign
=====================

Create a new codemod script
~~~~~~~~~~~~~~~~~~~~~~~~~~~

At the core of every campaign is a script that can make changes
to a version controlled branch.

This script will be executed in a version controlled checkout of
a source codebase, and can make changes to the codebase as it sees fit.
See `this blog post <https://www.jelmer.uk/silver-platter-intro.html>`_ for more
information about creating codemod scripts.

You can test the script independently by running silver-platter, e.g.

``./debian-svp apply --command=myscript --dry-run --diff`` (from a checkout)
or

``./debian-svp run --command=myscript --dry-run --diff package-name``

Add configuration for the campaign
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

In janitor.conf, add a section for the campaign. E.g.::

    campaign {
      name: "some-name"
      branch_name: "some-name"
      debian_build {
        build_suffix: "suf"
      }
    }

Add script for finding candidates
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Add a script that can gather candidates for the new campaign. This script should
be run regularly to find new candidates to schedule, with its JSON output
uploaded to $RUNNER_URL/candidates.
