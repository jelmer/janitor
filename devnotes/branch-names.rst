Goal
====

For runs:
* runs/$uuid/$function tags
* runs/$uuid/tags/$tag tags for new/updated upstream tags

There is a symref at refs/$suite/$function that points at said tag and updated

Also, track this information in the database.

For related repositories, create remotes:

* remotes/origin/ for Debian packaging
* remotes/upstream/ for Upstream

Names for functions:

* "main" for the main branch (packaging or otherwise)
* "upstream" for the upstream import branch for Debian packages
* "pristine-tar" for the pristine-tar branch

Roadmap
=======

* Export a branch_names dictionary in janitor.worker / janitor.pull_worker
  + contains mapping from function to revision

* Export a set of new tags with values in janitor.worker / janitor.pull_worker

* Push funky named tags (runs/$uuid/$function) to vcs repository

* Push symrefs (refs/$suite/$function => refs/tags/$uuid/$function)
  + would the worker do this, or do we leave it up to the publisher?

* Push origin and upstream repositories

* Update publisher to use new tag names
