Adding a new suite
==================

Add a "changer" in silver-platter
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Add a worker class in silver-platter, probably in
``silver_platter/debian/some-name.py``.

The worker class should derive from ``DebianChanger`` in
``silver_platter.debian.changer``. See e.g. rrr.py for an example.

You can test the worker class independently by running silver-platter, e.g.
``./debian-svp some-name pkg-name --dry-run --diff``

Add configuration for the suite
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

In janitor.conf, add a section for the suite. E.g.:

```
suite {
  name: "some-name"
  archive_description: "Description for use in apt"
  branch_name: "some-name"
  build_suffix: "suf"
}
```

In policy.conf, add a default stanza:

```
policy {
  suite {
   name: "some-name"  # This is the name of the suite
   command: "some-name"  # This is the silver-platter subcommand to run
   publish { mode: propose }  # Default publishing mode
  }
}
```

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
