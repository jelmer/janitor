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

For a Janitor instance, you probably want a custom website in combination with
the Janitor API. See the existing instances for inspiration.

## Registering workers

Workers authenticate to the site with a name and password stored in the `worker`
table. There are two ways to register one.

The `janitor-admin` CLI is the easiest and safest option, since it generates the
password for you rather than asking you to invent and hash one yourself:

```console
$ janitor-admin --url https://your-janitor-instance/ worker add myworker
Created worker myworker
Password: <generated password>
(Store this password now; it is not recoverable.)
```

This calls the admin-only `POST /cupboard/api/workers` API on the site, which
generates a random password, stores its hash, and returns the plaintext password once
in the response. It isn't kept anywhere in recoverable form, so copy it down when it's
printed.

If you don't have `janitor-admin` set up, you can register a worker directly in the
database instead:

```console
$ psql janitor -c "INSERT INTO worker (name, password) VALUES ('myworker', crypt('mypassword', gen_salt('bf')))"
```

Replace `mypassword` with the password you want the worker to use.
