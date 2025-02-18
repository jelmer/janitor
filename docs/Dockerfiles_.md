## Containers (Dockerfiles_*)

_Stand alone_

**Pull (Pre-Built)**:
```console
$ podman pull ghcr.io/jelmer/janitor/worker:latest
```

**Build**:
```console
$ podman build -t ghcr.io/jelmer/janitor/worker:latest -f Dockerfile_worker .
$ buildah build -t ghcr.io/jelmer/janitor/worker:latest -f Dockerfile_worker .
```

**Run**:
```console
$ podman run --rm --publish 9914:9914 --name janitor-archive ghcr.io/jelmer/janitor/archive:latest
$ podman run --rm --publish 9929:9929 --name janitor-bzr_store --volume $( pwd ):/mnt/janitor ghcr.io/jelmer/janitor/bzr_store:latest --config /mnt/janitor/janitor.conf.example
$ podman run --rm --publish 9920:9920 --name janitor-differ --volume $( pwd ):/mnt/janitor ghcr.io/jelmer/janitor/differ:latest --config /mnt/janitor/janitor.conf.example
$ podman run --rm --publish 9923:9923 --name janitor-git_store --volume $( pwd ):/mnt/janitor ghcr.io/jelmer/janitor/git_store:latest --config /mnt/janitor/janitor.conf.example
$ podman run --rm --publish 9934:9934 --name janitor-ognibuild_dep ghcr.io/jelmer/janitor/ognibuild_dep:latest
$ podman run --rm --publish 9912:9912 --name janitor-publish ghcr.io/jelmer/janitor/publish:latest
$ podman run --rm --publish 9911:9911 --name janitor-runner --volume $( pwd ):/mnt/janitor ghcr.io/jelmer/janitor/runner:latest --config /mnt/janitor/janitor.conf.example
$ podman run --rm --publish 8082:8082 --name janitor-site ghcr.io/jelmer/janitor/site:latest
$ podman run --rm --publish 8080:8080 --name janitor-worker ghcr.io/jelmer/janitor/worker:latest 80
```

**Troubleshooting**:
```console
$ podman run -it --rm -p 8080:8080 --entrypoint=/bin/bash ghcr.io/jelmer/janitor/worker:latest
$ podman run \
  --interactive \
  --tty \
  --rm \
  --publish 8080:8080 \
  --entrypoint=/bin/bash \
  --volume $( pwd ):/mnt/janitor \
  --workdir /mnt/janitor \
  ghcr.io/jelmer/janitor/worker:latest
```

- - -
