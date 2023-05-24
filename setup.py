#!/usr/bin/python3
from setuptools import setup
from setuptools_protobuf import Protobuf
from setuptools_rust import Binding, RustExtension

setup(
    protobufs=[Protobuf('janitor/config.proto', mypy=True)],
    rust_extensions=[
        RustExtension(
            "janitor._mail_filter", "crates/mail-filter-py/Cargo.toml",
            binding=Binding.PyO3),
        RustExtension(
            "janitor._worker", "crates/worker-py/Cargo.toml",
            binding=Binding.PyO3)]
)
