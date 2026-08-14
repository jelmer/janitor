# Running Janitor in production

There are [containers](Dockerfiles_.md) available for each of the Janitor services.

[pre-built containers](https://github.com/jelmer?tab=packages&repo_name=janitor) are
available, but you can also create them yourself:

```console
$ sudo apt install \
    buildah \
    make
$ make build-all
```

## Database

The services share a single PostgreSQL database. Before starting anything,
load the schema:

```console
$ createdb janitor
$ cat schema/state.sql | psql janitor
$ cat schema/debian/debian.sql | psql janitor  # only if using the Debian-specific features
```

Without this, every service connects to the database successfully and then
fails on its first query, since none of the expected tables exist yet.

## Forge credentials

Publishing merge proposals requires a forge API token (GitHub/GitLab/etc),
but this is not part of janitor.conf - publish and runner delegate to
breezy's own credential store instead, at
`~/.config/breezy/authentication.conf`.

For GitHub, run:

```console
$ brz github-login
```

which prompts for a username and token and writes the `[Github]` section
itself. To do it by hand instead, the section breezy expects is:

```
[Github]
scheme = https
host = github.com
url = https://api.github.com
private_token = <your token>
```

For a Janitor instance, you probably want a custom website in combination with
the Janitor API. See the existing instances for inspiration.
