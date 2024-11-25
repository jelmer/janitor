#!/usr/bin/python3
from setuptools import setup
from setuptools_rust import Binding, RustBin, RustExtension

setup(
    rust_extensions=[
        RustExtension("janitor._common", "common-py/Cargo.toml", binding=Binding.PyO3),
        RustExtension(
            "janitor._publish", "publish-py/Cargo.toml", binding=Binding.PyO3
        ),
        RustExtension("janitor._runner", "runner-py/Cargo.toml", binding=Binding.PyO3),
        RustExtension("janitor._site", "site-py/Cargo.toml", binding=Binding.PyO3),
        RustBin("janitor-mail-filter", "mail-filter/Cargo.toml", features=["cmdline"]),
        RustBin("janitor-worker", "worker/Cargo.toml", features=["cli", "debian"]),
        RustBin("janitor-dist", "worker/Cargo.toml", features=["cli", "debian"]),
    ]
)
