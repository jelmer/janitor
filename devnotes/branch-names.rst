Goal
====

For runs:
* Push $uuid/$function tags to each repository

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

* Push funky named tags ($uuid/$function)

* Push symrefs (refs/$suite/$function => refs/tags/$uuid/$function)

* Push origin repositories

* Update publisher to use new tag names
