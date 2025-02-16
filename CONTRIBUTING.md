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
    libgpgme-dev \
    libpython3-dev \
    libssl-dev \
    pkg-config \
    protobuf-compiler \
    swig
```

<!--
In order (Package, command, error):
  - git                  $ git clone https://github.com/jelmer/janitor.git     # -bash: git: command not found
  - protobuf-compiler    $ pip3 install --editable .                           # Getting requirements to build editable ... error -> error: Unable to find protobuf compiler protoc
  - pkg-config           $ pip3 install --editable .                           # Collecting gpg -> /root/janitor/.venv/bin/gpgme-config: line 29: exec: pkg-config: not found
  - libgpgme-dev         $ pip3 install --editable .                           # Collecting gpg -> Package gpgme was not found in the pkg-config search path.
  - cargo                $ pip3 install --editable .                           # Building wheels for collected packages: janitor, aiohttp-apispec, breezy -> Building editable for janitor (pyproject.toml) ... error -> distutils.errors.DistutilsPlatformError: can't find Rust compiler
  - gcc                  $ pip3 install --editable .                           # Building wheel for breezy (pyproject.toml) ... error -> error: command 'x86_64-linux-gnu-gcc' failed: No such file or directory
  - libpython3-dev       $ pip3 install --editable .                           # Building wheel for breezy (pyproject.toml) ... error -> breezy/bzr/_simple_set_pyx.c:32:10: fatal error: Python.h: No such file or directory
  - libssl-dev           $ pip3 install --editable .                           # warning: openssl-sys@0.9.105: Could not find directory of OpenSSL installation, and this `-sys` crate cannot proceed without this knowledge. If OpenSSL is installed and this crate had trouble finding it,  you can set the `OPENSSL_DIR` environment variable for the compilation process. See stderr section below for further information.
  - g++                  $ pip3 install --editable .                           # error: linking with `cc` failed: exit status: 1 -> distutils.errors.CompileError: `cargo build --manifest-path worker/Cargo.toml --message-format=json-render-diagnostics -v --features 'cli debian'` failed with code 101
  - swig                 $ pip3 install --editable .                           # Running setup.py install for gpg ... error -> Using gpgme.h from /usr/include/gpgme.h -> error: command 'swig' failed: No such file or directory

Dependencies
  - cargo -> rustc
-->

- - -

Even when using the latest [Debian](https://tracker.debian.org/pkg/rustc)
stable or [Ubuntu](https://packages.ubuntu.com/search?keywords=rustc) LTS version,
as the base OS/chroot, their included OS network apt package repositories may have an an
unsupported outdated version of `cargo`, and `rustc`.
_As a result, would need to use [rustup](https://rustup.rs/) will bring you to the latest version._

```console
$ sudo apt purge rustc
$ sudo apt install \
    curl \
  && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh \
  && . "$HOME/.cargo/env"
```
<!--
If updating the packages above, check:
  - ./.github/workflows/python-package.yml
  - ./CONTRIBUTING.md
  - ./Dockerfile_*

- - -

rustup should be in Debian 13
  Switch to using this package when its out & drop curl command/line and this comment section
  https://tracker.debian.org/pkg/rustup
  https://wiki.debian.org/Rust
  https://rust-lang.github.io/rustup/installation/other.html

- - -

Debian "bookworm" 12 is the current stable version which has rustc@1.63.0
> lock file version `4` was found, but this version of Cargo does not understand this lock file, perhaps Cargo needs to be updated?

Ubuntu "Noble Numbat" 24.04 is the current LTS version which has rustc@1.75.0
> lock file version 4 requires `-Znext-lockfile-bump`

GitHub Actions (SaaS) uses package(s) outside of the standard network apt repos:
> $ dpkg -l | grep "cargo\|rustc" -> "empty"
> $ cargo --version -> 1.84.1 (66221abde 2024-11-19) // $ rustc --version -> 1.84.1 (e71f9a9a9 2025-01-27)

REF: https://github.com/rust-lang/cargo/issues/14655#issuecomment-2400237392
> Lockfile v4 has been stable since Rust 1.78.
-->

In addition to these packages, will need to use Python's PIP and a virtual
environment to install the rest of the Python-based dependencies:

```console
$ sudo apt install \
    python3-venv
$ git clone https://github.com/jelmer/janitor.git
$ cd janitor/
$ python3 -m venv .venv
$ cp -v ./scripts/* ./.venv/bin/
$ . ./.venv/bin/activate
$ pip3 install --editable .[dev]
```
<!--
Via python3-venv, there will be: `./.venv/bin/pip` (which is why don't need python3-pip)
-->

_Python's package management over OS network apt package, as they may be too dated._

### Production Environment

We would recommend using containers to to run each of the Janitor services.

There are (daily) [pre-built containers](https://github.com/jelmer?tab=packages&repo_name=janitor),
otherwise you can create them yourself:

```console
$ sudo apt install \
    buildah \
    make
$ make build-all
```

- - -

Running the containers can be done however is best to suite your environment,
such as using Docker or Kubernetes.

Example, using Docker:

```console
$ sudo apt install \
    podman-compose
$ podman-compose --project-name janitor up --build --force-recreate
```
