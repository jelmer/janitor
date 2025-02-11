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

We would recommend using containers to run each of the Janitor services.

There are (daily) [pre-built containers](https://github.com/jelmer?tab=packages&repo_name=janitor),
otherwise you can create them yourself:

```console
$ sudo apt install \
    buildah \
    make
$ make build-all
```
