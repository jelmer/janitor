The Janitor sits atop a number of other projects, and those are where
most of the interesting things happen. You may want to check out one of them.
They're probably also easier to setup and run, unlike the Janitor.


Environment
===========

Mostly you can use pip to install Python-based dependencies. In addition to
those, you'll also want to install various other bits of software. On Debian,
run:

 $ sudo apt install libgpgme-dev rustc libapt-pkg-dev protobuf-compiler \
     python3-venv python3-pip rustc

For example, to create a dev enviroment:

 $ python3 -m venv
 $ . ./bin/activate
 $ pip3 install -e .[dev,debian]
