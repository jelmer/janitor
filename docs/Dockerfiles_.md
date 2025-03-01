## Containers (`Dockerfiles_*`)

_Stand-alone_

**Pull (Pre-Built)**:

```console
$ podman pull ghcr.io/jelmer/janitor/site:latest
```

**Build**:

```console
$ podman build -t ghcr.io/jelmer/janitor/site:latest -f Dockerfile_site .
$ buildah build -t ghcr.io/jelmer/janitor/site:latest -f Dockerfile_site .
```

**Run**:

```console
$ podman run --rm                     --name janitor-archive       --volume $( pwd ):/mnt/janitor ghcr.io/jelmer/janitor/archive:latest       --config /mnt/janitor/janitor.conf.example --cache-directory /srv/cache --dists-directory /srv/dists
$ podman run --rm --publish 9930:9930 --name janitor-bzr_store     --volume $( pwd ):/mnt/janitor ghcr.io/jelmer/janitor/bzr_store:latest     --config /mnt/janitor/janitor.conf.example        --vcs-path /srv/bzr
$ podman run --rm                     --name janitor-differ        --volume $( pwd ):/mnt/janitor ghcr.io/jelmer/janitor/differ:latest        --config /mnt/janitor/janitor.conf.example --cache-path /srv/cache
$ podman run --rm --publish 9924:9924 --name janitor-git_store     --volume $( pwd ):/mnt/janitor ghcr.io/jelmer/janitor/git_store:latest     --config /mnt/janitor/janitor.conf.example        --vcs-path /srv/git
$ podman run --rm                     --name janitor-ognibuild_dep                                ghcr.io/jelmer/janitor/ognibuild_dep:latest
$ podman run --rm                     --name janitor-publish       --volume $( pwd ):/mnt/janitor ghcr.io/jelmer/janitor/publish:latest       --config /mnt/janitor/janitor.conf.example                                       --differ-url http://localhost:9920/ --external-url http://localhost/
$ podman run --rm --publish 9919:9919 --name janitor-runner        --volume $( pwd ):/mnt/janitor ghcr.io/jelmer/janitor/runner:latest        --config /mnt/janitor/janitor.conf.example
$ podman run --rm --publish 8090:8090 --name janitor-site          --volume $( pwd ):/mnt/janitor ghcr.io/jelmer/janitor/site:latest          --config /mnt/janitor/janitor.conf.example --archiver-url http://localhost:9914/ --differ-url http://localhost:9920/ --external-url http://localhost/ --publisher-url http://localhost:9912/ --runner-url http://localhost:9911/
$ podman run --rm                     --name janitor-worker                                       ghcr.io/jelmer/janitor/worker:latest                                                   --base-url http://localhost/ --site-port 8080 --new-port 9820 9821
```

**Troubleshooting**:

```console
$ podman run -it --entrypoint=/bin/bash --rm -p 8090:8090 -v $( pwd ):/mnt ghcr.io/jelmer/janitor/site:latest
$ podman run \
  --tty \
  --interactive \
  --entrypoint=/bin/bash \
  --rm \
  --publish 8090:8090 \
  --volume $( pwd ):/janitor \
  --workdir /janitor \
  ghcr.io/jelmer/janitor/site:latest
```
