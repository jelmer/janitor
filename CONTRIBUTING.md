[The Janitor](https://github.com/jelmer/janitor) sits atop a number of other
projects, and those are where most of the interesting things happen.
You may want to check out one of them.
They're probably also easier to setup and run, unlike the Janitor.

## Environments

It is recommended to use Debian Testing as the base OS/chroot.

### Development Environment

You'll want to install various bits of software.
On a Debian-based OS, run:

```console
$ sudo apt install \
    cargo \
    gcc \
    git \
    libpython3-dev \
    libssl-dev \
    pkg-config \
    protobuf-compiler \
    python3-gpg
```

- - -

In addition to these packages, will need to use Python's PIP and a virtual
environment to install the rest of the Python-based dependencies:

```console
$ sudo apt install \
    python3-venv
$ git clone https://github.com/jelmer/janitor.git
$ cd janitor/
$ python3 -m venv .venv
$ . ./.venv/bin/activate
$ pip3 install --editable .[dev]
```
<!--
Via python3-venv, there will be: `./.venv/bin/pip` (which is why don't need python3-pip)
-->

_Python's package management over OS network apt package, as they may be too dated._

### Production Environment

We would recommend using [containers](Dockerfiles_.md) to run each of the Janitor services.

There are (daily) [pre-built containers](https://github.com/jelmer?tab=packages&repo_name=janitor),
otherwise you can create them yourself:

```console
$ sudo apt install \
    buildah \
    make
$ make build-all
```
