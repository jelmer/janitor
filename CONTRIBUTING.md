[The Janitor](https://github.com/jelmer/janitor) sits atop a number of other
projects, and those are where most of the interesting things happen.
You may want to check out one of them.
They're probably also easier to setup and run, unlike the Janitor.

## Development Environment

Debian testing or unstable are the recommended base environments for development,
but other Linux distributions should work too.

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

In addition to these packages, you will need to install a number of
Python dependencies. These can be installed from the OS package manager,
or using a Python virtual environment.

For example, on Debian-based systems:

```console
$ sudo apt install \
    python3-venv
$ git clone https://github.com/jelmer/janitor.git
$ cd janitor/
$ python3 -m venv .venv
$ . ./.venv/bin/activate
$ pip3 install --editable .[dev]
```

## Running the tests

To run the tests, use:

```console

$ make test
```

This will run both the Python and Rust tests.
