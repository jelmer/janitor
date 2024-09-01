#!/usr/bin/python3
from setuptools import setup
from setuptools_rust import Binding, RustBin, RustExtension

setup(
        rust_extensions=[
            RustExtension('janitor._worker', 'worker-py/Cargo.toml', binding=Binding.PyO3),
            RustExtension('janitor._common', 'common-py/Cargo.toml', binding=Binding.PyO3),
            RustExtension('janitor._publish', 'publish-py/Cargo.toml', binding=Binding.PyO3),
            RustExtension('janitor._runner', 'runner-py/Cargo.toml', binding=Binding.PyO3),
            RustBin('janitor-mail-filter', 'mail-filter/Cargo.toml', features=['cmdline']),
        ])
