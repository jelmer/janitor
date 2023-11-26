#!/usr/bin/python3
from setuptools import setup
from setuptools_rust import RustExtension, RustBin, Binding

setup(
        rust_extensions=[
            RustExtension('janitor._worker', 'crates/worker-py/Cargo.toml', binding=Binding.PyO3),
            RustBin('janitor-worker', 'crates/worker/Cargo.toml', features=['cli']),
            RustBin('janitor-mail-filter', 'crates/mail-filter/Cargo.toml', features=['cmdline']),
        ])
