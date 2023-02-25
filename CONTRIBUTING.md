The Janitor sits atop a number of other projects, and those are where
most of the interesting things happen. You may want to check out one of them.
They're probably also easier to setup and run, unlike the Janitor.


Environment
===========

Mostly you can use pip to install Python-based dependencies. In addition to
those, you'll also want to install various other bits of software. On Debian,
run:

```
 $ sudo apt install libgpgme-dev rustc libapt-pkg-dev protobuf-compiler \
     python3-venv python3-pip rustc libpcre3-dev libgpg-error-dev swig
```

For example, to create a dev environment:

```
 $ python3 -m venv
 $ . ./bin/activate
 $ pip3 install python_apt@git+https://salsa.debian.org/apt-team/python-apt
 $ pip3 install -e .[dev,debian]
```
