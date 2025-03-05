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
or using PIP in a Python virtual environment.

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

## Code Style

The Janitor uses the rustfmt for Rust code and ``ruff format`` for Python code.
You can use ``make reformat`` to format all code.

## Guidelines

### Programming Languages

The Janitor was originally written in Python, but is now being converted to
Rust. New code should be written in Rust, unless there is a good reason to
write it in Python.

### Code Coverage

All new code should be covered by tests. The Janitor uses the
[pytest](https://pytest.org) test framework for Python code and
[Rust's built-in test framework](https://doc.rust-lang.org/book/ch11-01-writing-tests.html)

Our current test coverage is a bit spotty, but we're working on improving it.

For now, please ensure that any new code you write is covered by tests - but also
make sure to manually test code that you change and ensure that it works as expected.

### Code That Does Not Belong Here

The Janitor is the basis for a number of other projects, and we try to keep it
manageable. If you're working on a feature that is not directly related to the
Janitor itself but specific to one of the other projects, please consider
whether it would be better to add it there instead.

Conversely, any code that purely deals with VCS interactions should probably
be in silver-platter.

### Maintenance Overhead

Every bit of code that is added to the Janitor increases the maintenance
overhead, but this is especially true for code that needs to be kept up to date
with other parts of the codebase or with external dependencies.

Please consider whether the code or text you are adding is really
necessary, and how easily it can bitrot.

### Keep Changes Small and Self-Contained

Try to keep changes small and self-contained. This makes it easier to review
them and to understand the impact of the change.

In general, changes should be preceded by a discussion in an issue.
PRs or commits should reference the issue they are addressing.
