We'd like to limit the number of merge proposals that maintainers receive. One
way of doing this is to limit the number of open merge proposals per maintainer
at any given time.

This requires that the janitor has some idea of for which maintainers there are
merge proposals open. We can access the open merge proposals, and somehow keep
track of the involved maintainers (in an sqlite database?).

Open questions:

 * What do we do with merge proposals for which we don't know the associated
   maintainer?
