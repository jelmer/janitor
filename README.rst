This repository contains the setup for the "Debian Janitor" bot. It contains
the specific configuration & infrastructure for the instance running on
janitor.debian.net. Any code that is more generic should probably be
in either ``silver-platter``, ``lintian-brush`` or ``breezy``.

To change what packages the janitor considers for merge proposals,
edit the `policy file <policy.conf>`_.
