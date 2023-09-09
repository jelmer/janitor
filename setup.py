#!/usr/bin/python3
from setuptools import setup
from setuptools_protobuf import Protobuf
from setuptools_rust import Binding, RustBin, RustExtension

setup(
    protobufs=[Protobuf('janitor/config.proto', mypy=True)],
    rust_extensions=[
        RustBin(
            "janitor-mail-filter", "crates/mail-filter/Cargo.toml",
            features=["cmdline"]),
        RustBin(
            "janitor-worker", "Cargo.toml",
            features=["cli"]),
        RustExtension(
            "janitor._worker", "crates/worker-py/Cargo.toml",
            binding=Binding.PyO3)]
)
