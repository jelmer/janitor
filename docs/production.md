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
