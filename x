python3 -m mypy py/janitor tests
Success: no issues found in 61 source files
python3 setup.py build_ext -i
/usr/lib/python3/dist-packages/setuptools/config/_apply_pyprojecttoml.py:82: SetuptoolsDeprecationWarning: `project.license` as a TOML table is deprecated
!!

        ********************************************************************************
        Please use a simple string containing a SPDX expression for `project.license`. You can also use `project.license-files`. (Both options available on setuptools>=77.0.0).

        By 2026-Feb-18, you need to update your project and remove deprecated calls
        or your builds will no longer be supported.

        See https://packaging.python.org/en/latest/guides/writing-pyproject-toml/#license for details.
        ********************************************************************************

!!
  corresp(dist, value, root_dir)
imported
/usr/lib/python3/dist-packages/setuptools/config/_apply_pyprojecttoml.py:61: SetuptoolsDeprecationWarning: License classifiers are deprecated.
!!

        ********************************************************************************
        Please consider removing the following classifiers in favor of a SPDX license expression:

        License :: OSI Approved :: GNU General Public License (GPL)

        See https://packaging.python.org/en/latest/guides/writing-pyproject-toml/#license for details.
        ********************************************************************************

!!
  dist._finalize_license_expression()
/usr/lib/python3/dist-packages/setuptools/dist.py:759: SetuptoolsDeprecationWarning: License classifiers are deprecated.
!!

        ********************************************************************************
        Please consider removing the following classifiers in favor of a SPDX license expression:

        License :: OSI Approved :: GNU General Public License (GPL)

        See https://packaging.python.org/en/latest/guides/writing-pyproject-toml/#license for details.
        ********************************************************************************

!!
  self._finalize_license_expression()
running build_ext
running build_rust
    Updating crates.io index
 Downloading crates ...
  Downloaded clap v4.5.36
  Downloaded clap_builder v4.5.36
  Downloaded prometheus v0.14.0
  Downloaded axum-extra v0.10.1
  Downloaded async-compression v0.4.23
  Downloaded minijinja v2.9.0
cargo rustc --lib --message-format=json-render-diagnostics --manifest-path common-py/Cargo.toml -v --features extension-module pyo3/extension-module --crate-type cdylib --
       Fresh unicode-ident v1.0.18
       Fresh cfg-if v1.0.0
       Fresh autocfg v1.4.0
       Fresh memchr v2.7.4
       Fresh once_cell v1.21.0
       Fresh value-bag v1.10.0
       Fresh pin-project-lite v0.2.16
       Fresh futures-core v0.3.31
       Fresh bitflags v2.9.0
       Fresh futures-io v0.3.31
       Fresh itoa v1.0.15
       Fresh stable_deref_trait v1.2.0
       Fresh regex-syntax v0.8.5
       Fresh scopeguard v1.2.0
       Fresh shlex v1.3.0
       Fresh foldhash v0.1.4
       Fresh writeable v0.5.5
       Fresh equivalent v1.0.2
       Fresh allocator-api2 v0.2.21
       Fresh litemap v0.7.5
       Fresh bytes v1.10.1
       Fresh icu_locid_transform_data v1.5.0
       Fresh fastrand v2.3.0
       Fresh pin-utils v0.1.0
       Fresh icu_properties_data v1.5.0
       Fresh pkg-config v0.3.32
       Fresh proc-macro2 v1.0.94
       Fresh cc v1.2.16
       Fresh tracing-core v0.1.33
       Fresh hashbrown v0.15.2
       Fresh utf16_iter v1.0.5
       Fresh write16 v1.0.0
       Fresh parking v2.2.1
       Fresh vcpkg v0.2.15
       Fresh atomic-waker v1.1.2
       Fresh icu_normalizer_data v1.5.0
       Fresh ryu v1.0.20
       Fresh linux-raw-sys v0.4.15
       Fresh utf8_iter v1.0.4
       Fresh percent-encoding v2.3.1
       Fresh futures-task v0.3.31
       Fresh log v0.4.27
       Fresh iana-time-zone v0.1.61
       Fresh quote v1.0.39
       Fresh libc v0.2.170
       Fresh crossbeam-utils v0.8.21
       Fresh indexmap v2.8.0
   Compiling openssl-sys v0.9.107
       Fresh rustix v0.38.44
   Compiling openssl v0.10.72
       Fresh version_check v0.9.5
       Fresh foreign-types-shared v0.1.1
       Fresh openssl-probe v0.1.6
       Fresh futures-lite v2.6.0
       Fresh aho-corasick v1.1.3
       Fresh heck v0.5.0
       Fresh home v0.5.11
       Fresh subtle v2.6.1
       Fresh async-task v4.7.1
       Fresh bitflags v1.3.2
       Fresh event-listener v2.5.3
       Fresh piper v0.2.4
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name build_script_main --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/openssl-sys-0.9.107/build/main.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("aws-lc", "bindgen", "bssl-sys", "openssl-src", "unstable_boringssl", "vendored"))' -C metadata=eeb5e148a4101717 -C extra-filename=-24dd128bb08a3f00 --out-dir /home/jelmer/src/janitor/target/debug/build/openssl-sys-24dd128bb08a3f00 -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern cc=/home/jelmer/src/janitor/target/debug/deps/libcc-fd7cf0a089fb2425.rlib --extern pkg_config=/home/jelmer/src/janitor/target/debug/deps/libpkg_config-4ab588afd44f44b3.rlib --extern vcpkg=/home/jelmer/src/janitor/target/debug/deps/libvcpkg-a8ffa4005601983f.rlib --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name build_script_build --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/openssl-0.10.72/build.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("aws-lc", "bindgen", "default", "unstable_boringssl", "v101", "v102", "v110", "v111", "vendored"))' -C metadata=cb2cf903fad6ac04 -C extra-filename=-a00aa62adb2944e3 --out-dir /home/jelmer/src/janitor/target/debug/build/openssl-a00aa62adb2944e3 -L dependency=/home/jelmer/src/janitor/target/debug/deps --cap-lints allow --cfg tokio_unstable`
       Fresh syn v2.0.100
       Fresh slab v0.4.9
       Fresh lock_api v0.4.12
       Fresh concurrent-queue v2.5.0
       Fresh getrandom v0.2.15
       Fresh target-lexicon v0.12.16
       Fresh zerocopy v0.8.23
       Fresh foreign-types v0.3.2
       Fresh regex-automata v0.4.9
       Fresh linux-raw-sys v0.9.2
       Fresh socket2 v0.5.8
       Fresh mio v1.0.3
       Fresh signal-hook-registry v1.4.2
       Fresh linux-raw-sys v0.3.8
       Fresh cpufeatures v0.2.17
       Fresh waker-fn v1.2.0
       Fresh fastrand v1.9.0
       Fresh async-lock v2.8.0
       Fresh socket2 v0.4.10
       Fresh serde_derive v1.0.219
       Fresh synstructure v0.13.1
       Fresh zerovec-derive v0.10.3
       Fresh displaydoc v0.2.5
       Fresh tracing-attributes v0.1.28
       Fresh icu_provider_macros v1.5.0
       Fresh thiserror-impl v2.0.12
       Fresh openssl-macros v0.1.1
       Fresh futures-macro v0.3.31
       Fresh typenum v1.18.0
       Fresh ppv-lite86 v0.2.21
       Fresh event-listener v5.4.0
       Fresh rand_core v0.6.4
       Fresh regex v1.11.1
       Fresh rustix v1.0.2
       Fresh async-executor v1.13.1
       Fresh tokio-macros v2.5.0
       Fresh futures-lite v1.13.0
       Fresh async-channel v1.9.0
       Fresh powerfmt v0.2.0
       Fresh num-conv v0.1.0
       Fresh time-core v0.1.3
       Fresh serde v1.0.219
       Fresh zerofrom-derive v0.1.6
       Fresh yoke-derive v0.7.5
       Fresh thiserror v2.0.12
       Fresh generic-array v0.14.7
       Fresh event-listener-strategy v0.5.3
       Fresh rand_chacha v0.3.1
       Fresh io-lifetimes v1.0.11
       Fresh tracing v0.1.41
       Fresh time-macros v0.2.20
       Fresh tinyvec_macros v0.1.1
       Fresh unicase v2.8.1
       Fresh crc-catalog v2.4.0
       Fresh crossbeam-queue v0.3.12
       Fresh hashlink v0.10.0
       Fresh futures-sink v0.3.31
       Fresh fnv v1.0.7
       Fresh thiserror-impl v1.0.69
       Fresh form_urlencoded v1.2.1
       Fresh hex v0.4.3
       Fresh unicode-bidi v0.3.18
       Fresh zerofrom v0.1.6
       Fresh serde_json v1.0.140
       Fresh pyo3-build-config v0.22.6
       Fresh crypto-common v0.1.6
       Fresh block-buffer v0.10.4
       Fresh async-lock v3.4.0
       Fresh smallvec v1.14.0
       Fresh async-channel v2.3.1
       Fresh rustix v0.37.28
       Fresh deranged v0.3.11
       Fresh crc v3.2.1
       Fresh tinyvec v1.9.0
       Fresh num-traits v0.2.19
       Fresh http v1.2.0
       Fresh polling v3.7.4
       Fresh indoc v2.0.6
       Fresh unindent v0.2.4
       Fresh unicode-properties v0.1.3
       Fresh futures-util v0.3.31
       Fresh either v1.15.0
       Fresh dotenvy v0.15.7
       Fresh byteorder v1.5.0
       Fresh yoke v0.7.5
       Fresh digest v0.10.7
       Fresh blocking v1.6.1
       Fresh parking_lot_core v0.9.10
       Fresh time v0.3.39
       Fresh unicode-normalization v0.1.24
       Fresh memoffset v0.9.1
       Fresh thiserror v1.0.69
       Fresh async-io v2.4.0
       Fresh chrono v0.4.40
       Fresh whoami v1.5.2
       Fresh http-body v1.0.1
       Fresh futures-channel v0.3.31
       Fresh polling v2.8.0
       Fresh kv-log-macro v1.0.7
       Fresh countme v3.0.1
       Fresh try-lock v0.2.5
       Fresh text-size v1.1.1
       Fresh hashbrown v0.14.5
       Fresh base64 v0.22.1
       Fresh siphasher v1.0.1
       Fresh mime v0.3.17
       Fresh zerovec v0.10.4
       Fresh sha2 v0.10.8
       Fresh parking_lot v0.12.3
       Fresh hmac v0.12.1
       Fresh stringprep v0.1.5
       Fresh md-5 v0.10.6
       Fresh async-global-executor v2.4.1
       Fresh rustc-hash v1.1.0
       Fresh async-io v1.13.0
       Fresh want v0.3.1
       Fresh rand v0.8.5
       Fresh tower-service v0.3.3
       Fresh lazy-regex-proc_macros v3.4.1
       Fresh atoi v2.0.0
       Fresh http-body-util v0.1.3
       Fresh sync_wrapper v1.0.2
       Fresh utf8parse v0.2.2
       Fresh tower-layer v0.3.3
       Fresh rustls-pki-types v1.11.0
       Fresh same-file v1.0.6
       Fresh serde_urlencoded v0.7.1
       Fresh deb822-derive v0.2.0
       Fresh tinystr v0.7.6
       Fresh icu_collections v1.5.0
   Compiling tokio v1.44.2
       Fresh pyo3-macros-backend v0.22.6
       Fresh pyo3-ffi v0.22.6
       Fresh hkdf v0.12.4
       Fresh httparse v1.10.1
       Fresh futures-intrusive v0.5.0
       Fresh async-std v1.13.1
       Fresh lazy-regex v3.4.1
       Fresh rowan v0.16.1
       Fresh rustls-pemfile v2.2.0
       Fresh walkdir v2.5.0
       Fresh anstyle-parse v0.2.6
       Fresh encoding_rs v0.8.35
       Fresh is_terminal_polyfill v1.70.1
       Fresh ipnet v2.11.0
       Fresh adler2 v2.0.0
       Fresh anstyle-query v1.1.2
       Fresh colorchoice v1.0.3
       Fresh unicode-width v0.2.0
       Fresh anstyle v1.0.10
       Fresh phf_generator v0.11.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.44.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(loom)' --check-cfg 'cfg(mio_unsupported_force_poll_poll)' --check-cfg 'cfg(tokio_allow_from_blocking_fd)' --check-cfg 'cfg(tokio_internal_mt_counters)' --check-cfg 'cfg(tokio_no_parking_lot)' --check-cfg 'cfg(tokio_no_tuning_tests)' --check-cfg 'cfg(tokio_taskdump)' --check-cfg 'cfg(tokio_unstable)' --cfg 'feature="bytes"' --cfg 'feature="default"' --cfg 'feature="fs"' --cfg 'feature="io-util"' --cfg 'feature="libc"' --cfg 'feature="macros"' --cfg 'feature="mio"' --cfg 'feature="net"' --cfg 'feature="parking_lot"' --cfg 'feature="process"' --cfg 'feature="rt"' --cfg 'feature="rt-multi-thread"' --cfg 'feature="signal-hook-registry"' --cfg 'feature="socket2"' --cfg 'feature="sync"' --cfg 'feature="time"' --cfg 'feature="tokio-macros"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("bytes", "default", "fs", "full", "io-std", "io-util", "libc", "macros", "mio", "net", "parking_lot", "process", "rt", "rt-multi-thread", "signal", "signal-hook-registry", "socket2", "sync", "test-util", "time", "tokio-macros", "tracing", "windows-sys"))' -C metadata=0ce47db22393ab31 -C extra-filename=-2214166d5fa77833 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-5457d7c65e6ff7c6.rmeta --extern mio=/home/jelmer/src/janitor/target/debug/deps/libmio-ea8f193d550eeb3d.rmeta --extern parking_lot=/home/jelmer/src/janitor/target/debug/deps/libparking_lot-e2b62b5be6a25198.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern signal_hook_registry=/home/jelmer/src/janitor/target/debug/deps/libsignal_hook_registry-0134a4b6a31e32fc.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-631c96f5856b2ef3.rmeta --extern tokio_macros=/home/jelmer/src/janitor/target/debug/deps/libtokio_macros-6d6e842210b98dca.so --cap-lints allow --cfg tokio_unstable`
       Fresh icu_locid v1.5.0
       Fresh pyo3-macros v0.22.6
       Fresh getrandom v0.3.1
       Fresh miniz_oxide v0.8.5
       Fresh anstream v0.6.18
       Fresh lazy_static v1.5.0
       Fresh phf_codegen v0.11.3
       Fresh protobuf-support v3.7.2
       Fresh which v4.4.2
       Fresh parse-zoneinfo v0.3.1
       Fresh inotify-sys v0.1.5
       Fresh ucd-trie v0.1.7
       Fresh clap_lex v0.7.4
       Fresh strsim v0.11.1
       Fresh smawk v0.3.2
       Fresh unicode-linebreak v0.1.5
       Fresh gimli v0.31.1
       Fresh unicode-xid v0.2.6
       Fresh clap_derive v4.5.32
       Fresh num-integer v0.1.46
       Fresh icu_provider v1.5.0
       Fresh pyo3 v0.22.6
       Fresh anyhow v1.0.97
       Fresh syn v1.0.109
       Fresh addr2line v0.24.2
   Compiling protobuf v2.28.0
       Fresh pest v2.7.15
       Fresh textwrap v0.16.2
       Fresh chrono-tz-build v0.3.0
   Compiling clap_builder v4.5.36
       Fresh inotify v0.9.6
       Fresh crossbeam-epoch v0.9.18
       Fresh filetime v0.2.25
   Compiling crossbeam-channel v0.5.15
       Fresh mio v0.8.11
       Fresh bstr v1.11.3
       Fresh rustc-demangle v0.1.24
       Fresh unic-common v0.9.0
       Fresh unic-char-range v0.9.0
       Fresh dtor-proc-macro v0.0.5
       Fresh base64ct v1.7.1
       Fresh num-bigint v0.4.6
       Fresh tempfile v3.19.0
       Fresh rowan v0.15.16
     Running `/home/jelmer/src/janitor/target/debug/build/protobuf-d3946e4b022496ee/build-script-build`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name clap_builder --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/clap_builder-4.5.36/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::zero_sized_map_values' '--warn=clippy::verbose_file_reads' --warn=unused_qualifications --warn=unused_macro_rules --warn=unused_lifetimes --warn=unsafe_op_in_unsafe_fn --warn=unreachable_pub '--warn=clippy::uninlined_format_args' '--warn=clippy::trait_duplication_in_bounds' '--warn=clippy::todo' '--warn=clippy::string_lit_as_bytes' '--warn=clippy::string_add_assign' '--warn=clippy::semicolon_if_nothing_returned' '--warn=clippy::self_named_module_files' '--warn=clippy::same_functions_in_if_condition' '--warn=clippy::rest_pat_in_fully_bound_structs' '--warn=clippy::ref_option_ref' '--warn=clippy::redundant_feature_names' '--warn=clippy::rc_mutex' '--warn=clippy::ptr_as_ptr' '--warn=clippy::path_buf_push_overwrite' '--warn=clippy::negative_feature_names' '--warn=clippy::needless_for_each' '--warn=clippy::needless_continue' '--warn=clippy::mutex_integer' '--allow=clippy::multiple_bound_locations' '--warn=clippy::mem_forget' '--warn=clippy::macro_use_imports' '--warn=clippy::lossy_float_literal' '--warn=clippy::linkedlist' '--allow=clippy::let_and_return' '--warn=clippy::large_types_passed_by_value' '--warn=clippy::large_stack_arrays' '--warn=clippy::large_digit_groups' '--warn=clippy::invalid_upcast_comparisons' '--warn=clippy::infinite_loop' '--warn=clippy::inefficient_to_string' '--warn=clippy::inconsistent_struct_constructor' '--warn=clippy::imprecise_flops' '--warn=clippy::implicit_clone' '--allow=clippy::if_same_then_else' '--warn=clippy::from_iter_instead_of_collect' '--warn=clippy::fn_params_excessive_bools' '--warn=clippy::float_cmp_const' '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::fallible_impl_from' '--warn=clippy::explicit_into_iter_loop' '--warn=clippy::explicit_deref_methods' '--warn=clippy::expl_impl_clone_on_copy' '--warn=clippy::enum_glob_use' '--warn=clippy::empty_enum' '--warn=clippy::doc_markdown' '--warn=clippy::debug_assert_with_mut_call' '--warn=clippy::dbg_macro' '--warn=clippy::create_dir' '--allow=clippy::collapsible_else_if' '--warn=clippy::checked_conversions' '--allow=clippy::branches_sharing_code' '--allow=clippy::bool_assert_comparison' '--allow=clippy::blocks_in_conditions' '--allow=clippy::assigning_clones' --cfg 'feature="color"' --cfg 'feature="env"' --cfg 'feature="error-context"' --cfg 'feature="help"' --cfg 'feature="std"' --cfg 'feature="suggestions"' --cfg 'feature="usage"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cargo", "color", "debug", "default", "deprecated", "env", "error-context", "help", "std", "string", "suggestions", "unicode", "unstable-doc", "unstable-ext", "unstable-styles", "unstable-v5", "usage", "wrap_help"))' -C metadata=0fac24f1c8fe573f -C extra-filename=-df290e7e51aef50f --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern anstream=/home/jelmer/src/janitor/target/debug/deps/libanstream-2daa15f4ec64371e.rmeta --extern anstyle=/home/jelmer/src/janitor/target/debug/deps/libanstyle-3491f347c6e7c6e0.rmeta --extern clap_lex=/home/jelmer/src/janitor/target/debug/deps/libclap_lex-8eff0cda03ec45d2.rmeta --extern strsim=/home/jelmer/src/janitor/target/debug/deps/libstrsim-73001e6240a43464.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name crossbeam_channel --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/crossbeam-channel-0.5.15/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs '--allow=clippy::lint_groups_priority' '--allow=clippy::declare_interior_mutable_const' --check-cfg 'cfg(crossbeam_loom)' --check-cfg 'cfg(crossbeam_sanitize)' --cfg 'feature="default"' --cfg 'feature="std"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "std"))' -C metadata=1cf57fcdf74e7f15 -C extra-filename=-4b40e4830c191e81 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern crossbeam_utils=/home/jelmer/src/janitor/target/debug/deps/libcrossbeam_utils-b30db245c4fdf551.rmeta --cap-lints allow --cfg tokio_unstable`
       Fresh icu_locid_transform v1.5.0
       Fresh deb822-lossless v0.2.4
       Fresh synstructure v0.12.6
       Fresh protobuf v3.7.2
       Fresh object v0.36.7
       Fresh libm v0.2.11
       Fresh crossbeam-deque v0.8.6
       Fresh pem-rfc7468 v0.7.0
       Fresh pest_meta v2.7.15
       Fresh unic-ucd-version v0.9.0
       Fresh unic-char-property v0.9.0
       Fresh globset v0.4.16
       Fresh dtor v0.0.5
       Fresh phf_shared v0.11.3
       Fresh async-trait v0.1.88
       Fresh atty v0.2.14
       Fresh csv-core v0.1.12
       Fresh quick-error v1.2.3
       Fresh minimal-lexical v0.2.1
       Fresh difflib v0.4.0
       Fresh zeroize v1.8.1
       Fresh urlencoding v2.1.3
       Fresh icu_properties v1.5.1
       Fresh backtrace v0.3.74
       Fresh failure_derive v0.1.8
       Fresh const-oid v0.9.6
       Fresh termcolor v1.4.1
       Fresh untrusted v0.9.0
       Fresh ctor-proc-macro v0.0.5
       Fresh protobuf-parse v3.7.2
       Fresh csv v1.3.1
       Fresh humansize v2.1.3
       Fresh unic-ucd-segment v0.9.0
       Fresh nom v7.1.3
       Fresh phf v0.11.3
       Fresh ignore v0.4.23
       Fresh pest_generator v2.7.15
       Fresh humantime v1.3.0
       Fresh simple_asn1 v0.6.3
       Fresh pyo3-filelike v0.4.1
       Fresh patchkit v0.2.1
       Fresh rand_core v0.9.3
       Fresh protoc v2.28.0
       Fresh version-ranges v0.1.1
       Fresh icu_normalizer v1.5.0
       Fresh ctor v0.4.1
       Fresh der v0.7.9
       Fresh ring v0.17.13
       Fresh failure v0.1.8
       Fresh pem v3.0.5
       Fresh env_filter v0.1.3
       Fresh crc32fast v1.4.2
       Fresh bit-vec v0.8.0
       Fresh winnow v0.7.3
       Fresh unsafe-libyaml v0.2.11
       Fresh unscanny v0.1.0
       Fresh jiff v0.2.4
       Fresh toml_datetime v0.6.8
       Fresh deunicode v1.6.0
       Fresh maplit v1.0.2
       Fresh mime_guess v2.0.5
       Fresh semver v1.0.26
       Fresh askama_parser v0.2.1
       Fresh pest_derive v2.7.15
       Fresh protobuf-codegen v3.7.2
       Fresh rand_chacha v0.9.0
       Fresh globwalk v0.9.1
       Fresh unic-segment v0.9.0
       Fresh idna_adapter v1.2.0
       Fresh bit-set v0.8.0
       Fresh serde_yaml v0.9.34+deprecated
       Fresh env_logger v0.11.7
       Fresh flate2 v1.1.0
       Fresh jsonwebtoken v9.3.1
       Fresh pep440_rs v0.7.3
       Fresh toml_edit v0.22.24
       Fresh slug v0.1.6
       Fresh spki v0.7.3
       Fresh distro-info v0.4.0
       Fresh env_logger v0.7.1
       Fresh chrono-tz v0.9.0
       Fresh google-cloud-token v0.1.2
       Fresh makefile-lossless v0.1.7
       Fresh sha1 v0.10.6
       Fresh itertools v0.13.0
       Fresh basic-toml v0.1.10
       Fresh futures-executor v0.3.31
       Fresh async-stream-impl v0.3.6
       Fresh rustc-hash v2.1.1
       Fresh base64 v0.21.7
   Compiling prometheus v0.14.0
       Fresh configparser v3.1.0
       Fresh boxcar v0.2.10
       Fresh idna v1.0.3
       Fresh humantime v2.1.0
       Fresh askama_derive v0.12.5
       Fresh fancy-regex v0.14.0
       Fresh async-stream v0.3.6
       Fresh tera v1.20.0
       Fresh futures v0.3.31
       Fresh pretty_env_logger v0.4.0
       Fresh pkcs8 v0.10.2
       Fresh rand v0.9.0
       Fresh toml v0.5.11
       Fresh askama_escape v0.10.3
       Fresh inventory v0.3.20
       Fresh xdg v2.5.2
       Fresh arc-swap v1.7.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name build_script_build --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/prometheus-0.14.0/build.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no --cfg 'feature="default"' --cfg 'feature="protobuf"' --cfg 'feature="reqwest"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "gen", "libc", "nightly", "process", "procfs", "protobuf", "protobuf-codegen", "push", "reqwest"))' -C metadata=4ffaa0827facffb5 -C extra-filename=-c68351df9af92dee --out-dir /home/jelmer/src/janitor/target/debug/build/prometheus-c68351df9af92dee -L dependency=/home/jelmer/src/janitor/target/debug/deps --cap-lints allow --cfg tokio_unstable`
       Fresh url v2.5.4
       Fresh env_logger v0.9.3
       Fresh askama v0.12.1
       Fresh pyo3-log v0.11.0
       Fresh dep3 v0.1.28
       Fresh pep508_rs v0.9.2
       Fresh stackdriver_logger v0.8.2
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name protobuf --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/protobuf-2.28.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("bytes", "serde", "serde_derive", "with-bytes", "with-serde"))' -C metadata=984831cd4ba499b0 -C extra-filename=-98c17e40d8ea6aed --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --cap-lints allow --cfg tokio_unstable --cfg rustc_nightly`
   Compiling notify v6.1.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name notify --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/notify-6.1.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="crossbeam-channel"' --cfg 'feature="default"' --cfg 'feature="fsevent-sys"' --cfg 'feature="macos_fsevent"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("crossbeam-channel", "default", "fsevent-sys", "kqueue", "macos_fsevent", "macos_kqueue", "manual_tests", "mio", "serde", "timing_tests"))' -C metadata=e2691b69d8545734 -C extra-filename=-cd7a16e9f1336343 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern crossbeam_channel=/home/jelmer/src/janitor/target/debug/deps/libcrossbeam_channel-4b40e4830c191e81.rmeta --extern filetime=/home/jelmer/src/janitor/target/debug/deps/libfiletime-74a11c28d8d6d9ef.rmeta --extern inotify=/home/jelmer/src/janitor/target/debug/deps/libinotify-e1c9915788fbbdb4.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-5457d7c65e6ff7c6.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern mio=/home/jelmer/src/janitor/target/debug/deps/libmio-a24c639ec4c3f005.rmeta --extern walkdir=/home/jelmer/src/janitor/target/debug/deps/libwalkdir-f95d3688eab8bd63.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling protobuf-codegen v2.28.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name protobuf_codegen --edition=2015 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/protobuf-codegen-2.28.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=b8b4a18362452533 -C extra-filename=-13f80eb668e85f15 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-98c17e40d8ea6aed.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling dirty-tracker v0.3.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name dirty_tracker --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/dirty-tracker-0.3.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=0654f6c36a0f2845 -C extra-filename=-15c2a709d36ea33e --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern notify=/home/jelmer/src/janitor/target/debug/deps/libnotify-cd7a16e9f1336343.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-34844f972d674a37.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling clap v4.5.36
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name clap --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/clap-4.5.36/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::zero_sized_map_values' '--warn=clippy::verbose_file_reads' --warn=unused_qualifications --warn=unused_macro_rules --warn=unused_lifetimes --warn=unsafe_op_in_unsafe_fn --warn=unreachable_pub '--warn=clippy::uninlined_format_args' '--warn=clippy::trait_duplication_in_bounds' '--warn=clippy::todo' '--warn=clippy::string_lit_as_bytes' '--warn=clippy::string_add_assign' '--warn=clippy::semicolon_if_nothing_returned' '--warn=clippy::self_named_module_files' '--warn=clippy::same_functions_in_if_condition' '--warn=clippy::rest_pat_in_fully_bound_structs' '--warn=clippy::ref_option_ref' '--warn=clippy::redundant_feature_names' '--warn=clippy::rc_mutex' '--warn=clippy::ptr_as_ptr' '--warn=clippy::path_buf_push_overwrite' '--warn=clippy::negative_feature_names' '--warn=clippy::needless_for_each' '--warn=clippy::needless_continue' '--warn=clippy::mutex_integer' '--allow=clippy::multiple_bound_locations' '--warn=clippy::mem_forget' '--warn=clippy::macro_use_imports' '--warn=clippy::lossy_float_literal' '--warn=clippy::linkedlist' '--allow=clippy::let_and_return' '--warn=clippy::large_types_passed_by_value' '--warn=clippy::large_stack_arrays' '--warn=clippy::large_digit_groups' '--warn=clippy::invalid_upcast_comparisons' '--warn=clippy::infinite_loop' '--warn=clippy::inefficient_to_string' '--warn=clippy::inconsistent_struct_constructor' '--warn=clippy::imprecise_flops' '--warn=clippy::implicit_clone' '--allow=clippy::if_same_then_else' '--warn=clippy::from_iter_instead_of_collect' '--warn=clippy::fn_params_excessive_bools' '--warn=clippy::float_cmp_const' '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::fallible_impl_from' '--warn=clippy::explicit_into_iter_loop' '--warn=clippy::explicit_deref_methods' '--warn=clippy::expl_impl_clone_on_copy' '--warn=clippy::enum_glob_use' '--warn=clippy::empty_enum' '--warn=clippy::doc_markdown' '--warn=clippy::debug_assert_with_mut_call' '--warn=clippy::dbg_macro' '--warn=clippy::create_dir' '--allow=clippy::collapsible_else_if' '--warn=clippy::checked_conversions' '--allow=clippy::branches_sharing_code' '--allow=clippy::bool_assert_comparison' '--allow=clippy::blocks_in_conditions' '--allow=clippy::assigning_clones' --cfg 'feature="color"' --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="env"' --cfg 'feature="error-context"' --cfg 'feature="help"' --cfg 'feature="std"' --cfg 'feature="suggestions"' --cfg 'feature="usage"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cargo", "color", "debug", "default", "deprecated", "derive", "env", "error-context", "help", "std", "string", "suggestions", "unicode", "unstable-derive-ui-tests", "unstable-doc", "unstable-ext", "unstable-markdown", "unstable-styles", "unstable-v5", "usage", "wrap_help"))' -C metadata=f772c7a1edf307df -C extra-filename=-59b401794f276823 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern clap_builder=/home/jelmer/src/janitor/target/debug/deps/libclap_builder-df290e7e51aef50f.rmeta --extern clap_derive=/home/jelmer/src/janitor/target/debug/deps/libclap_derive-daf434ff39723ea2.so --cap-lints allow --cfg tokio_unstable`
   Compiling merge3 v0.2.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name merge3 --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/merge3-0.2.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "default", "patiencediff"))' -C metadata=3e6f163844a4ac5b -C extra-filename=-1c24ac3badc9ba5b --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern difflib=/home/jelmer/src/janitor/target/debug/deps/libdifflib-20593172ebc6932e.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling protoc-rust v2.28.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name protoc_rust --edition=2015 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/protoc-rust-2.28.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=d751fc49caaac348 -C extra-filename=-aaf82484d7bd6338 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-98c17e40d8ea6aed.rmeta --extern protobuf_codegen=/home/jelmer/src/janitor/target/debug/deps/libprotobuf_codegen-13f80eb668e85f15.rmeta --extern protoc=/home/jelmer/src/janitor/target/debug/deps/libprotoc-4b211fff6c1cb7ad.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-b5647751c7f60687.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/src/janitor/target/debug/build/prometheus-c68351df9af92dee/build-script-build`
   Compiling janitor v0.1.0 (/home/jelmer/src/janitor)
     Running `/home/jelmer/src/janitor/target/debug/build/openssl-sys-24dd128bb08a3f00/build-script-main`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name build_script_build --edition=2021 build.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="gcp"' --cfg 'feature="gcs"' --cfg 'feature="stackdriver_logger"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("debian", "default", "gcp", "gcs", "stackdriver_logger"))' -C metadata=0a120410ceeb7d86 -C extra-filename=-5ee49d7480f4d8ea --out-dir /home/jelmer/src/janitor/target/debug/build/janitor-5ee49d7480f4d8ea -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern protobuf_codegen=/home/jelmer/src/janitor/target/debug/deps/libprotobuf_codegen-22fc861be64f6de9.rlib --extern protoc_rust=/home/jelmer/src/janitor/target/debug/deps/libprotoc_rust-aaf82484d7bd6338.rlib --cfg tokio_unstable`
   Compiling native-tls v0.2.14
     Running `/home/jelmer/src/janitor/target/debug/build/openssl-a00aa62adb2944e3/build-script-build`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name openssl_sys --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/openssl-sys-0.9.107/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("aws-lc", "bindgen", "bssl-sys", "openssl-src", "unstable_boringssl", "vendored"))' -C metadata=abb2cd6a79aa8bdc -C extra-filename=-5073eff240dc450b --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-5457d7c65e6ff7c6.rmeta --cap-lints allow --cfg tokio_unstable -l ssl -l crypto --cfg 'osslconf="OPENSSL_NO_IDEA"' --cfg 'osslconf="OPENSSL_NO_SSL3_METHOD"' --cfg openssl --cfg ossl340 --cfg ossl330 --cfg ossl320 --cfg ossl300 --cfg ossl101 --cfg ossl102 --cfg ossl102f --cfg ossl102h --cfg ossl110 --cfg ossl110f --cfg ossl110g --cfg ossl110h --cfg ossl111 --cfg ossl111b --cfg ossl111c --cfg ossl111d --check-cfg 'cfg(osslconf, values("OPENSSL_NO_OCB", "OPENSSL_NO_SM4", "OPENSSL_NO_SEED", "OPENSSL_NO_CHACHA", "OPENSSL_NO_CAST", "OPENSSL_NO_IDEA", "OPENSSL_NO_CAMELLIA", "OPENSSL_NO_RC4", "OPENSSL_NO_BF", "OPENSSL_NO_PSK", "OPENSSL_NO_DEPRECATED_3_0", "OPENSSL_NO_SCRYPT", "OPENSSL_NO_SM3", "OPENSSL_NO_RMD160", "OPENSSL_NO_EC2M", "OPENSSL_NO_OCSP", "OPENSSL_NO_CMS", "OPENSSL_NO_COMP", "OPENSSL_NO_SOCK", "OPENSSL_NO_STDIO", "OPENSSL_NO_EC", "OPENSSL_NO_SSL3_METHOD", "OPENSSL_NO_KRB5", "OPENSSL_NO_TLSEXT", "OPENSSL_NO_SRP", "OPENSSL_NO_RFC3779", "OPENSSL_NO_SHA", "OPENSSL_NO_NEXTPROTONEG", "OPENSSL_NO_ENGINE", "OPENSSL_NO_BUF_FREELISTS", "OPENSSL_NO_RC2"))' --check-cfg 'cfg(openssl)' --check-cfg 'cfg(libressl)' --check-cfg 'cfg(boringssl)' --check-cfg 'cfg(awslc)' --check-cfg 'cfg(libressl250)' --check-cfg 'cfg(libressl251)' --check-cfg 'cfg(libressl252)' --check-cfg 'cfg(libressl261)' --check-cfg 'cfg(libressl270)' --check-cfg 'cfg(libressl271)' --check-cfg 'cfg(libressl273)' --check-cfg 'cfg(libressl280)' --check-cfg 'cfg(libressl281)' --check-cfg 'cfg(libressl291)' --check-cfg 'cfg(libressl310)' --check-cfg 'cfg(libressl321)' --check-cfg 'cfg(libressl332)' --check-cfg 'cfg(libressl340)' --check-cfg 'cfg(libressl350)' --check-cfg 'cfg(libressl360)' --check-cfg 'cfg(libressl361)' --check-cfg 'cfg(libressl370)' --check-cfg 'cfg(libressl380)' --check-cfg 'cfg(libressl381)' --check-cfg 'cfg(libressl382)' --check-cfg 'cfg(libressl390)' --check-cfg 'cfg(libressl400)' --check-cfg 'cfg(libressl410)' --check-cfg 'cfg(ossl101)' --check-cfg 'cfg(ossl102)' --check-cfg 'cfg(ossl102f)' --check-cfg 'cfg(ossl102h)' --check-cfg 'cfg(ossl110)' --check-cfg 'cfg(ossl110f)' --check-cfg 'cfg(ossl110g)' --check-cfg 'cfg(ossl110h)' --check-cfg 'cfg(ossl111)' --check-cfg 'cfg(ossl111b)' --check-cfg 'cfg(ossl111c)' --check-cfg 'cfg(ossl111d)' --check-cfg 'cfg(ossl300)' --check-cfg 'cfg(ossl310)' --check-cfg 'cfg(ossl320)' --check-cfg 'cfg(ossl330)' --check-cfg 'cfg(ossl340)'`
     Running `/home/jelmer/src/janitor/target/debug/build/native-tls-edef69d5d1949eff/build-script-build`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name openssl --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/openssl-0.10.72/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("aws-lc", "bindgen", "default", "unstable_boringssl", "v101", "v102", "v110", "v111", "vendored"))' -C metadata=8be32b73063aebc5 -C extra-filename=-08ecf1416439f2ef --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern cfg_if=/home/jelmer/src/janitor/target/debug/deps/libcfg_if-c30e1cd0415bac84.rmeta --extern foreign_types=/home/jelmer/src/janitor/target/debug/deps/libforeign_types-0bf9645f98990128.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-5457d7c65e6ff7c6.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern openssl_macros=/home/jelmer/src/janitor/target/debug/deps/libopenssl_macros-89150665c9ae34c2.so --extern ffi=/home/jelmer/src/janitor/target/debug/deps/libopenssl_sys-5073eff240dc450b.rmeta --cap-lints allow --cfg tokio_unstable --cfg 'osslconf="OPENSSL_NO_IDEA"' --cfg 'osslconf="OPENSSL_NO_SSL3_METHOD"' --cfg ossl101 --cfg ossl102 --cfg ossl110 --cfg ossl110g --cfg ossl110h --cfg ossl111 --cfg ossl111d --cfg ossl300 --cfg ossl310 --cfg ossl320 --cfg ossl330 --check-cfg 'cfg(osslconf, values("OPENSSL_NO_OCB", "OPENSSL_NO_SM4", "OPENSSL_NO_SEED", "OPENSSL_NO_CHACHA", "OPENSSL_NO_CAST", "OPENSSL_NO_IDEA", "OPENSSL_NO_CAMELLIA", "OPENSSL_NO_RC4", "OPENSSL_NO_BF", "OPENSSL_NO_PSK", "OPENSSL_NO_DEPRECATED_3_0", "OPENSSL_NO_SCRYPT", "OPENSSL_NO_SM3", "OPENSSL_NO_RMD160", "OPENSSL_NO_EC2M", "OPENSSL_NO_OCSP", "OPENSSL_NO_CMS", "OPENSSL_NO_EC", "OPENSSL_NO_ARGON2", "OPENSSL_NO_RC2"))' --check-cfg 'cfg(libressl)' --check-cfg 'cfg(boringssl)' --check-cfg 'cfg(awslc)' --check-cfg 'cfg(libressl250)' --check-cfg 'cfg(libressl251)' --check-cfg 'cfg(libressl261)' --check-cfg 'cfg(libressl270)' --check-cfg 'cfg(libressl271)' --check-cfg 'cfg(libressl273)' --check-cfg 'cfg(libressl280)' --check-cfg 'cfg(libressl291)' --check-cfg 'cfg(libressl310)' --check-cfg 'cfg(libressl321)' --check-cfg 'cfg(libressl332)' --check-cfg 'cfg(libressl340)' --check-cfg 'cfg(libressl350)' --check-cfg 'cfg(libressl360)' --check-cfg 'cfg(libressl361)' --check-cfg 'cfg(libressl370)' --check-cfg 'cfg(libressl380)' --check-cfg 'cfg(libressl382)' --check-cfg 'cfg(libressl390)' --check-cfg 'cfg(libressl400)' --check-cfg 'cfg(libressl410)' --check-cfg 'cfg(ossl101)' --check-cfg 'cfg(ossl102)' --check-cfg 'cfg(ossl110)' --check-cfg 'cfg(ossl110g)' --check-cfg 'cfg(ossl110h)' --check-cfg 'cfg(ossl111)' --check-cfg 'cfg(ossl111d)' --check-cfg 'cfg(ossl300)' --check-cfg 'cfg(ossl310)' --check-cfg 'cfg(ossl320)' --check-cfg 'cfg(ossl330)'`
   Compiling tokio-util v0.7.14
   Compiling tower v0.5.2
   Compiling async-compression v0.4.23
   Compiling pyo3-async-runtimes v0.22.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio_util --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-util-0.7.14/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(loom)' --check-cfg 'cfg(mio_unsupported_force_poll_poll)' --check-cfg 'cfg(tokio_allow_from_blocking_fd)' --check-cfg 'cfg(tokio_internal_mt_counters)' --check-cfg 'cfg(tokio_no_parking_lot)' --check-cfg 'cfg(tokio_no_tuning_tests)' --check-cfg 'cfg(tokio_taskdump)' --check-cfg 'cfg(tokio_unstable)' --cfg 'feature="codec"' --cfg 'feature="default"' --cfg 'feature="io"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__docs_rs", "codec", "compat", "default", "full", "futures-io", "futures-util", "hashbrown", "io", "io-util", "net", "rt", "slab", "time", "tracing"))' -C metadata=8b2697bbf3ce580e -C extra-filename=-9820bdd1ae0640b6 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_sink=/home/jelmer/src/janitor/target/debug/deps/libfutures_sink-0f1aae5d0426fde7.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name pyo3_async_runtimes --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pyo3-async-runtimes-0.22.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="tokio"' --cfg 'feature="tokio-runtime"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("async-channel", "async-std", "async-std-runtime", "attributes", "clap", "default", "inventory", "pyo3-async-runtimes-macros", "testing", "tokio", "tokio-runtime", "unstable-streams"))' -C metadata=14a9abe34be440c6 -C extra-filename=-326186b750d2f5f4 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern futures=/home/jelmer/src/janitor/target/debug/deps/libfutures-463765cbcde647a0.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tower --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tower-0.5.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="__common"' --cfg 'feature="futures-core"' --cfg 'feature="futures-util"' --cfg 'feature="pin-project-lite"' --cfg 'feature="sync_wrapper"' --cfg 'feature="timeout"' --cfg 'feature="tokio"' --cfg 'feature="util"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__common", "balance", "buffer", "discover", "filter", "full", "futures-core", "futures-util", "hdrhistogram", "hedge", "indexmap", "limit", "load", "load-shed", "log", "make", "pin-project-lite", "ready-cache", "reconnect", "retry", "slab", "spawn-ready", "steer", "sync_wrapper", "timeout", "tokio", "tokio-stream", "tokio-util", "tracing", "util"))' -C metadata=d8c41dea5ccb5302 -C extra-filename=-73af22d1aa520722 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rmeta --extern tower_layer=/home/jelmer/src/janitor/target/debug/deps/libtower_layer-fff117c1c28e545a.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name async_compression --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/async-compression-0.4.23/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="flate2"' --cfg 'feature="gzip"' --cfg 'feature="tokio"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("all", "all-algorithms", "all-implementations", "brotli", "bzip2", "deflate", "deflate64", "flate2", "futures-io", "gzip", "libzstd", "lz4", "lzma", "tokio", "xz", "xz2", "zlib", "zstd", "zstd-safe", "zstdmt"))' -C metadata=2edc44cec51be5b2 -C extra-filename=-4d0766f656f1104b --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling h2 v0.4.8
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name h2 --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/h2-0.4.8/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("stream", "unstable"))' -C metadata=ae31c7d6e67ecb16 -C extra-filename=-c6daa318ccbdba51 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern atomic_waker=/home/jelmer/src/janitor/target/debug/deps/libatomic_waker-21f0b624b8878034.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern fnv=/home/jelmer/src/janitor/target/debug/deps/libfnv-135eca59eff18b18.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_sink=/home/jelmer/src/janitor/target/debug/deps/libfutures_sink-0f1aae5d0426fde7.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-9a486ccb6575c0f1.rmeta --extern slab=/home/jelmer/src/janitor/target/debug/deps/libslab-58feeb60e58ddd09.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-9820bdd1ae0640b6.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name native_tls --edition=2015 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/native-tls-0.2.14/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alpn", "vendored"))' -C metadata=25538a9a352d7af9 -C extra-filename=-41e5bdd64bc7cd9e --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern openssl=/home/jelmer/src/janitor/target/debug/deps/libopenssl-08ecf1416439f2ef.rmeta --extern openssl_probe=/home/jelmer/src/janitor/target/debug/deps/libopenssl_probe-81c031c110cf4218.rmeta --extern openssl_sys=/home/jelmer/src/janitor/target/debug/deps/libopenssl_sys-5073eff240dc450b.rmeta --cap-lints allow --cfg tokio_unstable --cfg have_min_max_version --check-cfg 'cfg(have_min_max_version)'`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name native_tls --edition=2015 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/native-tls-0.2.14/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alpn", "vendored"))' -C metadata=ab05cd6e48488def -C extra-filename=-e28561433c9ccd8a --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-400fd68aa602ed65.rmeta --extern openssl=/home/jelmer/src/janitor/target/debug/deps/libopenssl-08ecf1416439f2ef.rmeta --extern openssl_probe=/home/jelmer/src/janitor/target/debug/deps/libopenssl_probe-81c031c110cf4218.rmeta --extern openssl_sys=/home/jelmer/src/janitor/target/debug/deps/libopenssl_sys-5073eff240dc450b.rmeta --cap-lints allow --cfg tokio_unstable --cfg have_min_max_version --check-cfg 'cfg(have_min_max_version)'`
   Compiling sqlx-core v0.8.3
   Compiling tokio-native-tls v0.3.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_core --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-core-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="_rt-async-std"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="any"' --cfg 'feature="async-io"' --cfg 'feature="async-std"' --cfg 'feature="chrono"' --cfg 'feature="crc"' --cfg 'feature="default"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="native-tls"' --cfg 'feature="offline"' --cfg 'feature="serde"' --cfg 'feature="serde_json"' --cfg 'feature="sha2"' --cfg 'feature="time"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_tls-native-tls", "_tls-none", "_tls-rustls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "any", "async-io", "async-std", "bigdecimal", "bit-vec", "bstr", "chrono", "crc", "default", "ipnetwork", "json", "mac_address", "migrate", "native-tls", "offline", "regex", "rust_decimal", "rustls", "rustls-native-certs", "rustls-pemfile", "serde", "serde_json", "sha2", "time", "tokio", "tokio-stream", "uuid", "webpki-roots"))' -C metadata=937436a6d472e0a2 -C extra-filename=-b1311c083eaa4498 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_io=/home/jelmer/src/janitor/target/debug/deps/libasync_io-ab3ec6953b241562.rmeta --extern async_std=/home/jelmer/src/janitor/target/debug/deps/libasync_std-538cf550a53fa4e6.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern crossbeam_queue=/home/jelmer/src/janitor/target/debug/deps/libcrossbeam_queue-577e4d13a58a6351.rmeta --extern either=/home/jelmer/src/janitor/target/debug/deps/libeither-5d949479ced69761.rmeta --extern event_listener=/home/jelmer/src/janitor/target/debug/deps/libevent_listener-55331feab369961e.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_intrusive=/home/jelmer/src/janitor/target/debug/deps/libfutures_intrusive-2f7a96ce78bbdca9.rmeta --extern futures_io=/home/jelmer/src/janitor/target/debug/deps/libfutures_io-40db0a981b134123.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern hashbrown=/home/jelmer/src/janitor/target/debug/deps/libhashbrown-06a88afb9eab536b.rmeta --extern hashlink=/home/jelmer/src/janitor/target/debug/deps/libhashlink-59dbdb8fc63c8797.rmeta --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-9a486ccb6575c0f1.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-41e5bdd64bc7cd9e.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-59505d94661b74c2.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-93a16e2f2de5f6ed.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio_native_tls --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-native-tls-0.3.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("vendored"))' -C metadata=4cb1eb68458d8f2c -C extra-filename=-74c072e1f23ffc24 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-41e5bdd64bc7cd9e.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_core --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-core-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="_rt-async-std"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="any"' --cfg 'feature="async-io"' --cfg 'feature="async-std"' --cfg 'feature="chrono"' --cfg 'feature="crc"' --cfg 'feature="default"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="native-tls"' --cfg 'feature="offline"' --cfg 'feature="serde"' --cfg 'feature="serde_json"' --cfg 'feature="sha2"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_tls-native-tls", "_tls-none", "_tls-rustls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "any", "async-io", "async-std", "bigdecimal", "bit-vec", "bstr", "chrono", "crc", "default", "ipnetwork", "json", "mac_address", "migrate", "native-tls", "offline", "regex", "rust_decimal", "rustls", "rustls-native-certs", "rustls-pemfile", "serde", "serde_json", "sha2", "time", "tokio", "tokio-stream", "uuid", "webpki-roots"))' -C metadata=54d782f50cb27651 -C extra-filename=-8975446723735c81 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_io=/home/jelmer/src/janitor/target/debug/deps/libasync_io-835a56f561c864c0.rmeta --extern async_std=/home/jelmer/src/janitor/target/debug/deps/libasync_std-220d57f9d1d250bf.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-4eb151582e08ecdb.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern crossbeam_queue=/home/jelmer/src/janitor/target/debug/deps/libcrossbeam_queue-577e4d13a58a6351.rmeta --extern either=/home/jelmer/src/janitor/target/debug/deps/libeither-accfca6c6f5e11c6.rmeta --extern event_listener=/home/jelmer/src/janitor/target/debug/deps/libevent_listener-55331feab369961e.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_intrusive=/home/jelmer/src/janitor/target/debug/deps/libfutures_intrusive-d61fea0a40cf5f80.rmeta --extern futures_io=/home/jelmer/src/janitor/target/debug/deps/libfutures_io-40db0a981b134123.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-e2fb2d440b459c82.rmeta --extern hashbrown=/home/jelmer/src/janitor/target/debug/deps/libhashbrown-06a88afb9eab536b.rmeta --extern hashlink=/home/jelmer/src/janitor/target/debug/deps/libhashlink-59dbdb8fc63c8797.rmeta --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-9a486ccb6575c0f1.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-400fd68aa602ed65.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-e28561433c9ccd8a.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-9969bfe2b2f70651.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-f6f36781d1866faf.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-93d7573978769e30.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper v1.6.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-1.6.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(hyper_unstable_tracing)' --check-cfg 'cfg(hyper_unstable_ffi)' --cfg 'feature="client"' --cfg 'feature="default"' --cfg 'feature="http1"' --cfg 'feature="http2"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("capi", "client", "default", "ffi", "full", "http1", "http2", "nightly", "server", "tracing"))' -C metadata=48b10bac1d0ac7ac -C extra-filename=-39c49a085705b8f2 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern h2=/home/jelmer/src/janitor/target/debug/deps/libh2-c6daa318ccbdba51.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern httparse=/home/jelmer/src/janitor/target/debug/deps/libhttparse-de9e4dfe0f78db23.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-59505d94661b74c2.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rmeta --extern want=/home/jelmer/src/janitor/target/debug/deps/libwant-676b1650d2642fde.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper-util v0.1.10
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper_util --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-util-0.1.10/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="client"' --cfg 'feature="client-legacy"' --cfg 'feature="default"' --cfg 'feature="http1"' --cfg 'feature="http2"' --cfg 'feature="tokio"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__internal_happy_eyeballs_tests", "client", "client-legacy", "default", "full", "http1", "http2", "server", "server-auto", "server-graceful", "service", "tokio"))' -C metadata=94a33997f9c8ca47 -C extra-filename=-5d879c30ec986e01 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-39c49a085705b8f2.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-631c96f5856b2ef3.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/src/janitor/target/debug/build/janitor-5ee49d7480f4d8ea/build-script-build`
   Compiling hyper-tls v0.6.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper_tls --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-tls-0.6.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alpn", "vendored"))' -C metadata=aa5260fb16c9e8a7 -C extra-filename=-074c0668bfa7277c --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-39c49a085705b8f2.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-5d879c30ec986e01.rmeta --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-41e5bdd64bc7cd9e.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rmeta --extern tokio_native_tls=/home/jelmer/src/janitor/target/debug/deps/libtokio_native_tls-74c072e1f23ffc24.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling reqwest v0.12.15
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name reqwest --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/reqwest-0.12.15/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(reqwest_unstable)' --cfg 'feature="__tls"' --cfg 'feature="blocking"' --cfg 'feature="charset"' --cfg 'feature="default"' --cfg 'feature="default-tls"' --cfg 'feature="h2"' --cfg 'feature="http2"' --cfg 'feature="json"' --cfg 'feature="macos-system-configuration"' --cfg 'feature="multipart"' --cfg 'feature="stream"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__rustls", "__rustls-ring", "__tls", "blocking", "brotli", "charset", "cookies", "default", "default-tls", "deflate", "gzip", "h2", "hickory-dns", "http2", "http3", "json", "macos-system-configuration", "multipart", "native-tls", "native-tls-alpn", "native-tls-vendored", "rustls-tls", "rustls-tls-manual-roots", "rustls-tls-manual-roots-no-provider", "rustls-tls-native-roots", "rustls-tls-native-roots-no-provider", "rustls-tls-no-provider", "rustls-tls-webpki-roots", "rustls-tls-webpki-roots-no-provider", "socks", "stream", "trust-dns", "zstd"))' -C metadata=a386dd782563bec8 -C extra-filename=-48a1b478844485eb --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-b1a4de7ed4da6927.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern encoding_rs=/home/jelmer/src/janitor/target/debug/deps/libencoding_rs-58c41e4932181cd7.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern h2=/home/jelmer/src/janitor/target/debug/deps/libh2-c6daa318ccbdba51.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-39c49a085705b8f2.rmeta --extern hyper_tls=/home/jelmer/src/janitor/target/debug/deps/libhyper_tls-074c0668bfa7277c.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-5d879c30ec986e01.rmeta --extern ipnet=/home/jelmer/src/janitor/target/debug/deps/libipnet-5873e4e1530bf49f.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern mime=/home/jelmer/src/janitor/target/debug/deps/libmime-c74000db2aaf511f.rmeta --extern mime_guess=/home/jelmer/src/janitor/target/debug/deps/libmime_guess-7ee1813410f2722d.rmeta --extern native_tls_crate=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-41e5bdd64bc7cd9e.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern rustls_pemfile=/home/jelmer/src/janitor/target/debug/deps/librustls_pemfile-68bb2d10b5046659.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_urlencoded=/home/jelmer/src/janitor/target/debug/deps/libserde_urlencoded-e0562f68e1545f98.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rmeta --extern tokio_native_tls=/home/jelmer/src/janitor/target/debug/deps/libtokio_native_tls-74c072e1f23ffc24.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-9820bdd1ae0640b6.rmeta --extern tower=/home/jelmer/src/janitor/target/debug/deps/libtower-73af22d1aa520722.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling google-cloud-metadata v0.5.1
   Compiling reqwest-middleware v0.3.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name google_cloud_metadata --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/google-cloud-metadata-0.5.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=7c16ae93dec07ecb -C extra-filename=-981c54b703973094 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-48a1b478844485eb.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name reqwest_middleware --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/reqwest-middleware-0.3.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="json"' --cfg 'feature="multipart"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("charset", "http2", "json", "multipart", "rustls-tls"))' -C metadata=33a8216d1c6f0b11 -C extra-filename=-5409462546424d86 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern anyhow=/home/jelmer/src/janitor/target/debug/deps/libanyhow-904a89ff6dd1202e.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-48a1b478844485eb.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name prometheus --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/prometheus-0.14.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="protobuf"' --cfg 'feature="reqwest"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "gen", "libc", "nightly", "process", "procfs", "protobuf", "protobuf-codegen", "push", "reqwest"))' -C metadata=f6b8b01f4588287a -C extra-filename=-a2f283677911744d --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern cfg_if=/home/jelmer/src/janitor/target/debug/deps/libcfg_if-c30e1cd0415bac84.rmeta --extern fnv=/home/jelmer/src/janitor/target/debug/deps/libfnv-135eca59eff18b18.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern parking_lot=/home/jelmer/src/janitor/target/debug/deps/libparking_lot-e2b62b5be6a25198.rmeta --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-b1e3dc89f529da22.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-48a1b478844485eb.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling google-cloud-auth v0.17.2
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name google_cloud_auth --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/google-cloud-auth-0.17.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="default-tls"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "default-tls", "external-account", "hex", "hickory-dns", "hmac", "path-clean", "percent-encoding", "rustls-tls", "sha2", "url"))' -C metadata=00e341392d7b9af4 -C extra-filename=-15bdd22f92eb38ea --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-41324bb9dba3dad5.rmeta --extern google_cloud_metadata=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_metadata-981c54b703973094.rmeta --extern google_cloud_token=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_token-6145b7093dd432ee.rmeta --extern home=/home/jelmer/src/janitor/target/debug/deps/libhome-fc8b76e8d1b394eb.rmeta --extern jsonwebtoken=/home/jelmer/src/janitor/target/debug/deps/libjsonwebtoken-18b13ae9cfbca9a3.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-48a1b478844485eb.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-93a16e2f2de5f6ed.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern urlencoding=/home/jelmer/src/janitor/target/debug/deps/liburlencoding-0ba1b8b89d728edb.rmeta --cap-lints allow --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
   Compiling sqlx-postgres v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_postgres --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-postgres-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="chrono"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="offline"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("any", "bigdecimal", "bit-vec", "chrono", "ipnetwork", "json", "mac_address", "migrate", "offline", "rust_decimal", "time", "uuid"))' -C metadata=73e547a56bee8fb2 -C extra-filename=-f1110f9d12059c83 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern atoi=/home/jelmer/src/janitor/target/debug/deps/libatoi-f8455101c6ea3fc4.rmeta --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-bf6eccdff131582a.rmeta --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern byteorder=/home/jelmer/src/janitor/target/debug/deps/libbyteorder-99211d86bad9f8bb.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-4eb151582e08ecdb.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern dotenvy=/home/jelmer/src/janitor/target/debug/deps/libdotenvy-c5aead2ec43889b7.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-1f4beae7161f5951.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-e2fb2d440b459c82.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern hkdf=/home/jelmer/src/janitor/target/debug/deps/libhkdf-46ecc70a36f2ad04.rmeta --extern hmac=/home/jelmer/src/janitor/target/debug/deps/libhmac-d886aec669ee88c9.rmeta --extern home=/home/jelmer/src/janitor/target/debug/deps/libhome-fc8b76e8d1b394eb.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-400fd68aa602ed65.rmeta --extern md5=/home/jelmer/src/janitor/target/debug/deps/libmd5-e96a7bc866d34328.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-c5d47a5e42694f78.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-9969bfe2b2f70651.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-8975446723735c81.rmeta --extern stringprep=/home/jelmer/src/janitor/target/debug/deps/libstringprep-3e84ecdea7cbe138.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-f6f36781d1866faf.rmeta --extern whoami=/home/jelmer/src/janitor/target/debug/deps/libwhoami-aa57d5d7654aa522.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_postgres --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-postgres-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="any"' --cfg 'feature="chrono"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="time"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("any", "bigdecimal", "bit-vec", "chrono", "ipnetwork", "json", "mac_address", "migrate", "offline", "rust_decimal", "time", "uuid"))' -C metadata=b83e9caed06e6f1d -C extra-filename=-928485792788ad80 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern atoi=/home/jelmer/src/janitor/target/debug/deps/libatoi-08701d6ef2ff6341.rmeta --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-b1a4de7ed4da6927.rmeta --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern byteorder=/home/jelmer/src/janitor/target/debug/deps/libbyteorder-99211d86bad9f8bb.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern dotenvy=/home/jelmer/src/janitor/target/debug/deps/libdotenvy-c5aead2ec43889b7.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern hkdf=/home/jelmer/src/janitor/target/debug/deps/libhkdf-46ecc70a36f2ad04.rmeta --extern hmac=/home/jelmer/src/janitor/target/debug/deps/libhmac-d886aec669ee88c9.rmeta --extern home=/home/jelmer/src/janitor/target/debug/deps/libhome-fc8b76e8d1b394eb.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern md5=/home/jelmer/src/janitor/target/debug/deps/libmd5-e96a7bc866d34328.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-c535a7a8ba116747.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-59505d94661b74c2.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-b1311c083eaa4498.rmeta --extern stringprep=/home/jelmer/src/janitor/target/debug/deps/libstringprep-3e84ecdea7cbe138.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-93a16e2f2de5f6ed.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern whoami=/home/jelmer/src/janitor/target/debug/deps/libwhoami-aa57d5d7654aa522.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling google-cloud-storage v0.22.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name google_cloud_storage --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/google-cloud-storage-0.22.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="auth"' --cfg 'feature="default"' --cfg 'feature="default-tls"' --cfg 'feature="google-cloud-auth"' --cfg 'feature="google-cloud-metadata"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("auth", "default", "default-tls", "external-account", "google-cloud-auth", "google-cloud-metadata", "hickory-dns", "rustls-tls", "trace"))' -C metadata=49fa5062acaf80cc -C extra-filename=-67189881ad923dbd --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern anyhow=/home/jelmer/src/janitor/target/debug/deps/libanyhow-904a89ff6dd1202e.rmeta --extern async_stream=/home/jelmer/src/janitor/target/debug/deps/libasync_stream-f0f1e6ef812a7b6c.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-41324bb9dba3dad5.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern google_cloud_auth=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_auth-15bdd22f92eb38ea.rmeta --extern google_cloud_metadata=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_metadata-981c54b703973094.rmeta --extern google_cloud_token=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_token-6145b7093dd432ee.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pkcs8=/home/jelmer/src/janitor/target/debug/deps/libpkcs8-ef54810b56a401a1.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-48a1b478844485eb.rmeta --extern reqwest_middleware=/home/jelmer/src/janitor/target/debug/deps/libreqwest_middleware-5409462546424d86.rmeta --extern ring=/home/jelmer/src/janitor/target/debug/deps/libring-1446534c300ac753.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-93a16e2f2de5f6ed.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
   Compiling sqlx-macros-core v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_macros_core --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-macros-core-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no --warn=unexpected_cfgs --cfg 'feature="_rt-async-std"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="async-std"' --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="json"' --cfg 'feature="macros"' --cfg 'feature="migrate"' --cfg 'feature="postgres"' --cfg 'feature="sqlx-postgres"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_sqlite", "_tls-native-tls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "async-std", "bigdecimal", "bit-vec", "chrono", "default", "derive", "ipnetwork", "json", "mac_address", "macros", "migrate", "mysql", "postgres", "rust_decimal", "sqlite", "sqlite-unbundled", "sqlx-mysql", "sqlx-postgres", "sqlx-sqlite", "time", "tokio", "uuid"))' -C metadata=90b7cac1f74046dc -C extra-filename=-d7356c01e38d72c2 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_std=/home/jelmer/src/janitor/target/debug/deps/libasync_std-220d57f9d1d250bf.rmeta --extern dotenvy=/home/jelmer/src/janitor/target/debug/deps/libdotenvy-c5aead2ec43889b7.rmeta --extern either=/home/jelmer/src/janitor/target/debug/deps/libeither-accfca6c6f5e11c6.rmeta --extern heck=/home/jelmer/src/janitor/target/debug/deps/libheck-4d6a9c8516811f18.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern proc_macro2=/home/jelmer/src/janitor/target/debug/deps/libproc_macro2-a7e2001652539cec.rmeta --extern quote=/home/jelmer/src/janitor/target/debug/deps/libquote-8533776b6f1db290.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-8975446723735c81.rmeta --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-f1110f9d12059c83.rmeta --extern syn=/home/jelmer/src/janitor/target/debug/deps/libsyn-7fe0b75e1b133791.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-b5647751c7f60687.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-93d7573978769e30.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling sqlx-macros v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_macros --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-macros-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type proc-macro --emit=dep-info,link -C prefer-dynamic -C embed-bitcode=no '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="_rt-async-std"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="json"' --cfg 'feature="macros"' --cfg 'feature="migrate"' --cfg 'feature="postgres"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_tls-native-tls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "bigdecimal", "bit-vec", "chrono", "default", "derive", "ipnetwork", "json", "mac_address", "macros", "migrate", "mysql", "postgres", "rust_decimal", "sqlite", "sqlite-unbundled", "time", "uuid"))' -C metadata=d6043751c2215b43 -C extra-filename=-1379cc8b3feb9ea0 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern proc_macro2=/home/jelmer/src/janitor/target/debug/deps/libproc_macro2-a7e2001652539cec.rlib --extern quote=/home/jelmer/src/janitor/target/debug/deps/libquote-8533776b6f1db290.rlib --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-8975446723735c81.rlib --extern sqlx_macros_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_macros_core-d7356c01e38d72c2.rlib --extern syn=/home/jelmer/src/janitor/target/debug/deps/libsyn-7fe0b75e1b133791.rlib --extern proc_macro --cap-lints allow --cfg tokio_unstable`
   Compiling sqlx v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="_rt-async-std"' --cfg 'feature="any"' --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="json"' --cfg 'feature="macros"' --cfg 'feature="migrate"' --cfg 'feature="postgres"' --cfg 'feature="runtime-async-std"' --cfg 'feature="runtime-async-std-native-tls"' --cfg 'feature="sqlx-macros"' --cfg 'feature="sqlx-postgres"' --cfg 'feature="tls-native-tls"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_sqlite", "_unstable-all-types", "all-databases", "any", "bigdecimal", "bit-vec", "chrono", "default", "derive", "ipnetwork", "json", "mac_address", "macros", "migrate", "mysql", "postgres", "regexp", "runtime-async-std", "runtime-async-std-native-tls", "runtime-async-std-rustls", "runtime-tokio", "runtime-tokio-native-tls", "runtime-tokio-rustls", "rust_decimal", "sqlite", "sqlite-unbundled", "sqlx-macros", "sqlx-mysql", "sqlx-postgres", "sqlx-sqlite", "time", "tls-native-tls", "tls-none", "tls-rustls", "tls-rustls-aws-lc-rs", "tls-rustls-ring", "tls-rustls-ring-native-roots", "tls-rustls-ring-webpki", "uuid"))' -C metadata=0e4f0cc6168fa188 -C extra-filename=-02615dedbab651d7 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-b1311c083eaa4498.rmeta --extern sqlx_macros=/home/jelmer/src/janitor/target/debug/deps/libsqlx_macros-1379cc8b3feb9ea0.so --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-928485792788ad80.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling debversion v0.4.4
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debversion --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debversion-0.4.4/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="python-debian"' --cfg 'feature="serde"' --cfg 'feature="sqlx"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "python-debian", "serde", "sqlx"))' -C metadata=df5c7a2a4c8ed394 -C extra-filename=-40db761eabe70986 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-02615dedbab651d7.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling debian-control v0.1.41
   Compiling debian-changelog v0.2.0
   Compiling debian-copyright v0.1.27
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_control --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-control-0.1.41/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="lossless"' --cfg 'feature="python-debian"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("chrono", "default", "lossless", "python-debian", "serde"))' -C metadata=addf6f34c5180ac6 -C extra-filename=-9f2af6068106d2fb --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-85e845bd7914badc.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-40db761eabe70986.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern rowan=/home/jelmer/src/janitor/target/debug/deps/librowan-e05f61e8ea0a6615.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_changelog --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-changelog-0.2.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=ef7b7f8d745c9d0d -C extra-filename=-83daf3b37151d30c --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-40db761eabe70986.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern rowan=/home/jelmer/src/janitor/target/debug/deps/librowan-c1fdca08b3081a85.rmeta --extern textwrap=/home/jelmer/src/janitor/target/debug/deps/libtextwrap-5e0992fd5b607969.rmeta --extern whoami=/home/jelmer/src/janitor/target/debug/deps/libwhoami-aa57d5d7654aa522.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_copyright --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-copyright-0.1.27/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=3abaf0c6d709b99f -C extra-filename=-48cae804b27ee72b --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-85e845bd7914badc.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-40db761eabe70986.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling breezyshim v0.1.227
   Compiling buildlog-consultant v0.1.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name breezyshim --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/breezyshim-0.1.227/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="auto-initialize"' --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="dirty-tracker"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("auto-initialize", "debian", "default", "dirty-tracker", "sqlx"))' -C metadata=0d77b645458296f3 -C extra-filename=-9d8884674d468014 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern ctor=/home/jelmer/src/janitor/target/debug/deps/libctor-72258acac2d0b9ee.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-83daf3b37151d30c.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-9f2af6068106d2fb.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-40db761eabe70986.rmeta --extern difflib=/home/jelmer/src/janitor/target/debug/deps/libdifflib-20593172ebc6932e.rmeta --extern dirty_tracker=/home/jelmer/src/janitor/target/debug/deps/libdirty_tracker-15c2a709d36ea33e.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern patchkit=/home/jelmer/src/janitor/target/debug/deps/libpatchkit-045f9ba36150b582.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern pyo3_filelike=/home/jelmer/src/janitor/target/debug/deps/libpyo3_filelike-bc0667b965758a04.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-34844f972d674a37.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name buildlog_consultant --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/buildlog-consultant-0.1.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("chatgpt", "cli", "default", "tokio"))' -C metadata=0a22bf2a4adabb02 -C extra-filename=-87cd661227323dc7 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-9f2af6068106d2fb.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-40db761eabe70986.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-808eaebb057ce37e.rmeta --extern fancy_regex=/home/jelmer/src/janitor/target/debug/deps/libfancy_regex-0688edb11485e39a.rmeta --extern inventory=/home/jelmer/src/janitor/target/debug/deps/libinventory-97a54ddffe78909c.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern pep508_rs=/home/jelmer/src/janitor/target/debug/deps/libpep508_rs-9aa259a9ee5b2c33.rlib --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_yaml=/home/jelmer/src/janitor/target/debug/deps/libserde_yaml-d19b32863dd48a61.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern text_size=/home/jelmer/src/janitor/target/debug/deps/libtext_size-68834c6d82d5a146.rmeta --extern textwrap=/home/jelmer/src/janitor/target/debug/deps/libtextwrap-5e0992fd5b607969.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling debian-analyzer v0.158.25
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_analyzer --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-analyzer-0.158.25/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="merge3"' --cfg 'feature="python"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "default", "merge3", "python", "svp", "udd"))' -C metadata=e3d58edb9d23b6b9 -C extra-filename=-523cb2c460b2954a --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-9d8884674d468014.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern configparser=/home/jelmer/src/janitor/target/debug/deps/libconfigparser-aaa60c0f437f3031.rmeta --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-85e845bd7914badc.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-83daf3b37151d30c.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-9f2af6068106d2fb.rmeta --extern debian_copyright=/home/jelmer/src/janitor/target/debug/deps/libdebian_copyright-48cae804b27ee72b.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-40db761eabe70986.rmeta --extern dep3=/home/jelmer/src/janitor/target/debug/deps/libdep3-cef4e33c810b0205.rmeta --extern difflib=/home/jelmer/src/janitor/target/debug/deps/libdifflib-20593172ebc6932e.rmeta --extern distro_info=/home/jelmer/src/janitor/target/debug/deps/libdistro_info-e2f22ea1e25dba0a.rmeta --extern filetime=/home/jelmer/src/janitor/target/debug/deps/libfiletime-74a11c28d8d6d9ef.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern makefile_lossless=/home/jelmer/src/janitor/target/debug/deps/libmakefile_lossless-f7c86ff44e7ff685.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern merge3=/home/jelmer/src/janitor/target/debug/deps/libmerge3-1c24ac3badc9ba5b.rmeta --extern patchkit=/home/jelmer/src/janitor/target/debug/deps/libpatchkit-045f9ba36150b582.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-48a1b478844485eb.rmeta --extern semver=/home/jelmer/src/janitor/target/debug/deps/libsemver-11fd74ac82b27f0f.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha1=/home/jelmer/src/janitor/target/debug/deps/libsha1-666ba0d12790bffa.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-34844f972d674a37.rmeta --extern toml_edit=/home/jelmer/src/janitor/target/debug/deps/libtoml_edit-51c35483d814c85f.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling silver-platter v0.5.48
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name silver_platter --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/silver-platter-0.5.48/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="detect-update-changelog"' --cfg 'feature="pyo3"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default", "detect-update-changelog", "gpg", "last-attempt-db", "pyo3"))' -C metadata=30abb3082209c486 -C extra-filename=-a14a7ffa6ced013e --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-9d8884674d468014.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-523cb2c460b2954a.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-83daf3b37151d30c.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-9f2af6068106d2fb.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-40db761eabe70986.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-808eaebb057ce37e.rmeta --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-5457d7c65e6ff7c6.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-6571000d5ff98899.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-48a1b478844485eb.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_yaml=/home/jelmer/src/janitor/target/debug/deps/libserde_yaml-d19b32863dd48a61.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-34844f972d674a37.rmeta --extern tera=/home/jelmer/src/janitor/target/debug/deps/libtera-39fe85b2b15ed66c.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --extern xdg=/home/jelmer/src/janitor/target/debug/deps/libxdg-23f110d46d019c5b.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor --edition=2021 src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="gcp"' --cfg 'feature="gcs"' --cfg 'feature="stackdriver_logger"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("debian", "default", "gcp", "gcs", "stackdriver_logger"))' -C metadata=3475718e159beeec -C extra-filename=-c8437112b2f8fc7a --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e239d113cd99a25a.rmeta --extern async_compression=/home/jelmer/src/janitor/target/debug/deps/libasync_compression-4d0766f656f1104b.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-9d8884674d468014.rmeta --extern buildlog_consultant=/home/jelmer/src/janitor/target/debug/deps/libbuildlog_consultant-87cd661227323dc7.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-9f2af6068106d2fb.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-40db761eabe70986.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-808eaebb057ce37e.rmeta --extern fancy_regex=/home/jelmer/src/janitor/target/debug/deps/libfancy_regex-0688edb11485e39a.rmeta --extern filetime=/home/jelmer/src/janitor/target/debug/deps/libfiletime-74a11c28d8d6d9ef.rmeta --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern futures=/home/jelmer/src/janitor/target/debug/deps/libfutures-463765cbcde647a0.rmeta --extern google_cloud_auth=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_auth-15bdd22f92eb38ea.rmeta --extern google_cloud_storage=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_storage-67189881ad923dbd.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-a2f283677911744d.rmeta --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-b1e3dc89f529da22.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-48a1b478844485eb.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-a14a7ffa6ced013e.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-02615dedbab651d7.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-b1311c083eaa4498.rmeta --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-928485792788ad80.rmeta --extern stackdriver_logger=/home/jelmer/src/janitor/target/debug/deps/libstackdriver_logger-ca9fc4b835919b4a.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-34844f972d674a37.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
warning: function `reprocess_run_logs` is never used
 --> src/reprocess_logs.rs:8:10
  |
8 | async fn reprocess_run_logs(
  |          ^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(dead_code)]` on by default

warning: field `branch_url` is never read
  --> src/schedule.rs:32:5
   |
30 | pub struct ScheduleRequest {
   |            --------------- field in this struct
31 |     codebase: String,
32 |     branch_url: String,
   |     ^^^^^^^^^^

warning: function `has_cotenants` is never used
  --> src/state.rs:80:10
   |
80 | async fn has_cotenants(
   |          ^^^^^^^^^^^^^

warning: field `name` is never read
  --> src/state.rs:87:13
   |
86 |     struct Codebase {
   |            -------- field in this struct
87 |         pub name: String,
   |             ^^^^
   |
   = note: `Codebase` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: function `iter_publishable_suites` is never used
   --> src/state.rs:113:10
    |
113 | async fn iter_publishable_suites(
    |          ^^^^^^^^^^^^^^^^^^^^^^^

warning: `janitor` (lib) generated 5 warnings
       Dirty common-py v0.0.0 (/home/jelmer/src/janitor/common-py): name of dependency changed (tokio => pyo3)
   Compiling common-py v0.0.0 (/home/jelmer/src/janitor/common-py)
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name common_py --edition=2021 common-py/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type cdylib --emit=dep-info,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="extension-module"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("extension-module"))' -C metadata=da6df15d416e2647 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-9d8884674d468014.rlib --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rlib --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-c8437112b2f8fc7a.rlib --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rlib --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rlib --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rlib --extern pyo3_async_runtimes=/home/jelmer/src/janitor/target/debug/deps/libpyo3_async_runtimes-326186b750d2f5f4.rlib --extern pyo3_filelike=/home/jelmer/src/janitor/target/debug/deps/libpyo3_filelike-bc0667b965758a04.rlib --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-f11d63d32a114e1a.rlib --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-48a1b478844485eb.rlib --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rlib --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-a14a7ffa6ced013e.rlib --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-2214166d5fa77833.rlib --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rlib --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
warning: unused imports: `PathBuf` and `Path`
 --> common-py/src/config.rs:3:17
  |
3 | use std::path::{Path, PathBuf};
  |                 ^^^^  ^^^^^^^
  |
  = note: `#[warn(unused_imports)]` on by default

warning: unused import: `PyNotImplementedError`
 --> common-py/src/vcs.rs:5:24
  |
5 | use pyo3::exceptions::{PyNotImplementedError, PyValueError};
  |                        ^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `Path`
 --> common-py/src/vcs.rs:9:17
  |
9 | use std::path::{Path, PathBuf};
  |                 ^^^^

warning: unused variable: `py`
   --> common-py/src/config.rs:484:20
    |
484 | pub(crate) fn init(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    |                    ^^ help: if this is intentional, prefix it with an underscore: `_py`
    |
    = note: `#[warn(unused_variables)]` on by default

warning: unused variable: `possible_transports`
   --> common-py/src/vcs.rs:325:5
    |
325 |     possible_transports: Option<Vec<PyObject>>,
    |     ^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_possible_transports`

warning: unused variable: `probers`
   --> common-py/src/vcs.rs:326:5
    |
326 |     probers: Option<Vec<PyObject>>,
    |     ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_probers`

warning: `common-py` (lib) generated 6 warnings (run `cargo fix --lib -p common-py` to apply 3 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 52.63s
Copying rust artifact from target/debug/libcommon_py.so to py/janitor/_common.cpython-313-x86_64-linux-gnu.so
cargo rustc --lib --message-format=json-render-diagnostics --manifest-path differ-py/Cargo.toml -v --features extension-module pyo3/extension-module --crate-type cdylib --
       Fresh unicode-ident v1.0.18
       Fresh cfg-if v1.0.0
       Fresh autocfg v1.4.0
       Fresh memchr v2.7.4
       Fresh once_cell v1.21.0
       Fresh value-bag v1.10.0
       Fresh pin-project-lite v0.2.16
       Fresh futures-core v0.3.31
       Fresh bitflags v2.9.0
       Fresh itoa v1.0.15
       Fresh futures-io v0.3.31
       Fresh bytes v1.10.1
       Fresh scopeguard v1.2.0
       Fresh stable_deref_trait v1.2.0
       Fresh regex-syntax v0.8.5
       Fresh shlex v1.3.0
       Fresh foldhash v0.1.4
       Fresh equivalent v1.0.2
       Fresh allocator-api2 v0.2.21
       Fresh writeable v0.5.5
       Fresh litemap v0.7.5
       Fresh pin-utils v0.1.0
       Fresh fastrand v2.3.0
       Fresh ryu v1.0.20
       Fresh icu_locid_transform_data v1.5.0
       Fresh icu_properties_data v1.5.0
       Fresh proc-macro2 v1.0.94
       Fresh tracing-core v0.1.33
       Fresh cc v1.2.16
       Fresh hashbrown v0.15.2
       Fresh percent-encoding v2.3.1
       Fresh futures-task v0.3.31
       Fresh utf8_iter v1.0.4
       Fresh vcpkg v0.2.15
       Fresh utf16_iter v1.0.5
       Fresh write16 v1.0.0
       Fresh icu_normalizer_data v1.5.0
       Fresh pkg-config v0.3.32
       Fresh atomic-waker v1.1.2
       Fresh parking v2.2.1
       Fresh linux-raw-sys v0.4.15
       Fresh version_check v0.9.5
       Fresh log v0.4.27
       Fresh quote v1.0.39
       Fresh libc v0.2.170
       Fresh crossbeam-utils v0.8.21
       Fresh indexmap v2.8.0
       Fresh foreign-types-shared v0.1.1
       Fresh iana-time-zone v0.1.61
       Fresh openssl-probe v0.1.6
       Fresh subtle v2.6.1
       Fresh futures-lite v2.6.0
       Fresh aho-corasick v1.1.3
       Fresh bitflags v1.3.2
       Fresh fnv v1.0.7
       Fresh home v0.5.11
       Fresh heck v0.5.0
       Fresh async-task v4.7.1
       Fresh syn v2.0.100
       Fresh slab v0.4.9
       Fresh lock_api v0.4.12
       Fresh rustix v0.38.44
       Fresh concurrent-queue v2.5.0
       Fresh zerocopy v0.8.23
       Fresh foreign-types v0.3.2
       Fresh typenum v1.18.0
       Fresh socket2 v0.5.8
       Fresh signal-hook-registry v1.4.2
       Fresh mio v1.0.3
       Fresh regex-automata v0.4.9
       Fresh event-listener v2.5.3
       Fresh piper v0.2.4
       Fresh cpufeatures v0.2.17
       Fresh linux-raw-sys v0.9.2
       Fresh http v1.2.0
       Fresh getrandom v0.2.15
       Fresh linux-raw-sys v0.3.8
       Fresh serde_derive v1.0.219
       Fresh synstructure v0.13.1
       Fresh zerovec-derive v0.10.3
       Fresh tracing-attributes v0.1.28
       Fresh displaydoc v0.2.5
       Fresh icu_provider_macros v1.5.0
       Fresh futures-macro v0.3.31
       Fresh openssl-macros v0.1.1
       Fresh ppv-lite86 v0.2.21
       Fresh thiserror-impl v2.0.12
       Fresh target-lexicon v0.12.16
       Fresh generic-array v0.14.7
       Fresh event-listener v5.4.0
       Fresh tokio-macros v2.5.0
   Compiling openssl-sys v0.9.107
       Fresh regex v1.11.1
       Fresh async-executor v1.13.1
       Fresh waker-fn v1.2.0
       Fresh fastrand v1.9.0
       Fresh async-channel v1.9.0
       Fresh async-lock v2.8.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name openssl_sys --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/openssl-sys-0.9.107/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("aws-lc", "bindgen", "bssl-sys", "openssl-src", "unstable_boringssl", "vendored"))' -C metadata=5237f66b0acb2b4c -C extra-filename=-6fff705ae2f3aa73 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-f96927166a8959d2.rmeta --cap-lints allow --cfg tokio_unstable -l ssl -l crypto --cfg 'osslconf="OPENSSL_NO_IDEA"' --cfg 'osslconf="OPENSSL_NO_SSL3_METHOD"' --cfg openssl --cfg ossl340 --cfg ossl330 --cfg ossl320 --cfg ossl300 --cfg ossl101 --cfg ossl102 --cfg ossl102f --cfg ossl102h --cfg ossl110 --cfg ossl110f --cfg ossl110g --cfg ossl110h --cfg ossl111 --cfg ossl111b --cfg ossl111c --cfg ossl111d --check-cfg 'cfg(osslconf, values("OPENSSL_NO_OCB", "OPENSSL_NO_SM4", "OPENSSL_NO_SEED", "OPENSSL_NO_CHACHA", "OPENSSL_NO_CAST", "OPENSSL_NO_IDEA", "OPENSSL_NO_CAMELLIA", "OPENSSL_NO_RC4", "OPENSSL_NO_BF", "OPENSSL_NO_PSK", "OPENSSL_NO_DEPRECATED_3_0", "OPENSSL_NO_SCRYPT", "OPENSSL_NO_SM3", "OPENSSL_NO_RMD160", "OPENSSL_NO_EC2M", "OPENSSL_NO_OCSP", "OPENSSL_NO_CMS", "OPENSSL_NO_COMP", "OPENSSL_NO_SOCK", "OPENSSL_NO_STDIO", "OPENSSL_NO_EC", "OPENSSL_NO_SSL3_METHOD", "OPENSSL_NO_KRB5", "OPENSSL_NO_TLSEXT", "OPENSSL_NO_SRP", "OPENSSL_NO_RFC3779", "OPENSSL_NO_SHA", "OPENSSL_NO_NEXTPROTONEG", "OPENSSL_NO_ENGINE", "OPENSSL_NO_BUF_FREELISTS", "OPENSSL_NO_RC2"))' --check-cfg 'cfg(openssl)' --check-cfg 'cfg(libressl)' --check-cfg 'cfg(boringssl)' --check-cfg 'cfg(awslc)' --check-cfg 'cfg(libressl250)' --check-cfg 'cfg(libressl251)' --check-cfg 'cfg(libressl252)' --check-cfg 'cfg(libressl261)' --check-cfg 'cfg(libressl270)' --check-cfg 'cfg(libressl271)' --check-cfg 'cfg(libressl273)' --check-cfg 'cfg(libressl280)' --check-cfg 'cfg(libressl281)' --check-cfg 'cfg(libressl291)' --check-cfg 'cfg(libressl310)' --check-cfg 'cfg(libressl321)' --check-cfg 'cfg(libressl332)' --check-cfg 'cfg(libressl340)' --check-cfg 'cfg(libressl350)' --check-cfg 'cfg(libressl360)' --check-cfg 'cfg(libressl361)' --check-cfg 'cfg(libressl370)' --check-cfg 'cfg(libressl380)' --check-cfg 'cfg(libressl381)' --check-cfg 'cfg(libressl382)' --check-cfg 'cfg(libressl390)' --check-cfg 'cfg(libressl400)' --check-cfg 'cfg(libressl410)' --check-cfg 'cfg(ossl101)' --check-cfg 'cfg(ossl102)' --check-cfg 'cfg(ossl102f)' --check-cfg 'cfg(ossl102h)' --check-cfg 'cfg(ossl110)' --check-cfg 'cfg(ossl110f)' --check-cfg 'cfg(ossl110g)' --check-cfg 'cfg(ossl110h)' --check-cfg 'cfg(ossl111)' --check-cfg 'cfg(ossl111b)' --check-cfg 'cfg(ossl111c)' --check-cfg 'cfg(ossl111d)' --check-cfg 'cfg(ossl300)' --check-cfg 'cfg(ossl310)' --check-cfg 'cfg(ossl320)' --check-cfg 'cfg(ossl330)' --check-cfg 'cfg(ossl340)'`
       Fresh serde v1.0.219
       Fresh zerofrom-derive v0.1.6
       Fresh yoke-derive v0.7.5
       Fresh thiserror v2.0.12
       Fresh event-listener-strategy v0.5.3
       Fresh crypto-common v0.1.6
       Fresh block-buffer v0.10.4
       Fresh rustix v1.0.2
       Fresh tracing v0.1.41
       Fresh futures-lite v1.13.0
       Fresh powerfmt v0.2.0
       Fresh mime v0.3.17
       Fresh num-conv v0.1.0
       Fresh time-core v0.1.3
       Fresh crc-catalog v2.4.0
       Fresh tinyvec_macros v0.1.1
       Fresh unicase v2.8.1
       Fresh http-body v1.0.1
       Fresh crossbeam-queue v0.3.12
       Fresh hashlink v0.10.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name openssl_sys --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/openssl-sys-0.9.107/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("aws-lc", "bindgen", "bssl-sys", "openssl-src", "unstable_boringssl", "vendored"))' -C metadata=7b0bed979f78baed -C extra-filename=-44f593d45a51be8b --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-b68f60cf32f6788d.rmeta --cap-lints allow --cfg tokio_unstable -l ssl -l crypto --cfg 'osslconf="OPENSSL_NO_IDEA"' --cfg 'osslconf="OPENSSL_NO_SSL3_METHOD"' --cfg openssl --cfg ossl340 --cfg ossl330 --cfg ossl320 --cfg ossl300 --cfg ossl101 --cfg ossl102 --cfg ossl102f --cfg ossl102h --cfg ossl110 --cfg ossl110f --cfg ossl110g --cfg ossl110h --cfg ossl111 --cfg ossl111b --cfg ossl111c --cfg ossl111d --check-cfg 'cfg(osslconf, values("OPENSSL_NO_OCB", "OPENSSL_NO_SM4", "OPENSSL_NO_SEED", "OPENSSL_NO_CHACHA", "OPENSSL_NO_CAST", "OPENSSL_NO_IDEA", "OPENSSL_NO_CAMELLIA", "OPENSSL_NO_RC4", "OPENSSL_NO_BF", "OPENSSL_NO_PSK", "OPENSSL_NO_DEPRECATED_3_0", "OPENSSL_NO_SCRYPT", "OPENSSL_NO_SM3", "OPENSSL_NO_RMD160", "OPENSSL_NO_EC2M", "OPENSSL_NO_OCSP", "OPENSSL_NO_CMS", "OPENSSL_NO_COMP", "OPENSSL_NO_SOCK", "OPENSSL_NO_STDIO", "OPENSSL_NO_EC", "OPENSSL_NO_SSL3_METHOD", "OPENSSL_NO_KRB5", "OPENSSL_NO_TLSEXT", "OPENSSL_NO_SRP", "OPENSSL_NO_RFC3779", "OPENSSL_NO_SHA", "OPENSSL_NO_NEXTPROTONEG", "OPENSSL_NO_ENGINE", "OPENSSL_NO_BUF_FREELISTS", "OPENSSL_NO_RC2"))' --check-cfg 'cfg(openssl)' --check-cfg 'cfg(libressl)' --check-cfg 'cfg(boringssl)' --check-cfg 'cfg(awslc)' --check-cfg 'cfg(libressl250)' --check-cfg 'cfg(libressl251)' --check-cfg 'cfg(libressl252)' --check-cfg 'cfg(libressl261)' --check-cfg 'cfg(libressl270)' --check-cfg 'cfg(libressl271)' --check-cfg 'cfg(libressl273)' --check-cfg 'cfg(libressl280)' --check-cfg 'cfg(libressl281)' --check-cfg 'cfg(libressl291)' --check-cfg 'cfg(libressl310)' --check-cfg 'cfg(libressl321)' --check-cfg 'cfg(libressl332)' --check-cfg 'cfg(libressl340)' --check-cfg 'cfg(libressl350)' --check-cfg 'cfg(libressl360)' --check-cfg 'cfg(libressl361)' --check-cfg 'cfg(libressl370)' --check-cfg 'cfg(libressl380)' --check-cfg 'cfg(libressl381)' --check-cfg 'cfg(libressl382)' --check-cfg 'cfg(libressl390)' --check-cfg 'cfg(libressl400)' --check-cfg 'cfg(libressl410)' --check-cfg 'cfg(ossl101)' --check-cfg 'cfg(ossl102)' --check-cfg 'cfg(ossl102f)' --check-cfg 'cfg(ossl102h)' --check-cfg 'cfg(ossl110)' --check-cfg 'cfg(ossl110f)' --check-cfg 'cfg(ossl110g)' --check-cfg 'cfg(ossl110h)' --check-cfg 'cfg(ossl111)' --check-cfg 'cfg(ossl111b)' --check-cfg 'cfg(ossl111c)' --check-cfg 'cfg(ossl111d)' --check-cfg 'cfg(ossl300)' --check-cfg 'cfg(ossl310)' --check-cfg 'cfg(ossl320)' --check-cfg 'cfg(ossl330)' --check-cfg 'cfg(ossl340)'`
       Fresh zerofrom v0.1.6
       Fresh serde_json v1.0.140
       Fresh digest v0.10.7
       Fresh async-lock v3.4.0
       Fresh smallvec v1.14.0
       Fresh async-channel v2.3.1
       Fresh either v1.15.0
       Fresh deranged v0.3.11
       Fresh time-macros v0.2.20
       Fresh tinyvec v1.9.0
       Fresh crc v3.2.1
       Fresh num-traits v0.2.19
       Fresh futures-sink v0.3.31
       Fresh polling v3.7.4
       Fresh thiserror-impl v1.0.69
       Fresh rand_core v0.6.4
       Fresh yoke v0.7.5
       Fresh pyo3-build-config v0.22.6
       Fresh blocking v1.6.1
       Fresh sha2 v0.10.8
       Fresh time v0.3.39
       Fresh parking_lot_core v0.9.10
       Fresh unicode-normalization v0.1.24
       Fresh hmac v0.12.1
       Fresh memoffset v0.9.1
       Fresh form_urlencoded v1.2.1
       Fresh hex v0.4.3
       Fresh unicode-properties v0.1.3
       Fresh try-lock v0.2.5
       Fresh tower-service v0.3.3
       Fresh unindent v0.2.4
       Fresh httpdate v1.0.3
       Fresh unicode-bidi v0.3.18
       Fresh indoc v2.0.6
       Fresh futures-util v0.3.31
       Fresh rand_chacha v0.3.1
       Fresh httparse v1.10.1
       Fresh md-5 v0.10.6
       Fresh thiserror v1.0.69
       Fresh async-io v2.4.0
       Fresh chrono v0.4.40
       Fresh io-lifetimes v1.0.11
       Fresh dotenvy v0.15.7
       Fresh zerovec v0.10.4
       Fresh parking_lot v0.12.3
       Fresh want v0.3.1
       Fresh hkdf v0.12.4
       Fresh stringprep v0.1.5
       Fresh whoami v1.5.2
       Fresh byteorder v1.5.0
       Fresh rand v0.8.5
       Fresh rustix v0.37.28
       Fresh async-global-executor v2.4.1
       Fresh futures-channel v0.3.31
       Fresh http-body-util v0.1.3
       Fresh polling v2.8.0
       Fresh socket2 v0.4.10
       Fresh kv-log-macro v1.0.7
       Fresh sync_wrapper v1.0.2
       Fresh siphasher v1.0.1
       Fresh rustc-hash v1.1.0
       Fresh hashbrown v0.14.5
       Fresh tower-layer v0.3.3
       Fresh countme v3.0.1
       Fresh text-size v1.1.1
       Fresh base64 v0.22.1
       Fresh tinystr v0.7.6
       Fresh icu_collections v1.5.0
   Compiling tokio v1.44.2
       Fresh async-io v1.13.0
       Fresh async-std v1.13.1
       Fresh futures-intrusive v0.5.0
       Fresh serde_urlencoded v0.7.1
       Fresh lazy-regex-proc_macros v3.4.1
       Fresh atoi v2.0.0
       Fresh rowan v0.16.1
       Fresh same-file v1.0.6
       Fresh rustls-pki-types v1.11.0
       Fresh utf8parse v0.2.2
       Fresh deb822-derive v0.2.0
       Fresh encoding_rs v0.8.35
       Fresh unicode-width v0.2.0
       Fresh anstyle v1.0.10
       Fresh is_terminal_polyfill v1.70.1
       Fresh ipnet v2.11.0
       Fresh adler2 v2.0.0
       Fresh anstyle-query v1.1.2
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.44.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(loom)' --check-cfg 'cfg(mio_unsupported_force_poll_poll)' --check-cfg 'cfg(tokio_allow_from_blocking_fd)' --check-cfg 'cfg(tokio_internal_mt_counters)' --check-cfg 'cfg(tokio_no_parking_lot)' --check-cfg 'cfg(tokio_no_tuning_tests)' --check-cfg 'cfg(tokio_taskdump)' --check-cfg 'cfg(tokio_unstable)' --cfg 'feature="bytes"' --cfg 'feature="default"' --cfg 'feature="fs"' --cfg 'feature="full"' --cfg 'feature="io-std"' --cfg 'feature="io-util"' --cfg 'feature="libc"' --cfg 'feature="macros"' --cfg 'feature="mio"' --cfg 'feature="net"' --cfg 'feature="parking_lot"' --cfg 'feature="process"' --cfg 'feature="rt"' --cfg 'feature="rt-multi-thread"' --cfg 'feature="signal"' --cfg 'feature="signal-hook-registry"' --cfg 'feature="socket2"' --cfg 'feature="sync"' --cfg 'feature="time"' --cfg 'feature="tokio-macros"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("bytes", "default", "fs", "full", "io-std", "io-util", "libc", "macros", "mio", "net", "parking_lot", "process", "rt", "rt-multi-thread", "signal", "signal-hook-registry", "socket2", "sync", "test-util", "time", "tokio-macros", "tracing", "windows-sys"))' -C metadata=203d64024ebbf181 -C extra-filename=-15192f2ea77506d4 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-f96927166a8959d2.rmeta --extern mio=/home/jelmer/src/janitor/target/debug/deps/libmio-4243848b43cf6eaa.rmeta --extern parking_lot=/home/jelmer/src/janitor/target/debug/deps/libparking_lot-83fcc7478224180d.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern signal_hook_registry=/home/jelmer/src/janitor/target/debug/deps/libsignal_hook_registry-07869e6e8c107085.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-9be7658adf7d58e8.rmeta --extern tokio_macros=/home/jelmer/src/janitor/target/debug/deps/libtokio_macros-6d6e842210b98dca.so --cap-lints allow --cfg tokio_unstable`
       Fresh icu_locid v1.5.0
       Fresh pyo3-macros-backend v0.22.6
       Fresh pyo3-ffi v0.22.6
       Fresh syn v1.0.109
       Fresh lazy-regex v3.4.1
       Fresh rustls-pemfile v2.2.0
       Fresh walkdir v2.5.0
       Fresh anstyle-parse v0.2.6
       Fresh colorchoice v1.0.3
       Fresh phf_generator v0.11.3
       Fresh miniz_oxide v0.8.5
       Fresh num-integer v0.1.46
       Fresh lazy_static v1.5.0
       Fresh protobuf-support v3.7.2
       Fresh which v4.4.2
       Fresh parse-zoneinfo v0.3.1
       Fresh async-trait v0.1.88
       Fresh inotify-sys v0.1.5
       Fresh smawk v0.3.2
       Fresh ucd-trie v0.1.7
       Fresh gimli v0.31.1
       Fresh icu_provider v1.5.0
       Fresh pyo3-macros v0.22.6
       Fresh getrandom v0.3.1
       Fresh anstream v0.6.18
       Fresh num-bigint v0.4.6
       Fresh anyhow v1.0.97
       Fresh phf_codegen v0.11.3
       Fresh unicode-linebreak v0.1.5
       Fresh unicode-xid v0.2.6
       Fresh clap_lex v0.7.4
       Fresh strsim v0.11.1
       Fresh inotify v0.9.6
       Fresh addr2line v0.24.2
       Fresh pest v2.7.15
       Fresh clap_derive v4.5.32
       Fresh filetime v0.2.25
       Fresh crossbeam-channel v0.5.15
       Fresh mio v0.8.11
       Fresh crossbeam-epoch v0.9.18
       Fresh bstr v1.11.3
       Fresh rustc-demangle v0.1.24
       Fresh unic-common v0.9.0
       Fresh icu_locid_transform v1.5.0
       Fresh pyo3 v0.22.6
       Fresh object v0.36.7
       Fresh synstructure v0.12.6
       Fresh textwrap v0.16.2
       Fresh protobuf v3.7.2
       Fresh clap_builder v4.5.36
       Fresh chrono-tz-build v0.3.0
       Fresh base64ct v1.7.1
       Fresh dtor-proc-macro v0.0.5
       Fresh unic-char-range v0.9.0
       Fresh protobuf v2.28.0
       Fresh unic-ucd-version v0.9.0
       Fresh tempfile v3.19.0
       Fresh globset v0.4.16
       Fresh crossbeam-deque v0.8.6
       Fresh pest_meta v2.7.15
   Compiling notify v6.1.1
       Fresh phf_shared v0.11.3
       Fresh rowan v0.15.16
       Fresh sha1 v0.10.6
       Fresh itertools v0.13.0
       Fresh futures-executor v0.3.31
       Fresh atty v0.2.14
       Fresh csv-core v0.1.12
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name notify --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/notify-6.1.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="crossbeam-channel"' --cfg 'feature="default"' --cfg 'feature="fsevent-sys"' --cfg 'feature="macos_fsevent"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("crossbeam-channel", "default", "fsevent-sys", "kqueue", "macos_fsevent", "macos_kqueue", "manual_tests", "mio", "serde", "timing_tests"))' -C metadata=85a47f81b0f26f41 -C extra-filename=-2f99c198d00e1e3f --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern crossbeam_channel=/home/jelmer/src/janitor/target/debug/deps/libcrossbeam_channel-4b40e4830c191e81.rmeta --extern filetime=/home/jelmer/src/janitor/target/debug/deps/libfiletime-8f90a18bbe2253cd.rmeta --extern inotify=/home/jelmer/src/janitor/target/debug/deps/libinotify-20d7d31bfb84f38a.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-f96927166a8959d2.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern mio=/home/jelmer/src/janitor/target/debug/deps/libmio-16e2e6ea8bcad402.rmeta --extern walkdir=/home/jelmer/src/janitor/target/debug/deps/libwalkdir-f95d3688eab8bd63.rmeta --cap-lints allow --cfg tokio_unstable`
       Fresh icu_properties v1.5.1
       Fresh deb822-lossless v0.2.4
       Fresh unic-char-property v0.9.0
       Fresh libm v0.2.11
       Fresh dtor v0.0.5
       Fresh clap v4.5.36
       Fresh failure_derive v0.1.8
       Fresh backtrace v0.3.74
       Fresh pem-rfc7468 v0.7.0
       Fresh const-oid v0.9.6
       Fresh termcolor v1.4.1
       Fresh difflib v0.4.0
       Fresh urlencoding v2.1.3
       Fresh zeroize v1.8.1
       Fresh minimal-lexical v0.2.1
       Fresh untrusted v0.9.0
       Fresh base64 v0.21.7
       Fresh ctor-proc-macro v0.0.5
       Fresh quick-error v1.2.3
       Fresh pyo3-filelike v0.4.1
       Fresh ignore v0.4.23
       Fresh futures v0.3.31
       Fresh csv v1.3.1
       Fresh phf v0.11.3
       Fresh icu_normalizer v1.5.0
       Fresh der v0.7.9
       Fresh ring v0.17.13
       Fresh ctor v0.4.1
       Fresh humansize v2.1.3
       Fresh failure v0.1.8
       Fresh unic-ucd-segment v0.9.0
       Fresh humantime v1.3.0
       Fresh nom v7.1.3
       Fresh pest_generator v2.7.15
       Fresh protobuf-parse v3.7.2
       Fresh protobuf-codegen v2.28.0
       Fresh simple_asn1 v0.6.3
       Fresh rand_core v0.9.3
       Fresh patchkit v0.2.1
       Fresh protoc v2.28.0
       Fresh version-ranges v0.1.1
       Fresh pem v3.0.5
       Fresh env_filter v0.1.3
       Fresh crc32fast v1.4.2
       Fresh maplit v1.0.2
       Fresh toml_datetime v0.6.8
       Fresh unsafe-libyaml v0.2.11
       Fresh idna_adapter v1.2.0
       Fresh deunicode v1.6.0
       Fresh bit-vec v0.8.0
       Fresh jiff v0.2.4
       Fresh winnow v0.7.3
       Fresh unscanny v0.1.0
       Fresh spki v0.7.3
       Fresh flate2 v1.1.0
       Fresh protobuf-codegen v3.7.2
       Fresh chrono-tz v0.9.0
       Fresh rustversion v1.0.20
       Fresh distro-info v0.4.0
       Fresh jsonwebtoken v9.3.1
       Fresh askama_parser v0.2.1
       Fresh serde_yaml v0.9.34+deprecated
       Fresh rand_chacha v0.9.0
       Fresh semver v1.0.26
   Compiling protoc-rust v2.28.0
       Fresh env_logger v0.7.1
       Fresh unic-segment v0.9.0
       Fresh pest_derive v2.7.15
       Fresh mime_guess v2.0.5
       Fresh globwalk v0.9.1
       Fresh merge3 v0.2.0
       Fresh google-cloud-token v0.1.2
       Fresh makefile-lossless v0.1.7
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name protoc_rust --edition=2015 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/protoc-rust-2.28.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=c9fe425ad13f3e8c -C extra-filename=-7b27bf6847614d6f --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-98c17e40d8ea6aed.rmeta --extern protobuf_codegen=/home/jelmer/src/janitor/target/debug/deps/libprotobuf_codegen-13f80eb668e85f15.rmeta --extern protoc=/home/jelmer/src/janitor/target/debug/deps/libprotoc-4b211fff6c1cb7ad.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-5cc214a3774c4b08.rmeta --cap-lints allow --cfg tokio_unstable`
       Fresh idna v1.0.3
       Fresh env_logger v0.11.7
       Fresh pep440_rs v0.7.3
       Fresh toml_edit v0.22.24
       Fresh bit-set v0.8.0
       Fresh slug v0.1.6
       Fresh basic-toml v0.1.10
       Fresh async-stream-impl v0.3.6
       Fresh arc-swap v1.7.1
       Fresh humantime v2.1.0
       Fresh boxcar v0.2.10
       Fresh configparser v3.1.0
       Fresh rustc-hash v2.1.1
   Compiling axum-core v0.5.2
       Fresh pkcs8 v0.10.2
       Fresh rand v0.9.0
       Fresh pretty_env_logger v0.4.0
       Fresh toml v0.5.11
       Fresh serde_path_to_error v0.1.17
       Fresh backon v1.4.0
       Fresh heck v0.4.1
       Fresh inventory v0.3.20
       Fresh askama_escape v0.10.3
       Fresh xdg v2.5.2
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name axum_core --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/axum-core-0.5.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::verbose_file_reads' '--warn=clippy::unused_self' --forbid=unsafe_code --warn=unreachable_pub '--warn=clippy::unnested_or_patterns' '--warn=clippy::uninlined_format_args' '--allow=clippy::type_complexity' '--warn=clippy::todo' '--warn=clippy::suboptimal_flops' '--warn=clippy::str_to_string' '--warn=clippy::rest_pat_in_fully_bound_structs' '--warn=clippy::option_option' '--warn=clippy::needless_continue' '--warn=clippy::needless_borrow' --warn=missing_docs --warn=missing_debug_implementations '--warn=clippy::mem_forget' '--warn=clippy::match_wildcard_for_single_variants' '--warn=clippy::match_on_vec_items' '--warn=clippy::macro_use_imports' '--warn=clippy::lossy_float_literal' '--warn=clippy::linkedlist' '--warn=clippy::inefficient_to_string' '--warn=clippy::imprecise_flops' '--warn=clippy::if_let_mutex' '--warn=clippy::fn_params_excessive_bools' '--warn=clippy::filter_map_next' '--warn=clippy::exit' '--warn=clippy::enum_glob_use' '--warn=clippy::empty_enum' '--warn=clippy::dbg_macro' '--warn=clippy::await_holding_lock' --cfg 'feature="tracing"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__private_docs", "tracing"))' -C metadata=9b90dbbbc107ba29 -C extra-filename=-c7084f1580c648d3 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern mime=/home/jelmer/src/janitor/target/debug/deps/libmime-c74000db2aaf511f.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern rustversion=/home/jelmer/src/janitor/target/debug/deps/librustversion-494b2fd16358ba50.so --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tower_layer=/home/jelmer/src/janitor/target/debug/deps/libtower_layer-fff117c1c28e545a.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
       Fresh url v2.5.4
       Fresh fancy-regex v0.14.0
       Fresh askama_derive v0.12.5
       Fresh async-stream v0.3.6
       Fresh env_logger v0.9.3
       Fresh tera v1.20.0
       Fresh sha1_smol v1.0.1
       Fresh matchit v0.8.4
       Fresh cfg_aliases v0.2.1
       Fresh snafu-derive v0.7.5
       Fresh headers-core v0.3.0
       Fresh itertools v0.10.5
       Fresh http v0.2.12
       Fresh self_cell v1.1.0
       Fresh memo-map v0.3.3
       Fresh pyo3-log v0.11.0
       Fresh dep3 v0.1.28
       Fresh pep508_rs v0.9.2
       Fresh doc-comment v0.3.3
       Fresh askama v0.12.1
       Fresh stackdriver_logger v0.8.2
       Fresh headers v0.4.0
   Compiling minijinja v2.9.0
       Fresh snafu v0.7.5
       Fresh nix v0.29.0
       Fresh accept-header v0.2.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name minijinja --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/minijinja-2.9.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="adjacent_loop_items"' --cfg 'feature="builtins"' --cfg 'feature="debug"' --cfg 'feature="default"' --cfg 'feature="deserialization"' --cfg 'feature="loader"' --cfg 'feature="macros"' --cfg 'feature="memo-map"' --cfg 'feature="multi_template"' --cfg 'feature="self_cell"' --cfg 'feature="serde"' --cfg 'feature="std_collections"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("adjacent_loop_items", "builtins", "custom_syntax", "debug", "default", "deserialization", "fuel", "indexmap", "internal_debug", "internal_safe_search", "json", "key_interning", "loader", "loop_controls", "macros", "memo-map", "multi_template", "percent-encoding", "preserve_order", "self_cell", "serde", "serde_json", "speedups", "stacker", "std_collections", "unicase", "unicode", "unicode-ident", "unstable_machinery", "unstable_machinery_serde", "urlencode", "v_htmlescape"))' -C metadata=931864073b7d93e2 -C extra-filename=-6c444bb921e91f39 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern memo_map=/home/jelmer/src/janitor/target/debug/deps/libmemo_map-92db1326c6e2ab83.rmeta --extern self_cell=/home/jelmer/src/janitor/target/debug/deps/libself_cell-d39202239a457cd2.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling janitor v0.1.0 (/home/jelmer/src/janitor)
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name build_script_build --edition=2021 build.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="gcp"' --cfg 'feature="gcs"' --cfg 'feature="stackdriver_logger"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("debian", "default", "gcp", "gcs", "stackdriver_logger"))' -C metadata=f3dd540023186363 -C extra-filename=-8e254360748d6f65 --out-dir /home/jelmer/src/janitor/target/debug/build/janitor-8e254360748d6f65 -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern protobuf_codegen=/home/jelmer/src/janitor/target/debug/deps/libprotobuf_codegen-a297d111f2438ba9.rlib --extern protoc_rust=/home/jelmer/src/janitor/target/debug/deps/libprotoc_rust-7b27bf6847614d6f.rlib --cfg tokio_unstable`
   Compiling dirty-tracker v0.3.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name dirty_tracker --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/dirty-tracker-0.3.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=238e9961b086392f -C extra-filename=-6abba3579c29934f --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern notify=/home/jelmer/src/janitor/target/debug/deps/libnotify-2f99c198d00e1e3f.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling openssl v0.10.72
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name openssl --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/openssl-0.10.72/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("aws-lc", "bindgen", "default", "unstable_boringssl", "v101", "v102", "v110", "v111", "vendored"))' -C metadata=282c30ddce69f64f -C extra-filename=-e39279ccf92c2d8c --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern cfg_if=/home/jelmer/src/janitor/target/debug/deps/libcfg_if-c30e1cd0415bac84.rmeta --extern foreign_types=/home/jelmer/src/janitor/target/debug/deps/libforeign_types-0bf9645f98990128.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-f96927166a8959d2.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern openssl_macros=/home/jelmer/src/janitor/target/debug/deps/libopenssl_macros-89150665c9ae34c2.so --extern ffi=/home/jelmer/src/janitor/target/debug/deps/libopenssl_sys-6fff705ae2f3aa73.rmeta --cap-lints allow --cfg tokio_unstable --cfg 'osslconf="OPENSSL_NO_IDEA"' --cfg 'osslconf="OPENSSL_NO_SSL3_METHOD"' --cfg ossl101 --cfg ossl102 --cfg ossl110 --cfg ossl110g --cfg ossl110h --cfg ossl111 --cfg ossl111d --cfg ossl300 --cfg ossl310 --cfg ossl320 --cfg ossl330 --check-cfg 'cfg(osslconf, values("OPENSSL_NO_OCB", "OPENSSL_NO_SM4", "OPENSSL_NO_SEED", "OPENSSL_NO_CHACHA", "OPENSSL_NO_CAST", "OPENSSL_NO_IDEA", "OPENSSL_NO_CAMELLIA", "OPENSSL_NO_RC4", "OPENSSL_NO_BF", "OPENSSL_NO_PSK", "OPENSSL_NO_DEPRECATED_3_0", "OPENSSL_NO_SCRYPT", "OPENSSL_NO_SM3", "OPENSSL_NO_RMD160", "OPENSSL_NO_EC2M", "OPENSSL_NO_OCSP", "OPENSSL_NO_CMS", "OPENSSL_NO_EC", "OPENSSL_NO_ARGON2", "OPENSSL_NO_RC2"))' --check-cfg 'cfg(libressl)' --check-cfg 'cfg(boringssl)' --check-cfg 'cfg(awslc)' --check-cfg 'cfg(libressl250)' --check-cfg 'cfg(libressl251)' --check-cfg 'cfg(libressl261)' --check-cfg 'cfg(libressl270)' --check-cfg 'cfg(libressl271)' --check-cfg 'cfg(libressl273)' --check-cfg 'cfg(libressl280)' --check-cfg 'cfg(libressl291)' --check-cfg 'cfg(libressl310)' --check-cfg 'cfg(libressl321)' --check-cfg 'cfg(libressl332)' --check-cfg 'cfg(libressl340)' --check-cfg 'cfg(libressl350)' --check-cfg 'cfg(libressl360)' --check-cfg 'cfg(libressl361)' --check-cfg 'cfg(libressl370)' --check-cfg 'cfg(libressl380)' --check-cfg 'cfg(libressl382)' --check-cfg 'cfg(libressl390)' --check-cfg 'cfg(libressl400)' --check-cfg 'cfg(libressl410)' --check-cfg 'cfg(ossl101)' --check-cfg 'cfg(ossl102)' --check-cfg 'cfg(ossl110)' --check-cfg 'cfg(ossl110g)' --check-cfg 'cfg(ossl110h)' --check-cfg 'cfg(ossl111)' --check-cfg 'cfg(ossl111d)' --check-cfg 'cfg(ossl300)' --check-cfg 'cfg(ossl310)' --check-cfg 'cfg(ossl320)' --check-cfg 'cfg(ossl330)'`
     Running `/home/jelmer/src/janitor/target/debug/build/janitor-8e254360748d6f65/build-script-build`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name openssl --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/openssl-0.10.72/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("aws-lc", "bindgen", "default", "unstable_boringssl", "v101", "v102", "v110", "v111", "vendored"))' -C metadata=6eb604173c8132c6 -C extra-filename=-5f1bf55ae34de4e8 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern cfg_if=/home/jelmer/src/janitor/target/debug/deps/libcfg_if-c30e1cd0415bac84.rmeta --extern foreign_types=/home/jelmer/src/janitor/target/debug/deps/libforeign_types-0bf9645f98990128.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-b68f60cf32f6788d.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern openssl_macros=/home/jelmer/src/janitor/target/debug/deps/libopenssl_macros-89150665c9ae34c2.so --extern ffi=/home/jelmer/src/janitor/target/debug/deps/libopenssl_sys-44f593d45a51be8b.rmeta --cap-lints allow --cfg tokio_unstable --cfg 'osslconf="OPENSSL_NO_IDEA"' --cfg 'osslconf="OPENSSL_NO_SSL3_METHOD"' --cfg ossl101 --cfg ossl102 --cfg ossl110 --cfg ossl110g --cfg ossl110h --cfg ossl111 --cfg ossl111d --cfg ossl300 --cfg ossl310 --cfg ossl320 --cfg ossl330 --check-cfg 'cfg(osslconf, values("OPENSSL_NO_OCB", "OPENSSL_NO_SM4", "OPENSSL_NO_SEED", "OPENSSL_NO_CHACHA", "OPENSSL_NO_CAST", "OPENSSL_NO_IDEA", "OPENSSL_NO_CAMELLIA", "OPENSSL_NO_RC4", "OPENSSL_NO_BF", "OPENSSL_NO_PSK", "OPENSSL_NO_DEPRECATED_3_0", "OPENSSL_NO_SCRYPT", "OPENSSL_NO_SM3", "OPENSSL_NO_RMD160", "OPENSSL_NO_EC2M", "OPENSSL_NO_OCSP", "OPENSSL_NO_CMS", "OPENSSL_NO_EC", "OPENSSL_NO_ARGON2", "OPENSSL_NO_RC2"))' --check-cfg 'cfg(libressl)' --check-cfg 'cfg(boringssl)' --check-cfg 'cfg(awslc)' --check-cfg 'cfg(libressl250)' --check-cfg 'cfg(libressl251)' --check-cfg 'cfg(libressl261)' --check-cfg 'cfg(libressl270)' --check-cfg 'cfg(libressl271)' --check-cfg 'cfg(libressl273)' --check-cfg 'cfg(libressl280)' --check-cfg 'cfg(libressl291)' --check-cfg 'cfg(libressl310)' --check-cfg 'cfg(libressl321)' --check-cfg 'cfg(libressl332)' --check-cfg 'cfg(libressl340)' --check-cfg 'cfg(libressl350)' --check-cfg 'cfg(libressl360)' --check-cfg 'cfg(libressl361)' --check-cfg 'cfg(libressl370)' --check-cfg 'cfg(libressl380)' --check-cfg 'cfg(libressl382)' --check-cfg 'cfg(libressl390)' --check-cfg 'cfg(libressl400)' --check-cfg 'cfg(libressl410)' --check-cfg 'cfg(ossl101)' --check-cfg 'cfg(ossl102)' --check-cfg 'cfg(ossl110)' --check-cfg 'cfg(ossl110g)' --check-cfg 'cfg(ossl110h)' --check-cfg 'cfg(ossl111)' --check-cfg 'cfg(ossl111d)' --check-cfg 'cfg(ossl300)' --check-cfg 'cfg(ossl310)' --check-cfg 'cfg(ossl320)' --check-cfg 'cfg(ossl330)'`
   Compiling native-tls v0.2.14
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name native_tls --edition=2015 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/native-tls-0.2.14/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alpn", "vendored"))' -C metadata=fd7c976d110de6a4 -C extra-filename=-d6016ca7baa5f84b --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-400fd68aa602ed65.rmeta --extern openssl=/home/jelmer/src/janitor/target/debug/deps/libopenssl-5f1bf55ae34de4e8.rmeta --extern openssl_probe=/home/jelmer/src/janitor/target/debug/deps/libopenssl_probe-81c031c110cf4218.rmeta --extern openssl_sys=/home/jelmer/src/janitor/target/debug/deps/libopenssl_sys-44f593d45a51be8b.rmeta --cap-lints allow --cfg tokio_unstable --cfg have_min_max_version --check-cfg 'cfg(have_min_max_version)'`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name native_tls --edition=2015 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/native-tls-0.2.14/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alpn", "vendored"))' -C metadata=e1072e02262482d4 -C extra-filename=-9ca42638756f24e8 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern openssl=/home/jelmer/src/janitor/target/debug/deps/libopenssl-e39279ccf92c2d8c.rmeta --extern openssl_probe=/home/jelmer/src/janitor/target/debug/deps/libopenssl_probe-81c031c110cf4218.rmeta --extern openssl_sys=/home/jelmer/src/janitor/target/debug/deps/libopenssl_sys-6fff705ae2f3aa73.rmeta --cap-lints allow --cfg tokio_unstable --cfg have_min_max_version --check-cfg 'cfg(have_min_max_version)'`
   Compiling sqlx-core v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_core --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-core-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="_rt-async-std"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="any"' --cfg 'feature="async-io"' --cfg 'feature="async-std"' --cfg 'feature="chrono"' --cfg 'feature="crc"' --cfg 'feature="default"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="native-tls"' --cfg 'feature="offline"' --cfg 'feature="serde"' --cfg 'feature="serde_json"' --cfg 'feature="sha2"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_tls-native-tls", "_tls-none", "_tls-rustls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "any", "async-io", "async-std", "bigdecimal", "bit-vec", "bstr", "chrono", "crc", "default", "ipnetwork", "json", "mac_address", "migrate", "native-tls", "offline", "regex", "rust_decimal", "rustls", "rustls-native-certs", "rustls-pemfile", "serde", "serde_json", "sha2", "time", "tokio", "tokio-stream", "uuid", "webpki-roots"))' -C metadata=2265824bb0b95f83 -C extra-filename=-ec5d4634e2bc5bab --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_io=/home/jelmer/src/janitor/target/debug/deps/libasync_io-2442fda842a01f7a.rmeta --extern async_std=/home/jelmer/src/janitor/target/debug/deps/libasync_std-220d57f9d1d250bf.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-4eb151582e08ecdb.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern crossbeam_queue=/home/jelmer/src/janitor/target/debug/deps/libcrossbeam_queue-577e4d13a58a6351.rmeta --extern either=/home/jelmer/src/janitor/target/debug/deps/libeither-accfca6c6f5e11c6.rmeta --extern event_listener=/home/jelmer/src/janitor/target/debug/deps/libevent_listener-55331feab369961e.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_intrusive=/home/jelmer/src/janitor/target/debug/deps/libfutures_intrusive-c085726410f20eaa.rmeta --extern futures_io=/home/jelmer/src/janitor/target/debug/deps/libfutures_io-40db0a981b134123.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-e2fb2d440b459c82.rmeta --extern hashbrown=/home/jelmer/src/janitor/target/debug/deps/libhashbrown-06a88afb9eab536b.rmeta --extern hashlink=/home/jelmer/src/janitor/target/debug/deps/libhashlink-59dbdb8fc63c8797.rmeta --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-9a486ccb6575c0f1.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-400fd68aa602ed65.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-d6016ca7baa5f84b.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-9969bfe2b2f70651.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-f6f36781d1866faf.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-93d7573978769e30.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_core --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-core-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="_rt-async-std"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="any"' --cfg 'feature="async-io"' --cfg 'feature="async-std"' --cfg 'feature="chrono"' --cfg 'feature="crc"' --cfg 'feature="default"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="native-tls"' --cfg 'feature="offline"' --cfg 'feature="serde"' --cfg 'feature="serde_json"' --cfg 'feature="sha2"' --cfg 'feature="time"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_tls-native-tls", "_tls-none", "_tls-rustls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "any", "async-io", "async-std", "bigdecimal", "bit-vec", "bstr", "chrono", "crc", "default", "ipnetwork", "json", "mac_address", "migrate", "native-tls", "offline", "regex", "rust_decimal", "rustls", "rustls-native-certs", "rustls-pemfile", "serde", "serde_json", "sha2", "time", "tokio", "tokio-stream", "uuid", "webpki-roots"))' -C metadata=09705c10ca1a669f -C extra-filename=-e7cb44d99d43ea84 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_io=/home/jelmer/src/janitor/target/debug/deps/libasync_io-68c6881e06af5fb5.rmeta --extern async_std=/home/jelmer/src/janitor/target/debug/deps/libasync_std-538cf550a53fa4e6.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern crossbeam_queue=/home/jelmer/src/janitor/target/debug/deps/libcrossbeam_queue-577e4d13a58a6351.rmeta --extern either=/home/jelmer/src/janitor/target/debug/deps/libeither-5d949479ced69761.rmeta --extern event_listener=/home/jelmer/src/janitor/target/debug/deps/libevent_listener-55331feab369961e.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_intrusive=/home/jelmer/src/janitor/target/debug/deps/libfutures_intrusive-ff2b9c7fdd2577b0.rmeta --extern futures_io=/home/jelmer/src/janitor/target/debug/deps/libfutures_io-40db0a981b134123.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern hashbrown=/home/jelmer/src/janitor/target/debug/deps/libhashbrown-06a88afb9eab536b.rmeta --extern hashlink=/home/jelmer/src/janitor/target/debug/deps/libhashlink-59dbdb8fc63c8797.rmeta --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-9a486ccb6575c0f1.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-9ca42638756f24e8.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-59505d94661b74c2.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-93a16e2f2de5f6ed.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling tokio-util v0.7.14
   Compiling tower v0.5.2
   Compiling tokio-native-tls v0.3.1
   Compiling async-compression v0.4.23
   Compiling pyo3-async-runtimes v0.22.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio_util --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-util-0.7.14/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(loom)' --check-cfg 'cfg(mio_unsupported_force_poll_poll)' --check-cfg 'cfg(tokio_allow_from_blocking_fd)' --check-cfg 'cfg(tokio_internal_mt_counters)' --check-cfg 'cfg(tokio_no_parking_lot)' --check-cfg 'cfg(tokio_no_tuning_tests)' --check-cfg 'cfg(tokio_taskdump)' --check-cfg 'cfg(tokio_unstable)' --cfg 'feature="codec"' --cfg 'feature="default"' --cfg 'feature="io"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__docs_rs", "codec", "compat", "default", "full", "futures-io", "futures-util", "hashbrown", "io", "io-util", "net", "rt", "slab", "time", "tracing"))' -C metadata=1500ff54ed1726c4 -C extra-filename=-1715e889e03ebfb2 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_sink=/home/jelmer/src/janitor/target/debug/deps/libfutures_sink-0f1aae5d0426fde7.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tower --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tower-0.5.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="__common"' --cfg 'feature="futures-core"' --cfg 'feature="futures-util"' --cfg 'feature="log"' --cfg 'feature="make"' --cfg 'feature="pin-project-lite"' --cfg 'feature="sync_wrapper"' --cfg 'feature="timeout"' --cfg 'feature="tokio"' --cfg 'feature="tracing"' --cfg 'feature="util"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__common", "balance", "buffer", "discover", "filter", "full", "futures-core", "futures-util", "hdrhistogram", "hedge", "indexmap", "limit", "load", "load-shed", "log", "make", "pin-project-lite", "ready-cache", "reconnect", "retry", "slab", "spawn-ready", "steer", "sync_wrapper", "timeout", "tokio", "tokio-stream", "tokio-util", "tracing", "util"))' -C metadata=dc2a9637455ebc8f -C extra-filename=-f136ba52ebefd664 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --extern tower_layer=/home/jelmer/src/janitor/target/debug/deps/libtower_layer-fff117c1c28e545a.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio_native_tls --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-native-tls-0.3.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("vendored"))' -C metadata=3d876c49f9c6ed57 -C extra-filename=-6ad1bb228c67dbfa --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-9ca42638756f24e8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name async_compression --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/async-compression-0.4.23/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="flate2"' --cfg 'feature="gzip"' --cfg 'feature="tokio"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("all", "all-algorithms", "all-implementations", "brotli", "bzip2", "deflate", "deflate64", "flate2", "futures-io", "gzip", "libzstd", "lz4", "lzma", "tokio", "xz", "xz2", "zlib", "zstd", "zstd-safe", "zstdmt"))' -C metadata=7468f33de07895b7 -C extra-filename=-252274b96f2057ac --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name pyo3_async_runtimes --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pyo3-async-runtimes-0.22.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="tokio"' --cfg 'feature="tokio-runtime"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("async-channel", "async-std", "async-std-runtime", "attributes", "clap", "default", "inventory", "pyo3-async-runtimes-macros", "testing", "tokio", "tokio-runtime", "unstable-streams"))' -C metadata=560c8eaae3c23ef3 -C extra-filename=-0dab235484cc69b2 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern futures=/home/jelmer/src/janitor/target/debug/deps/libfutures-463765cbcde647a0.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-d43e1a0d42aa1c27.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling h2 v0.4.8
   Compiling combine v4.6.7
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name h2 --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/h2-0.4.8/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("stream", "unstable"))' -C metadata=29cba5c19d0f1936 -C extra-filename=-93e1da2dd2a3c2d5 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern atomic_waker=/home/jelmer/src/janitor/target/debug/deps/libatomic_waker-21f0b624b8878034.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern fnv=/home/jelmer/src/janitor/target/debug/deps/libfnv-135eca59eff18b18.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_sink=/home/jelmer/src/janitor/target/debug/deps/libfutures_sink-0f1aae5d0426fde7.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-9a486ccb6575c0f1.rmeta --extern slab=/home/jelmer/src/janitor/target/debug/deps/libslab-58feeb60e58ddd09.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-1715e889e03ebfb2.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name combine --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/combine-4.6.7/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="alloc"' --cfg 'feature="bytes"' --cfg 'feature="futures-core-03"' --cfg 'feature="pin-project-lite"' --cfg 'feature="std"' --cfg 'feature="tokio"' --cfg 'feature="tokio-dep"' --cfg 'feature="tokio-util"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alloc", "bytes", "bytes_05", "default", "futures-03", "futures-core-03", "futures-io-03", "mp4", "pin-project", "pin-project-lite", "regex", "std", "tokio", "tokio-02", "tokio-02-dep", "tokio-03", "tokio-03-dep", "tokio-dep", "tokio-util"))' -C metadata=2cc5f6d8c381ac49 -C extra-filename=-4d80f43cbb3bf6fb --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_core_03=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern tokio_dep=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-1715e889e03ebfb2.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling sqlx-postgres v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_postgres --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-postgres-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="any"' --cfg 'feature="chrono"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="time"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("any", "bigdecimal", "bit-vec", "chrono", "ipnetwork", "json", "mac_address", "migrate", "offline", "rust_decimal", "time", "uuid"))' -C metadata=204a2700221f1c0d -C extra-filename=-cee829b62c7443fd --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern atoi=/home/jelmer/src/janitor/target/debug/deps/libatoi-08701d6ef2ff6341.rmeta --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-b1a4de7ed4da6927.rmeta --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern byteorder=/home/jelmer/src/janitor/target/debug/deps/libbyteorder-99211d86bad9f8bb.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern dotenvy=/home/jelmer/src/janitor/target/debug/deps/libdotenvy-c5aead2ec43889b7.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern hkdf=/home/jelmer/src/janitor/target/debug/deps/libhkdf-46ecc70a36f2ad04.rmeta --extern hmac=/home/jelmer/src/janitor/target/debug/deps/libhmac-d886aec669ee88c9.rmeta --extern home=/home/jelmer/src/janitor/target/debug/deps/libhome-fc8b76e8d1b394eb.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern md5=/home/jelmer/src/janitor/target/debug/deps/libmd5-e96a7bc866d34328.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-4ffe539611cdf71f.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-59505d94661b74c2.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-e7cb44d99d43ea84.rmeta --extern stringprep=/home/jelmer/src/janitor/target/debug/deps/libstringprep-3e84ecdea7cbe138.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-93a16e2f2de5f6ed.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern whoami=/home/jelmer/src/janitor/target/debug/deps/libwhoami-aa57d5d7654aa522.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_postgres --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-postgres-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="chrono"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="offline"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("any", "bigdecimal", "bit-vec", "chrono", "ipnetwork", "json", "mac_address", "migrate", "offline", "rust_decimal", "time", "uuid"))' -C metadata=10549588d67e3ab3 -C extra-filename=-9a76ee04d51a06ae --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern atoi=/home/jelmer/src/janitor/target/debug/deps/libatoi-f8455101c6ea3fc4.rmeta --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-bf6eccdff131582a.rmeta --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern byteorder=/home/jelmer/src/janitor/target/debug/deps/libbyteorder-99211d86bad9f8bb.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-4eb151582e08ecdb.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern dotenvy=/home/jelmer/src/janitor/target/debug/deps/libdotenvy-c5aead2ec43889b7.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-1f4beae7161f5951.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-e2fb2d440b459c82.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern hkdf=/home/jelmer/src/janitor/target/debug/deps/libhkdf-46ecc70a36f2ad04.rmeta --extern hmac=/home/jelmer/src/janitor/target/debug/deps/libhmac-d886aec669ee88c9.rmeta --extern home=/home/jelmer/src/janitor/target/debug/deps/libhome-fc8b76e8d1b394eb.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-400fd68aa602ed65.rmeta --extern md5=/home/jelmer/src/janitor/target/debug/deps/libmd5-e96a7bc866d34328.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-b175db0867abff37.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-9969bfe2b2f70651.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-ec5d4634e2bc5bab.rmeta --extern stringprep=/home/jelmer/src/janitor/target/debug/deps/libstringprep-3e84ecdea7cbe138.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-f6f36781d1866faf.rmeta --extern whoami=/home/jelmer/src/janitor/target/debug/deps/libwhoami-aa57d5d7654aa522.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper v1.6.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-1.6.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(hyper_unstable_tracing)' --check-cfg 'cfg(hyper_unstable_ffi)' --cfg 'feature="client"' --cfg 'feature="default"' --cfg 'feature="http1"' --cfg 'feature="http2"' --cfg 'feature="server"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("capi", "client", "default", "ffi", "full", "http1", "http2", "nightly", "server", "tracing"))' -C metadata=75a9ecc69623d8b8 -C extra-filename=-951ccb6902778a1f --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern h2=/home/jelmer/src/janitor/target/debug/deps/libh2-93e1da2dd2a3c2d5.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern httparse=/home/jelmer/src/janitor/target/debug/deps/libhttparse-de9e4dfe0f78db23.rmeta --extern httpdate=/home/jelmer/src/janitor/target/debug/deps/libhttpdate-66eb51e4c8d24adc.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-59505d94661b74c2.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --extern want=/home/jelmer/src/janitor/target/debug/deps/libwant-676b1650d2642fde.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling redis v0.27.6
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name redis --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/redis-0.27.6/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="acl"' --cfg 'feature="aio"' --cfg 'feature="async-trait"' --cfg 'feature="backon"' --cfg 'feature="bytes"' --cfg 'feature="connection-manager"' --cfg 'feature="default"' --cfg 'feature="futures"' --cfg 'feature="futures-util"' --cfg 'feature="geospatial"' --cfg 'feature="json"' --cfg 'feature="keep-alive"' --cfg 'feature="pin-project-lite"' --cfg 'feature="script"' --cfg 'feature="serde"' --cfg 'feature="serde_json"' --cfg 'feature="sha1_smol"' --cfg 'feature="socket2"' --cfg 'feature="streams"' --cfg 'feature="tokio"' --cfg 'feature="tokio-comp"' --cfg 'feature="tokio-util"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("acl", "ahash", "aio", "async-native-tls", "async-std", "async-std-comp", "async-std-native-tls-comp", "async-std-rustls-comp", "async-std-tls-comp", "async-trait", "backon", "bigdecimal", "bytes", "cluster", "cluster-async", "connection-manager", "crc16", "default", "disable-client-setinfo", "futures", "futures-rustls", "futures-util", "geospatial", "hashbrown", "json", "keep-alive", "log", "native-tls", "num-bigint", "pin-project-lite", "r2d2", "rand", "rust_decimal", "rustls", "rustls-native-certs", "rustls-pemfile", "rustls-pki-types", "script", "sentinel", "serde", "serde_json", "sha1_smol", "socket2", "streams", "tcp_nodelay", "tls", "tls-native-tls", "tls-rustls", "tls-rustls-insecure", "tls-rustls-webpki-roots", "tokio", "tokio-comp", "tokio-native-tls", "tokio-native-tls-comp", "tokio-rustls", "tokio-rustls-comp", "tokio-util", "uuid", "webpki-roots"))' -C metadata=feba75cceaf515fc -C extra-filename=-d487a15a53645fe9 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern arc_swap=/home/jelmer/src/janitor/target/debug/deps/libarc_swap-bd5aa4a1e22f9e5d.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern backon=/home/jelmer/src/janitor/target/debug/deps/libbackon-9baa21c6e034bb14.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern combine=/home/jelmer/src/janitor/target/debug/deps/libcombine-4d80f43cbb3bf6fb.rmeta --extern futures=/home/jelmer/src/janitor/target/debug/deps/libfutures-463765cbcde647a0.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern itertools=/home/jelmer/src/janitor/target/debug/deps/libitertools-a8fb045921351b76.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern num_bigint=/home/jelmer/src/janitor/target/debug/deps/libnum_bigint-7a7a2dd2f34962d4.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern ryu=/home/jelmer/src/janitor/target/debug/deps/libryu-245a84a5a509b3a3.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha1_smol=/home/jelmer/src/janitor/target/debug/deps/libsha1_smol-03061bab6b3928dd.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-9be7658adf7d58e8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-1715e889e03ebfb2.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper-util v0.1.10
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper_util --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-util-0.1.10/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="client"' --cfg 'feature="client-legacy"' --cfg 'feature="default"' --cfg 'feature="http1"' --cfg 'feature="http2"' --cfg 'feature="server"' --cfg 'feature="service"' --cfg 'feature="tokio"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__internal_happy_eyeballs_tests", "client", "client-legacy", "default", "full", "http1", "http2", "server", "server-auto", "server-graceful", "service", "tokio"))' -C metadata=cb8aba50be9fb950 -C extra-filename=-8c1815b1cb91ec3d --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-951ccb6902778a1f.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-9be7658adf7d58e8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling sqlx-macros-core v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_macros_core --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-macros-core-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no --warn=unexpected_cfgs --cfg 'feature="_rt-async-std"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="async-std"' --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="json"' --cfg 'feature="macros"' --cfg 'feature="migrate"' --cfg 'feature="postgres"' --cfg 'feature="sqlx-postgres"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_sqlite", "_tls-native-tls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "async-std", "bigdecimal", "bit-vec", "chrono", "default", "derive", "ipnetwork", "json", "mac_address", "macros", "migrate", "mysql", "postgres", "rust_decimal", "sqlite", "sqlite-unbundled", "sqlx-mysql", "sqlx-postgres", "sqlx-sqlite", "time", "tokio", "uuid"))' -C metadata=d71ee1a8f3a619d3 -C extra-filename=-5865014c5ff52363 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_std=/home/jelmer/src/janitor/target/debug/deps/libasync_std-220d57f9d1d250bf.rmeta --extern dotenvy=/home/jelmer/src/janitor/target/debug/deps/libdotenvy-c5aead2ec43889b7.rmeta --extern either=/home/jelmer/src/janitor/target/debug/deps/libeither-accfca6c6f5e11c6.rmeta --extern heck=/home/jelmer/src/janitor/target/debug/deps/libheck-4d6a9c8516811f18.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern proc_macro2=/home/jelmer/src/janitor/target/debug/deps/libproc_macro2-a7e2001652539cec.rmeta --extern quote=/home/jelmer/src/janitor/target/debug/deps/libquote-8533776b6f1db290.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-ec5d4634e2bc5bab.rmeta --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-9a76ee04d51a06ae.rmeta --extern syn=/home/jelmer/src/janitor/target/debug/deps/libsyn-7fe0b75e1b133791.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-5cc214a3774c4b08.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-93d7573978769e30.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper-tls v0.6.0
   Compiling axum v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper_tls --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-tls-0.6.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alpn", "vendored"))' -C metadata=cc79f5376e2f7d4c -C extra-filename=-5b83912f299e7b9b --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-951ccb6902778a1f.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-8c1815b1cb91ec3d.rmeta --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-9ca42638756f24e8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --extern tokio_native_tls=/home/jelmer/src/janitor/target/debug/deps/libtokio_native_tls-6ad1bb228c67dbfa.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name axum --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/axum-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::verbose_file_reads' '--warn=clippy::unused_self' --forbid=unsafe_code --warn=unreachable_pub '--warn=clippy::unnested_or_patterns' '--warn=clippy::uninlined_format_args' '--allow=clippy::type_complexity' '--warn=clippy::todo' '--warn=clippy::suboptimal_flops' '--warn=clippy::str_to_string' '--warn=clippy::rest_pat_in_fully_bound_structs' '--warn=clippy::option_option' '--warn=clippy::needless_continue' '--warn=clippy::needless_borrow' --warn=missing_docs --warn=missing_debug_implementations '--warn=clippy::mem_forget' '--warn=clippy::match_wildcard_for_single_variants' '--warn=clippy::match_on_vec_items' '--warn=clippy::macro_use_imports' '--warn=clippy::lossy_float_literal' '--warn=clippy::linkedlist' '--warn=clippy::inefficient_to_string' '--warn=clippy::imprecise_flops' '--warn=clippy::if_let_mutex' '--warn=clippy::fn_params_excessive_bools' '--warn=clippy::filter_map_next' '--warn=clippy::exit' '--warn=clippy::enum_glob_use' '--warn=clippy::empty_enum' '--warn=clippy::dbg_macro' '--warn=clippy::await_holding_lock' --cfg 'feature="default"' --cfg 'feature="form"' --cfg 'feature="http1"' --cfg 'feature="json"' --cfg 'feature="matched-path"' --cfg 'feature="original-uri"' --cfg 'feature="query"' --cfg 'feature="tokio"' --cfg 'feature="tower-log"' --cfg 'feature="tracing"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__private", "__private_docs", "default", "form", "http1", "http2", "json", "macros", "matched-path", "multipart", "original-uri", "query", "tokio", "tower-log", "tracing", "ws"))' -C metadata=f0fe0bb4495caf8a -C extra-filename=-ffda23bb6061aea3 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern axum_core=/home/jelmer/src/janitor/target/debug/deps/libaxum_core-c7084f1580c648d3.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern form_urlencoded=/home/jelmer/src/janitor/target/debug/deps/libform_urlencoded-072fa14f50efb53e.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-951ccb6902778a1f.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-8c1815b1cb91ec3d.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern matchit=/home/jelmer/src/janitor/target/debug/deps/libmatchit-0d71d298d63a0df3.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern mime=/home/jelmer/src/janitor/target/debug/deps/libmime-c74000db2aaf511f.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern rustversion=/home/jelmer/src/janitor/target/debug/deps/librustversion-494b2fd16358ba50.so --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_path_to_error=/home/jelmer/src/janitor/target/debug/deps/libserde_path_to_error-72e72ae8986ce543.rmeta --extern serde_urlencoded=/home/jelmer/src/janitor/target/debug/deps/libserde_urlencoded-e0562f68e1545f98.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --extern tower=/home/jelmer/src/janitor/target/debug/deps/libtower-f136ba52ebefd664.rmeta --extern tower_layer=/home/jelmer/src/janitor/target/debug/deps/libtower_layer-fff117c1c28e545a.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling reqwest v0.12.15
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name reqwest --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/reqwest-0.12.15/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(reqwest_unstable)' --cfg 'feature="__tls"' --cfg 'feature="blocking"' --cfg 'feature="charset"' --cfg 'feature="default"' --cfg 'feature="default-tls"' --cfg 'feature="h2"' --cfg 'feature="http2"' --cfg 'feature="json"' --cfg 'feature="macos-system-configuration"' --cfg 'feature="multipart"' --cfg 'feature="stream"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__rustls", "__rustls-ring", "__tls", "blocking", "brotli", "charset", "cookies", "default", "default-tls", "deflate", "gzip", "h2", "hickory-dns", "http2", "http3", "json", "macos-system-configuration", "multipart", "native-tls", "native-tls-alpn", "native-tls-vendored", "rustls-tls", "rustls-tls-manual-roots", "rustls-tls-manual-roots-no-provider", "rustls-tls-native-roots", "rustls-tls-native-roots-no-provider", "rustls-tls-no-provider", "rustls-tls-webpki-roots", "rustls-tls-webpki-roots-no-provider", "socks", "stream", "trust-dns", "zstd"))' -C metadata=2199e53410c451de -C extra-filename=-d424a5639312b20a --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-b1a4de7ed4da6927.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern encoding_rs=/home/jelmer/src/janitor/target/debug/deps/libencoding_rs-58c41e4932181cd7.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern h2=/home/jelmer/src/janitor/target/debug/deps/libh2-93e1da2dd2a3c2d5.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-951ccb6902778a1f.rmeta --extern hyper_tls=/home/jelmer/src/janitor/target/debug/deps/libhyper_tls-5b83912f299e7b9b.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-8c1815b1cb91ec3d.rmeta --extern ipnet=/home/jelmer/src/janitor/target/debug/deps/libipnet-5873e4e1530bf49f.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern mime=/home/jelmer/src/janitor/target/debug/deps/libmime-c74000db2aaf511f.rmeta --extern mime_guess=/home/jelmer/src/janitor/target/debug/deps/libmime_guess-7ee1813410f2722d.rmeta --extern native_tls_crate=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-9ca42638756f24e8.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern rustls_pemfile=/home/jelmer/src/janitor/target/debug/deps/librustls_pemfile-68bb2d10b5046659.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_urlencoded=/home/jelmer/src/janitor/target/debug/deps/libserde_urlencoded-e0562f68e1545f98.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --extern tokio_native_tls=/home/jelmer/src/janitor/target/debug/deps/libtokio_native_tls-6ad1bb228c67dbfa.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-1715e889e03ebfb2.rmeta --extern tower=/home/jelmer/src/janitor/target/debug/deps/libtower-f136ba52ebefd664.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling sqlx-macros v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_macros --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-macros-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type proc-macro --emit=dep-info,link -C prefer-dynamic -C embed-bitcode=no '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="_rt-async-std"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="json"' --cfg 'feature="macros"' --cfg 'feature="migrate"' --cfg 'feature="postgres"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_tls-native-tls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "bigdecimal", "bit-vec", "chrono", "default", "derive", "ipnetwork", "json", "mac_address", "macros", "migrate", "mysql", "postgres", "rust_decimal", "sqlite", "sqlite-unbundled", "time", "uuid"))' -C metadata=74daf4d4c922ee0e -C extra-filename=-92bb779065197e03 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern proc_macro2=/home/jelmer/src/janitor/target/debug/deps/libproc_macro2-a7e2001652539cec.rlib --extern quote=/home/jelmer/src/janitor/target/debug/deps/libquote-8533776b6f1db290.rlib --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-ec5d4634e2bc5bab.rlib --extern sqlx_macros_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_macros_core-5865014c5ff52363.rlib --extern syn=/home/jelmer/src/janitor/target/debug/deps/libsyn-7fe0b75e1b133791.rlib --extern proc_macro --cap-lints allow --cfg tokio_unstable`
   Compiling google-cloud-metadata v0.5.1
   Compiling reqwest-middleware v0.3.3
   Compiling prometheus v0.14.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name google_cloud_metadata --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/google-cloud-metadata-0.5.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=5eb85a170755a93b -C extra-filename=-d1e4ee1b4704d31c --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-d424a5639312b20a.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name reqwest_middleware --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/reqwest-middleware-0.3.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="json"' --cfg 'feature="multipart"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("charset", "http2", "json", "multipart", "rustls-tls"))' -C metadata=790aa952ef817e17 -C extra-filename=-b611775e3dc0ef91 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern anyhow=/home/jelmer/src/janitor/target/debug/deps/libanyhow-904a89ff6dd1202e.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-d424a5639312b20a.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name prometheus --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/prometheus-0.14.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="protobuf"' --cfg 'feature="reqwest"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "gen", "libc", "nightly", "process", "procfs", "protobuf", "protobuf-codegen", "push", "reqwest"))' -C metadata=873c957b04f14e14 -C extra-filename=-7084c37394f9a2a9 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern cfg_if=/home/jelmer/src/janitor/target/debug/deps/libcfg_if-c30e1cd0415bac84.rmeta --extern fnv=/home/jelmer/src/janitor/target/debug/deps/libfnv-135eca59eff18b18.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern parking_lot=/home/jelmer/src/janitor/target/debug/deps/libparking_lot-83fcc7478224180d.rmeta --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-b1e3dc89f529da22.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-d424a5639312b20a.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling google-cloud-auth v0.17.2
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name google_cloud_auth --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/google-cloud-auth-0.17.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="default-tls"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "default-tls", "external-account", "hex", "hickory-dns", "hmac", "path-clean", "percent-encoding", "rustls-tls", "sha2", "url"))' -C metadata=d5669665203648cb -C extra-filename=-b943e1ee0466e8a5 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-41324bb9dba3dad5.rmeta --extern google_cloud_metadata=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_metadata-d1e4ee1b4704d31c.rmeta --extern google_cloud_token=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_token-6145b7093dd432ee.rmeta --extern home=/home/jelmer/src/janitor/target/debug/deps/libhome-fc8b76e8d1b394eb.rmeta --extern jsonwebtoken=/home/jelmer/src/janitor/target/debug/deps/libjsonwebtoken-605d0c3ad415f9db.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-d424a5639312b20a.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-93a16e2f2de5f6ed.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern urlencoding=/home/jelmer/src/janitor/target/debug/deps/liburlencoding-0ba1b8b89d728edb.rmeta --cap-lints allow --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
   Compiling sqlx v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="_rt-async-std"' --cfg 'feature="any"' --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="json"' --cfg 'feature="macros"' --cfg 'feature="migrate"' --cfg 'feature="postgres"' --cfg 'feature="runtime-async-std"' --cfg 'feature="runtime-async-std-native-tls"' --cfg 'feature="sqlx-macros"' --cfg 'feature="sqlx-postgres"' --cfg 'feature="tls-native-tls"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_sqlite", "_unstable-all-types", "all-databases", "any", "bigdecimal", "bit-vec", "chrono", "default", "derive", "ipnetwork", "json", "mac_address", "macros", "migrate", "mysql", "postgres", "regexp", "runtime-async-std", "runtime-async-std-native-tls", "runtime-async-std-rustls", "runtime-tokio", "runtime-tokio-native-tls", "runtime-tokio-rustls", "rust_decimal", "sqlite", "sqlite-unbundled", "sqlx-macros", "sqlx-mysql", "sqlx-postgres", "sqlx-sqlite", "time", "tls-native-tls", "tls-none", "tls-rustls", "tls-rustls-aws-lc-rs", "tls-rustls-ring", "tls-rustls-ring-native-roots", "tls-rustls-ring-webpki", "uuid"))' -C metadata=e548c0bd839ab829 -C extra-filename=-1643eb75ff35c7a0 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-e7cb44d99d43ea84.rmeta --extern sqlx_macros=/home/jelmer/src/janitor/target/debug/deps/libsqlx_macros-92bb779065197e03.so --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-cee829b62c7443fd.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling debversion v0.4.4
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debversion --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debversion-0.4.4/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="python-debian"' --cfg 'feature="serde"' --cfg 'feature="sqlx"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "python-debian", "serde", "sqlx"))' -C metadata=63e608f74fc93dd6 -C extra-filename=-da2cb4265ece088c --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-d43e1a0d42aa1c27.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-1643eb75ff35c7a0.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling debian-control v0.1.41
   Compiling debian-changelog v0.2.0
   Compiling debian-copyright v0.1.27
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_control --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-control-0.1.41/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="lossless"' --cfg 'feature="python-debian"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("chrono", "default", "lossless", "python-debian", "serde"))' -C metadata=aa7a9d8903630867 -C extra-filename=-00c78b0d40494b88 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-a0457559d645290d.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-da2cb4265ece088c.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-d43e1a0d42aa1c27.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern rowan=/home/jelmer/src/janitor/target/debug/deps/librowan-e05f61e8ea0a6615.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_changelog --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-changelog-0.2.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=c8577e9329945cb5 -C extra-filename=-959e755772e6312a --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-da2cb4265ece088c.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern rowan=/home/jelmer/src/janitor/target/debug/deps/librowan-c1fdca08b3081a85.rmeta --extern textwrap=/home/jelmer/src/janitor/target/debug/deps/libtextwrap-5e0992fd5b607969.rmeta --extern whoami=/home/jelmer/src/janitor/target/debug/deps/libwhoami-aa57d5d7654aa522.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_copyright --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-copyright-0.1.27/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=8308bbdff60f19d0 -C extra-filename=-decbc504c684e227 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-a0457559d645290d.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-da2cb4265ece088c.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling google-cloud-storage v0.22.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name google_cloud_storage --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/google-cloud-storage-0.22.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="auth"' --cfg 'feature="default"' --cfg 'feature="default-tls"' --cfg 'feature="google-cloud-auth"' --cfg 'feature="google-cloud-metadata"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("auth", "default", "default-tls", "external-account", "google-cloud-auth", "google-cloud-metadata", "hickory-dns", "rustls-tls", "trace"))' -C metadata=823f328cd5c02e9c -C extra-filename=-600c0df99df783f3 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern anyhow=/home/jelmer/src/janitor/target/debug/deps/libanyhow-904a89ff6dd1202e.rmeta --extern async_stream=/home/jelmer/src/janitor/target/debug/deps/libasync_stream-f0f1e6ef812a7b6c.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-41324bb9dba3dad5.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern google_cloud_auth=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_auth-b943e1ee0466e8a5.rmeta --extern google_cloud_metadata=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_metadata-d1e4ee1b4704d31c.rmeta --extern google_cloud_token=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_token-6145b7093dd432ee.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pkcs8=/home/jelmer/src/janitor/target/debug/deps/libpkcs8-ef54810b56a401a1.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-d424a5639312b20a.rmeta --extern reqwest_middleware=/home/jelmer/src/janitor/target/debug/deps/libreqwest_middleware-b611775e3dc0ef91.rmeta --extern ring=/home/jelmer/src/janitor/target/debug/deps/libring-5cd153576e85d8b3.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-93a16e2f2de5f6ed.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
   Compiling breezyshim v0.1.227
   Compiling buildlog-consultant v0.1.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name breezyshim --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/breezyshim-0.1.227/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="auto-initialize"' --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="dirty-tracker"' --cfg 'feature="sqlx"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("auto-initialize", "debian", "default", "dirty-tracker", "sqlx"))' -C metadata=e88cce31b337259f -C extra-filename=-8f48b542fabe92c2 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern ctor=/home/jelmer/src/janitor/target/debug/deps/libctor-72258acac2d0b9ee.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-959e755772e6312a.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-00c78b0d40494b88.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-da2cb4265ece088c.rmeta --extern difflib=/home/jelmer/src/janitor/target/debug/deps/libdifflib-20593172ebc6932e.rmeta --extern dirty_tracker=/home/jelmer/src/janitor/target/debug/deps/libdirty_tracker-6abba3579c29934f.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern patchkit=/home/jelmer/src/janitor/target/debug/deps/libpatchkit-045f9ba36150b582.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-d43e1a0d42aa1c27.rmeta --extern pyo3_filelike=/home/jelmer/src/janitor/target/debug/deps/libpyo3_filelike-4b7eedc52e56f8d8.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-1643eb75ff35c7a0.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name buildlog_consultant --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/buildlog-consultant-0.1.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("chatgpt", "cli", "default", "tokio"))' -C metadata=4fbe40aad7f691e3 -C extra-filename=-72d717d5da23331f --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-00c78b0d40494b88.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-da2cb4265ece088c.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-808eaebb057ce37e.rmeta --extern fancy_regex=/home/jelmer/src/janitor/target/debug/deps/libfancy_regex-0688edb11485e39a.rmeta --extern inventory=/home/jelmer/src/janitor/target/debug/deps/libinventory-97a54ddffe78909c.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern pep508_rs=/home/jelmer/src/janitor/target/debug/deps/libpep508_rs-9aa259a9ee5b2c33.rlib --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_yaml=/home/jelmer/src/janitor/target/debug/deps/libserde_yaml-d19b32863dd48a61.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern text_size=/home/jelmer/src/janitor/target/debug/deps/libtext_size-68834c6d82d5a146.rmeta --extern textwrap=/home/jelmer/src/janitor/target/debug/deps/libtextwrap-5e0992fd5b607969.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling axum-extra v0.10.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name axum_extra --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/axum-extra-0.10.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::verbose_file_reads' '--warn=clippy::unused_self' --forbid=unsafe_code --warn=unreachable_pub '--warn=clippy::unnested_or_patterns' '--warn=clippy::uninlined_format_args' '--allow=clippy::type_complexity' '--warn=clippy::todo' '--warn=clippy::suboptimal_flops' '--warn=clippy::str_to_string' '--warn=clippy::rest_pat_in_fully_bound_structs' '--warn=clippy::option_option' '--warn=clippy::needless_continue' '--warn=clippy::needless_borrow' --warn=missing_docs --warn=missing_debug_implementations '--warn=clippy::mem_forget' '--warn=clippy::match_wildcard_for_single_variants' '--warn=clippy::match_on_vec_items' '--warn=clippy::macro_use_imports' '--warn=clippy::lossy_float_literal' '--warn=clippy::linkedlist' '--warn=clippy::inefficient_to_string' '--warn=clippy::imprecise_flops' '--warn=clippy::if_let_mutex' '--warn=clippy::fn_params_excessive_bools' '--warn=clippy::filter_map_next' '--warn=clippy::exit' '--warn=clippy::enum_glob_use' '--warn=clippy::empty_enum' '--warn=clippy::dbg_macro' '--warn=clippy::await_holding_lock' --cfg 'feature="default"' --cfg 'feature="tracing"' --cfg 'feature="typed-header"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__private_docs", "async-read-body", "async-stream", "attachment", "cookie", "cookie-key-expansion", "cookie-private", "cookie-signed", "default", "erased-json", "error-response", "file-stream", "form", "json-deserializer", "json-lines", "multipart", "protobuf", "query", "scheme", "tracing", "typed-header", "typed-routing"))' -C metadata=44c2a0984ea81545 -C extra-filename=-b118dae5f04c8879 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-ffda23bb6061aea3.rmeta --extern axum_core=/home/jelmer/src/janitor/target/debug/deps/libaxum_core-c7084f1580c648d3.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern headers=/home/jelmer/src/janitor/target/debug/deps/libheaders-c3724028989d4414.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern mime=/home/jelmer/src/janitor/target/debug/deps/libmime-c74000db2aaf511f.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern rustversion=/home/jelmer/src/janitor/target/debug/deps/librustversion-494b2fd16358ba50.so --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern tower=/home/jelmer/src/janitor/target/debug/deps/libtower-f136ba52ebefd664.rmeta --extern tower_layer=/home/jelmer/src/janitor/target/debug/deps/libtower_layer-fff117c1c28e545a.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling rslock v0.5.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name rslock --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rslock-0.5.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="tokio-comp"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("async-std-comp", "default", "tokio-comp"))' -C metadata=dfdf5f894da08b22 -C extra-filename=-979e3c4182eb960b --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern futures=/home/jelmer/src/janitor/target/debug/deps/libfutures-463765cbcde647a0.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-4ffe539611cdf71f.rmeta --extern redis=/home/jelmer/src/janitor/target/debug/deps/libredis-d487a15a53645fe9.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling debian-analyzer v0.158.25
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_analyzer --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-analyzer-0.158.25/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="merge3"' --cfg 'feature="python"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "default", "merge3", "python", "svp", "udd"))' -C metadata=a78de8134882a18b -C extra-filename=-96e8e1ca032519a4 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-8f48b542fabe92c2.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern configparser=/home/jelmer/src/janitor/target/debug/deps/libconfigparser-aaa60c0f437f3031.rmeta --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-a0457559d645290d.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-959e755772e6312a.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-00c78b0d40494b88.rmeta --extern debian_copyright=/home/jelmer/src/janitor/target/debug/deps/libdebian_copyright-decbc504c684e227.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-da2cb4265ece088c.rmeta --extern dep3=/home/jelmer/src/janitor/target/debug/deps/libdep3-37a9e7e8397c8b8a.rmeta --extern difflib=/home/jelmer/src/janitor/target/debug/deps/libdifflib-20593172ebc6932e.rmeta --extern distro_info=/home/jelmer/src/janitor/target/debug/deps/libdistro_info-b4d1b2c89b3969c8.rmeta --extern filetime=/home/jelmer/src/janitor/target/debug/deps/libfiletime-8f90a18bbe2253cd.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern makefile_lossless=/home/jelmer/src/janitor/target/debug/deps/libmakefile_lossless-f7c86ff44e7ff685.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern merge3=/home/jelmer/src/janitor/target/debug/deps/libmerge3-1c24ac3badc9ba5b.rmeta --extern patchkit=/home/jelmer/src/janitor/target/debug/deps/libpatchkit-045f9ba36150b582.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-d43e1a0d42aa1c27.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-d424a5639312b20a.rmeta --extern semver=/home/jelmer/src/janitor/target/debug/deps/libsemver-11fd74ac82b27f0f.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha1=/home/jelmer/src/janitor/target/debug/deps/libsha1-666ba0d12790bffa.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --extern toml_edit=/home/jelmer/src/janitor/target/debug/deps/libtoml_edit-51c35483d814c85f.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling silver-platter v0.5.48
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name silver_platter --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/silver-platter-0.5.48/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="detect-update-changelog"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default", "detect-update-changelog", "gpg", "last-attempt-db", "pyo3"))' -C metadata=7724a19e55c86b8b -C extra-filename=-ccb2e8ac0e11814a --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-8f48b542fabe92c2.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-96e8e1ca032519a4.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-959e755772e6312a.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-00c78b0d40494b88.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-da2cb4265ece088c.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-808eaebb057ce37e.rmeta --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-f96927166a8959d2.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-d43e1a0d42aa1c27.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-d51b72eab852ecda.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-d424a5639312b20a.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_yaml=/home/jelmer/src/janitor/target/debug/deps/libserde_yaml-d19b32863dd48a61.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --extern tera=/home/jelmer/src/janitor/target/debug/deps/libtera-d53e293f50202668.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --extern xdg=/home/jelmer/src/janitor/target/debug/deps/libxdg-23f110d46d019c5b.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor --edition=2021 src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="gcp"' --cfg 'feature="gcs"' --cfg 'feature="stackdriver_logger"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("debian", "default", "gcp", "gcs", "stackdriver_logger"))' -C metadata=e0869927ad7b9a3b -C extra-filename=-765846f441e28202 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e239d113cd99a25a.rmeta --extern async_compression=/home/jelmer/src/janitor/target/debug/deps/libasync_compression-252274b96f2057ac.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-8f48b542fabe92c2.rmeta --extern buildlog_consultant=/home/jelmer/src/janitor/target/debug/deps/libbuildlog_consultant-72d717d5da23331f.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-00c78b0d40494b88.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-da2cb4265ece088c.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-808eaebb057ce37e.rmeta --extern fancy_regex=/home/jelmer/src/janitor/target/debug/deps/libfancy_regex-0688edb11485e39a.rmeta --extern filetime=/home/jelmer/src/janitor/target/debug/deps/libfiletime-8f90a18bbe2253cd.rmeta --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern futures=/home/jelmer/src/janitor/target/debug/deps/libfutures-463765cbcde647a0.rmeta --extern google_cloud_auth=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_auth-b943e1ee0466e8a5.rmeta --extern google_cloud_storage=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_storage-600c0df99df783f3.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-7084c37394f9a2a9.rmeta --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-b1e3dc89f529da22.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-d43e1a0d42aa1c27.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-d424a5639312b20a.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-ccb2e8ac0e11814a.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-1643eb75ff35c7a0.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-e7cb44d99d43ea84.rmeta --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-cee829b62c7443fd.rmeta --extern stackdriver_logger=/home/jelmer/src/janitor/target/debug/deps/libstackdriver_logger-b45c27ef10c05d41.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
warning: function `reprocess_run_logs` is never used
 --> src/reprocess_logs.rs:8:10
  |
8 | async fn reprocess_run_logs(
  |          ^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(dead_code)]` on by default

warning: field `branch_url` is never read
  --> src/schedule.rs:32:5
   |
30 | pub struct ScheduleRequest {
   |            --------------- field in this struct
31 |     codebase: String,
32 |     branch_url: String,
   |     ^^^^^^^^^^

warning: function `has_cotenants` is never used
  --> src/state.rs:80:10
   |
80 | async fn has_cotenants(
   |          ^^^^^^^^^^^^^

warning: field `name` is never read
  --> src/state.rs:87:13
   |
86 |     struct Codebase {
   |            -------- field in this struct
87 |         pub name: String,
   |             ^^^^
   |
   = note: `Codebase` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: function `iter_publishable_suites` is never used
   --> src/state.rs:113:10
    |
113 | async fn iter_publishable_suites(
    |          ^^^^^^^^^^^^^^^^^^^^^^^

   Compiling janitor-publish v0.0.0 (/home/jelmer/src/janitor/publish)
   Compiling janitor-differ v0.0.0 (/home/jelmer/src/janitor/differ)
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor_publish --edition=2021 publish/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=2da014eb33eac734 -C extra-filename=-8dce75e71ae37b8d --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-ffda23bb6061aea3.rmeta --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-8f48b542fabe92c2.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-959e755772e6312a.rmeta --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-765846f441e28202.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern minijinja=/home/jelmer/src/janitor/target/debug/deps/libminijinja-6c444bb921e91f39.rmeta --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-7084c37394f9a2a9.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-d43e1a0d42aa1c27.rmeta --extern redis=/home/jelmer/src/janitor/target/debug/deps/libredis-d487a15a53645fe9.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-d424a5639312b20a.rmeta --extern rslock=/home/jelmer/src/janitor/target/debug/deps/librslock-979e3c4182eb960b.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-ccb2e8ac0e11814a.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-1643eb75ff35c7a0.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor_differ --edition=2021 differ/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "default"))' -C metadata=49ff259e4d75c513 -C extra-filename=-c40baa0ef9d4bf60 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern accept_header=/home/jelmer/src/janitor/target/debug/deps/libaccept_header-06753735896e5c37.rmeta --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-ffda23bb6061aea3.rmeta --extern axum_extra=/home/jelmer/src/janitor/target/debug/deps/libaxum_extra-b118dae5f04c8879.rmeta --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-8f48b542fabe92c2.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-808eaebb057ce37e.rmeta --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-765846f441e28202.rmeta --extern mime=/home/jelmer/src/janitor/target/debug/deps/libmime-c74000db2aaf511f.rmeta --extern nix=/home/jelmer/src/janitor/target/debug/deps/libnix-583de5e65a7ae36a.rmeta --extern patchkit=/home/jelmer/src/janitor/target/debug/deps/libpatchkit-045f9ba36150b582.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-d43e1a0d42aa1c27.rmeta --extern redis=/home/jelmer/src/janitor/target/debug/deps/libredis-d487a15a53645fe9.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-1643eb75ff35c7a0.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-15192f2ea77506d4.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
warning: unused import: `OsStr`
  --> differ/src/lib.rs:10:16
   |
10 | use std::ffi::{OsStr, OsString};
   |                ^^^^^
   |
   = note: `#[warn(unused_imports)]` on by default

warning: unused import: `Mode`
 --> publish/src/proposal_info.rs:3:45
  |
3 | use janitor::publish::{MergeProposalStatus, Mode};
  |                                             ^^^^
  |
  = note: `#[warn(unused_imports)]` on by default

warning: unused import: `url::Url`
 --> publish/src/proposal_info.rs:6:5
  |
6 | use url::Url;
  |     ^^^^^^^^

warning: unused import: `breezyshim::forge::Forge`
 --> publish/src/web.rs:9:5
  |
9 | use breezyshim::forge::Forge;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused imports: `VcsManager` and `VcsType`
  --> publish/src/web.rs:11:20
   |
11 | use janitor::vcs::{VcsManager, VcsType};
   |                    ^^^^^^^^^^  ^^^^^^^

warning: unused import: `Mutex`
  --> publish/src/web.rs:14:22
   |
14 | use std::sync::{Arc, Mutex};
   |                      ^^^^^

warning: use of deprecated function `std::env::home_dir`: This function's behavior may be unexpected on Windows. Consider using a crate from crates.io instead.
   --> publish/src/web.rs:210:29
    |
210 |     let ssh_dir = std::env::home_dir().unwrap().join(".ssh");
    |                             ^^^^^^^^
    |
    = note: `#[warn(deprecated)]` on by default

warning: unused variable: `state`
   --> publish/src/lib.rs:725:5
    |
725 |     state: Arc<AppState>,
    |     ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`
    |
    = note: `#[warn(unused_variables)]` on by default

warning: unused variable: `interval`
   --> publish/src/lib.rs:726:5
    |
726 |     interval: chrono::Duration,
    |     ^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_interval`

warning: unused variable: `auto_publish`
   --> publish/src/lib.rs:727:5
    |
727 |     auto_publish: bool,
    |     ^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_auto_publish`

warning: unused variable: `state`
   --> publish/src/lib.rs:739:36
    |
739 | pub async fn publish_pending_ready(state: Arc<AppState>) -> Result<(), PublishError> {
    |                                    ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `state`
   --> publish/src/lib.rs:785:31
    |
785 | pub async fn listen_to_runner(state: Arc<AppState>) {
    |                               ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `conn`
   --> publish/src/lib.rs:890:5
    |
890 |     conn: &sqlx::PgPool,
    |     ^^^^ help: if this is intentional, prefix it with an underscore: `_conn`

warning: unused variable: `redis`
   --> publish/src/lib.rs:891:5
    |
891 |     redis: Option<redis::aio::ConnectionManager>,
    |     ^^^^^ help: if this is intentional, prefix it with an underscore: `_redis`

warning: unused variable: `config`
   --> publish/src/lib.rs:892:5
    |
892 |     config: &janitor::config::Config,
    |     ^^^^^^ help: if this is intentional, prefix it with an underscore: `_config`

warning: unused variable: `publish_worker`
   --> publish/src/lib.rs:893:5
    |
893 |     publish_worker: &crate::PublishWorker,
    |     ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_publish_worker`

warning: unused variable: `mp`
   --> publish/src/lib.rs:894:5
    |
894 |     mp: &breezyshim::forge::MergeProposal,
    |     ^^ help: if this is intentional, prefix it with an underscore: `_mp`

warning: unused variable: `status`
   --> publish/src/lib.rs:895:5
    |
895 |     status: breezyshim::forge::MergeProposalStatus,
    |     ^^^^^^ help: if this is intentional, prefix it with an underscore: `_status`

warning: unused variable: `vcs_managers`
   --> publish/src/lib.rs:896:5
    |
896 |     vcs_managers: &HashMap<VcsType, Box<dyn VcsManager>>,
    |     ^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_vcs_managers`

warning: unused variable: `bucket_rate_limiter`
   --> publish/src/lib.rs:897:5
    |
897 |     bucket_rate_limiter: &Mutex<Box<dyn crate::rate_limiter::RateLimiter>>,
    |     ^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_bucket_rate_limiter`

warning: unused variable: `check_only`
   --> publish/src/lib.rs:898:5
    |
898 |     check_only: bool,
    |     ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_check_only`

warning: unused variable: `mps_per_bucket`
   --> publish/src/lib.rs:899:5
    |
899 |     mps_per_bucket: Option<
    |     ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_mps_per_bucket`

warning: unused variable: `possible_transports`
   --> publish/src/lib.rs:902:5
    |
902 |     possible_transports: Option<&mut Vec<breezyshim::transport::Transport>>,
    |     ^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_possible_transports`

warning: unused variable: `bucket`
    --> publish/src/lib.rs:1062:14
     |
1062 |         for (bucket, count) in mps_per_bucket
     |              ^^^^^^ help: if this is intentional, prefix it with an underscore: `_bucket`

warning: unused variable: `conn`
    --> publish/src/lib.rs:1078:5
     |
1078 |     conn: &sqlx::PgPool,
     |     ^^^^ help: if this is intentional, prefix it with an underscore: `_conn`

warning: unused variable: `redis`
    --> publish/src/lib.rs:1079:5
     |
1079 |     redis: Option<redis::aio::ConnectionManager>,
     |     ^^^^^ help: if this is intentional, prefix it with an underscore: `_redis`

warning: unused variable: `config`
    --> publish/src/lib.rs:1080:5
     |
1080 |     config: &janitor::config::Config,
     |     ^^^^^^ help: if this is intentional, prefix it with an underscore: `_config`

warning: unused variable: `publish_worker`
    --> publish/src/lib.rs:1081:5
     |
1081 |     publish_worker: &crate::PublishWorker,
     |     ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_publish_worker`

warning: unused variable: `vcs_managers`
    --> publish/src/lib.rs:1082:5
     |
1082 |     vcs_managers: &HashMap<VcsType, Box<dyn VcsManager>>,
     |     ^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_vcs_managers`

warning: unused variable: `bucket_rate_limiter`
    --> publish/src/lib.rs:1083:5
     |
1083 |     bucket_rate_limiter: &Mutex<Box<dyn crate::rate_limiter::RateLimiter>>,
     |     ^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_bucket_rate_limiter`

warning: unused variable: `run`
    --> publish/src/lib.rs:1084:5
     |
1084 |     run: &janitor::state::Run,
     |     ^^^ help: if this is intentional, prefix it with an underscore: `_run`

warning: unused variable: `rate_limit_bucket`
    --> publish/src/lib.rs:1085:5
     |
1085 |     rate_limit_bucket: &str,
     |     ^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_rate_limit_bucket`

warning: unused variable: `unpublished_branches`
    --> publish/src/lib.rs:1086:5
     |
1086 |     unpublished_branches: &[crate::state::UnpublishedBranch],
     |     ^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_unpublished_branches`

warning: unused variable: `command`
    --> publish/src/lib.rs:1087:5
     |
1087 |     command: &str,
     |     ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_command`

warning: unused variable: `push_limit`
    --> publish/src/lib.rs:1088:5
     |
1088 |     push_limit: Option<usize>,
     |     ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_push_limit`

warning: unused variable: `require_binary_diff`
    --> publish/src/lib.rs:1089:5
     |
1089 |     require_binary_diff: bool,
     |     ^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_require_binary_diff`

warning: `janitor-differ` (lib) generated 1 warning (run `cargo fix --lib -p janitor-differ` to apply 1 suggestion)
warning: unused variable: `possible_transports`
   --> publish/src/state.rs:151:5
    |
151 |     possible_transports: Option<&mut Vec<Transport>>,
    |     ^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_possible_transports`

warning: unused variable: `retry_after`
   --> publish/src/web.rs:394:58
    |
394 |             Err(crate::CheckMpError::BranchRateLimited { retry_after }) => {
    |                                                          ^^^^^^^^^^^ help: try ignoring the field: `retry_after: _`

warning: unused import: `crate::rate_limiter::RateLimiter`
 --> publish/src/web.rs:1:5
  |
1 | use crate::rate_limiter::RateLimiter;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused variable: `bucket`
   --> publish/src/rate_limiter.rs:102:28
    |
102 |     fn get_max_open(&self, bucket: &str) -> Option<usize> {
    |                            ^^^^^^ help: if this is intentional, prefix it with an underscore: `_bucket`

warning: unused variable: `retry_after`
   --> publish/src/lib.rs:880:47
    |
880 |             CheckMpError::BranchRateLimited { retry_after } => write!(f, "Branch is rate limited"),
    |                                               ^^^^^^^^^^^ help: try ignoring the field: `retry_after: _`

warning: `janitor` (lib) generated 5 warnings
warning: type `ProposalInfo` is more private than the item `ProposalInfoManager::get_proposal_info`
  --> publish/src/proposal_info.rs:51:5
   |
51 | /     pub async fn get_proposal_info(
52 | |         &self,
53 | |         url: &url::Url,
54 | |     ) -> Result<Option<ProposalInfo>, sqlx::Error> {
   | |__________________________________________________^ method `ProposalInfoManager::get_proposal_info` is reachable at visibility `pub`
   |
note: but type `ProposalInfo` is only usable at visibility `pub(self)`
  --> publish/src/proposal_info.rs:10:1
   |
10 | struct ProposalInfo {
   | ^^^^^^^^^^^^^^^^^^^
   = note: `#[warn(private_interfaces)]` on by default

warning: function `run_worker_process` is never used
   --> publish/src/lib.rs:369:10
    |
369 | async fn run_worker_process(
    |          ^^^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` on by default

warning: method `publish_one` is never used
   --> publish/src/lib.rs:453:14
    |
420 | impl PublishWorker {
    | ------------------ method in this implementation
...
453 |     async fn publish_one(
    |              ^^^^^^^^^^^

warning: fields `can_be_merged`, `status`, `revision`, `target_branch_url`, `rate_limit_bucket`, and `codebase` are never read
  --> publish/src/proposal_info.rs:11:5
   |
10 | struct ProposalInfo {
   |        ------------ fields in this struct
11 |     can_be_merged: Option<bool>,
   |     ^^^^^^^^^^^^^
12 |     status: String,
   |     ^^^^^^
13 |     revision: RevisionId,
   |     ^^^^^^^^
14 |     target_branch_url: Option<String>,
   |     ^^^^^^^^^^^^^^^^^
15 |     rate_limit_bucket: Option<String>,
   |     ^^^^^^^^^^^^^^^^^
16 |     codebase: Option<String>,
   |     ^^^^^^^^
   |
   = note: `ProposalInfo` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis

warning: field `redis` is never read
  --> publish/src/proposal_info.rs:22:5
   |
20 | pub struct ProposalInfoManager {
   |            ------------------- field in this struct
21 |     conn: PgPool,
22 |     redis: Option<redis::aio::ConnectionManager>,
   |     ^^^^^

warning: method `update_proposal_info` is never used
   --> publish/src/proposal_info.rs:110:14
    |
25  | impl ProposalInfoManager {
    | ------------------------ method in this implementation
...
110 |     async fn update_proposal_info(
    |              ^^^^^^^^^^^^^^^^^^^^

warning: function `store_publish` is never used
 --> publish/src/state.rs:7:10
  |
7 | async fn store_publish(
  |          ^^^^^^^^^^^^^

warning: function `already_published` is never used
  --> publish/src/state.rs:83:10
   |
83 | async fn already_published(
   |          ^^^^^^^^^^^^^^^^^

warning: function `get_open_merge_proposal` is never used
  --> publish/src/state.rs:96:10
   |
96 | async fn get_open_merge_proposal(
   |          ^^^^^^^^^^^^^^^^^^^^^^^

warning: function `check_last_published` is never used
   --> publish/src/state.rs:129:10
    |
129 | async fn check_last_published(
    |          ^^^^^^^^^^^^^^^^^^^^

warning: function `guess_codebase_from_branch_url` is never used
   --> publish/src/state.rs:148:10
    |
148 | async fn guess_codebase_from_branch_url(
    |          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: multiple fields are never read
   --> publish/src/state.rs:218:5
    |
217 | pub struct MergeProposalRun {
    |            ---------------- fields in this struct
218 |     id: String,
    |     ^^
219 |     campaign: String,
    |     ^^^^^^^^
220 |     branch_url: String,
    |     ^^^^^^^^^^
221 |     command: String,
    |     ^^^^^^^
222 |     value: i64,
    |     ^^^^^
223 |     role: String,
    |     ^^^^
224 |     remote_branch_name: String,
    |     ^^^^^^^^^^^^^^^^^^
225 |     revision: RevisionId,
    |     ^^^^^^^^
226 |     codebase: String,
    |     ^^^^^^^^
227 |     change_set: String,
    |     ^^^^^^^^^^

warning: function `get_merge_proposal_run` is never used
   --> publish/src/state.rs:230:10
    |
230 | async fn get_merge_proposal_run(
    |          ^^^^^^^^^^^^^^^^^^^^^^

warning: function `get_last_effective_run` is never used
   --> publish/src/state.rs:260:10
    |
260 | async fn get_last_effective_run(
    |          ^^^^^^^^^^^^^^^^^^^^^^

warning: field `id` is never read
   --> publish/src/web.rs:590:9
    |
589 |     struct RunDetails {
    |            ---------- field in this struct
590 |         id: String,
    |         ^^

warning: `janitor-publish` (lib) generated 55 warnings (run `cargo fix --lib -p janitor-publish` to apply 5 suggestions)
       Dirty differ-py v0.0.0 (/home/jelmer/src/janitor/differ-py): dependency info changed
   Compiling differ-py v0.0.0 (/home/jelmer/src/janitor/differ-py)
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name differ_py --edition=2021 differ-py/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type cdylib --emit=dep-info,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="extension-module"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("extension-module"))' -C metadata=5e9be610169f1da6 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-8f48b542fabe92c2.rlib --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rlib --extern janitor_differ=/home/jelmer/src/janitor/target/debug/deps/libjanitor_differ-c40baa0ef9d4bf60.rlib --extern janitor_publish=/home/jelmer/src/janitor/target/debug/deps/libjanitor_publish-8dce75e71ae37b8d.rlib --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rlib --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-d43e1a0d42aa1c27.rlib --extern pyo3_async_runtimes=/home/jelmer/src/janitor/target/debug/deps/libpyo3_async_runtimes-0dab235484cc69b2.rlib --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-0eb92a087d2df449.rlib --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-ccb2e8ac0e11814a.rlib --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 25.71s
Copying rust artifact from target/debug/libdiffer_py.so to py/janitor/_differ.cpython-313-x86_64-linux-gnu.so
cargo rustc --lib --message-format=json-render-diagnostics --manifest-path publish-py/Cargo.toml -v --features extension-module pyo3/extension-module --crate-type cdylib --
       Fresh unicode-ident v1.0.18
       Fresh cfg-if v1.0.0
       Fresh autocfg v1.4.0
       Fresh memchr v2.7.4
       Fresh once_cell v1.21.0
       Fresh value-bag v1.10.0
       Fresh pin-project-lite v0.2.16
       Fresh futures-core v0.3.31
       Fresh bitflags v2.9.0
       Fresh futures-io v0.3.31
       Fresh itoa v1.0.15
       Fresh stable_deref_trait v1.2.0
       Fresh scopeguard v1.2.0
       Fresh bytes v1.10.1
       Fresh regex-syntax v0.8.5
       Fresh litemap v0.7.5
       Fresh writeable v0.5.5
       Fresh foldhash v0.1.4
       Fresh equivalent v1.0.2
       Fresh shlex v1.3.0
       Fresh fastrand v2.3.0
       Fresh allocator-api2 v0.2.21
       Fresh icu_locid_transform_data v1.5.0
       Fresh pin-utils v0.1.0
       Fresh icu_properties_data v1.5.0
       Fresh ryu v1.0.20
       Fresh utf8_iter v1.0.4
       Fresh proc-macro2 v1.0.94
       Fresh tracing-core v0.1.33
       Fresh hashbrown v0.15.2
       Fresh cc v1.2.16
       Fresh icu_normalizer_data v1.5.0
       Fresh utf16_iter v1.0.5
       Fresh write16 v1.0.0
       Fresh percent-encoding v2.3.1
       Fresh atomic-waker v1.1.2
       Fresh futures-task v0.3.31
       Fresh pkg-config v0.3.32
       Fresh linux-raw-sys v0.4.15
       Fresh vcpkg v0.2.15
       Fresh parking v2.2.1
       Fresh log v0.4.27
       Fresh version_check v0.9.5
       Fresh quote v1.0.39
       Fresh libc v0.2.170
       Fresh crossbeam-utils v0.8.21
       Fresh indexmap v2.8.0
       Fresh iana-time-zone v0.1.61
       Fresh foreign-types-shared v0.1.1
       Fresh openssl-probe v0.1.6
       Fresh futures-lite v2.6.0
       Fresh aho-corasick v1.1.3
       Fresh subtle v2.6.1
       Fresh home v0.5.11
       Fresh event-listener v2.5.3
       Fresh async-task v4.7.1
       Fresh bitflags v1.3.2
       Fresh heck v0.5.0
       Fresh piper v0.2.4
       Fresh syn v2.0.100
       Fresh slab v0.4.9
       Fresh lock_api v0.4.12
       Fresh rustix v0.38.44
       Fresh concurrent-queue v2.5.0
       Fresh getrandom v0.2.15
       Fresh zerocopy v0.8.23
       Fresh foreign-types v0.3.2
       Fresh typenum v1.18.0
       Fresh signal-hook-registry v1.4.2
       Fresh socket2 v0.5.8
       Fresh regex-automata v0.4.9
       Fresh mio v1.0.3
       Fresh linux-raw-sys v0.9.2
       Fresh waker-fn v1.2.0
       Fresh fastrand v1.9.0
       Fresh cpufeatures v0.2.17
       Fresh linux-raw-sys v0.3.8
       Fresh socket2 v0.4.10
       Fresh async-lock v2.8.0
       Fresh serde_derive v1.0.219
       Fresh synstructure v0.13.1
       Fresh zerovec-derive v0.10.3
       Fresh displaydoc v0.2.5
       Fresh tracing-attributes v0.1.28
       Fresh icu_provider_macros v1.5.0
       Fresh futures-macro v0.3.31
       Fresh thiserror-impl v2.0.12
       Fresh ppv-lite86 v0.2.21
       Fresh openssl-macros v0.1.1
       Fresh openssl-sys v0.9.107
       Fresh target-lexicon v0.12.16
       Fresh event-listener v5.4.0
       Fresh rand_core v0.6.4
       Fresh generic-array v0.14.7
       Fresh tokio-macros v2.5.0
       Fresh regex v1.11.1
       Fresh rustix v1.0.2
       Fresh async-executor v1.13.1
       Fresh futures-lite v1.13.0
       Fresh async-channel v1.9.0
       Fresh time-core v0.1.3
       Fresh powerfmt v0.2.0
       Fresh fnv v1.0.7
       Fresh serde v1.0.219
       Fresh zerofrom-derive v0.1.6
       Fresh yoke-derive v0.7.5
       Fresh thiserror v2.0.12
       Fresh openssl v0.10.72
       Fresh event-listener-strategy v0.5.3
       Fresh rand_chacha v0.3.1
       Fresh crypto-common v0.1.6
       Fresh block-buffer v0.10.4
       Fresh io-lifetimes v1.0.11
       Fresh tracing v0.1.41
       Fresh num-conv v0.1.0
       Fresh http v1.2.0
       Fresh crc-catalog v2.4.0
       Fresh tinyvec_macros v0.1.1
       Fresh unicase v2.8.1
       Fresh crossbeam-queue v0.3.12
       Fresh hashlink v0.10.0
       Fresh futures-sink v0.3.31
       Fresh thiserror-impl v1.0.69
       Fresh form_urlencoded v1.2.1
       Fresh mime v0.3.17
       Fresh zerofrom v0.1.6
       Fresh serde_json v1.0.140
       Fresh digest v0.10.7
       Fresh async-lock v3.4.0
       Fresh async-channel v2.3.1
       Fresh smallvec v1.14.0
       Fresh rustix v0.37.28
       Fresh time-macros v0.2.20
       Fresh deranged v0.3.11
       Fresh tinyvec v1.9.0
       Fresh crc v3.2.1
       Fresh either v1.15.0
       Fresh num-traits v0.2.19
       Fresh http-body v1.0.1
       Fresh polling v3.7.4
       Fresh unicode-bidi v0.3.18
       Fresh hex v0.4.3
       Fresh unicode-properties v0.1.3
       Fresh futures-util v0.3.31
       Fresh native-tls v0.2.14
       Fresh tower-service v0.3.3
       Fresh yoke v0.7.5
       Fresh pyo3-build-config v0.22.6
       Fresh blocking v1.6.1
       Fresh sha2 v0.10.8
       Fresh time v0.3.39
       Fresh parking_lot_core v0.9.10
       Fresh unicode-normalization v0.1.24
       Fresh hmac v0.12.1
       Fresh memoffset v0.9.1
       Fresh chrono v0.4.40
       Fresh thiserror v1.0.69
       Fresh md-5 v0.10.6
       Fresh async-io v2.4.0
       Fresh byteorder v1.5.0
       Fresh dotenvy v0.15.7
       Fresh indoc v2.0.6
       Fresh unindent v0.2.4
       Fresh try-lock v0.2.5
       Fresh whoami v1.5.2
       Fresh httparse v1.10.1
       Fresh rand v0.8.5
       Fresh futures-channel v0.3.31
       Fresh polling v2.8.0
       Fresh kv-log-macro v1.0.7
       Fresh httpdate v1.0.3
       Fresh rustc-hash v1.1.0
       Fresh hashbrown v0.14.5
       Fresh siphasher v1.0.1
       Fresh zerovec v0.10.4
       Fresh parking_lot v0.12.3
       Fresh hkdf v0.12.4
       Fresh stringprep v0.1.5
       Fresh async-global-executor v2.4.1
       Fresh want v0.3.1
       Fresh base64 v0.22.1
       Fresh countme v3.0.1
       Fresh text-size v1.1.1
       Fresh async-io v1.13.0
       Fresh http-body-util v0.1.3
       Fresh sync_wrapper v1.0.2
       Fresh tower-layer v0.3.3
       Fresh lazy-regex-proc_macros v3.4.1
       Fresh atoi v2.0.0
       Fresh serde_urlencoded v0.7.1
       Fresh utf8parse v0.2.2
       Fresh same-file v1.0.6
       Fresh rustls-pki-types v1.11.0
       Fresh deb822-derive v0.2.0
       Fresh encoding_rs v0.8.35
       Fresh adler2 v2.0.0
       Fresh is_terminal_polyfill v1.70.1
       Fresh tinystr v0.7.6
       Fresh icu_collections v1.5.0
   Compiling tokio v1.44.2
       Fresh futures-intrusive v0.5.0
       Fresh async-std v1.13.1
       Fresh rowan v0.16.1
       Fresh lazy-regex v3.4.1
       Fresh rustls-pemfile v2.2.0
       Fresh walkdir v2.5.0
       Fresh anstyle-parse v0.2.6
       Fresh anstyle-query v1.1.2
       Fresh ipnet v2.11.0
       Fresh anstyle v1.0.10
       Fresh unicode-width v0.2.0
       Fresh colorchoice v1.0.3
       Fresh miniz_oxide v0.8.5
       Fresh phf_generator v0.11.3
       Fresh num-integer v0.1.46
       Fresh lazy_static v1.5.0
       Fresh protobuf-support v3.7.2
       Fresh which v4.4.2
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.44.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(loom)' --check-cfg 'cfg(mio_unsupported_force_poll_poll)' --check-cfg 'cfg(tokio_allow_from_blocking_fd)' --check-cfg 'cfg(tokio_internal_mt_counters)' --check-cfg 'cfg(tokio_no_parking_lot)' --check-cfg 'cfg(tokio_no_tuning_tests)' --check-cfg 'cfg(tokio_taskdump)' --check-cfg 'cfg(tokio_unstable)' --cfg 'feature="bytes"' --cfg 'feature="default"' --cfg 'feature="fs"' --cfg 'feature="full"' --cfg 'feature="io-std"' --cfg 'feature="io-util"' --cfg 'feature="libc"' --cfg 'feature="macros"' --cfg 'feature="mio"' --cfg 'feature="net"' --cfg 'feature="parking_lot"' --cfg 'feature="process"' --cfg 'feature="rt"' --cfg 'feature="rt-multi-thread"' --cfg 'feature="signal"' --cfg 'feature="signal-hook-registry"' --cfg 'feature="socket2"' --cfg 'feature="sync"' --cfg 'feature="time"' --cfg 'feature="tokio-macros"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("bytes", "default", "fs", "full", "io-std", "io-util", "libc", "macros", "mio", "net", "parking_lot", "process", "rt", "rt-multi-thread", "signal", "signal-hook-registry", "socket2", "sync", "test-util", "time", "tokio-macros", "tracing", "windows-sys"))' -C metadata=a6a9517afda8bb67 -C extra-filename=-f832f4eaea3b145b --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-5457d7c65e6ff7c6.rmeta --extern mio=/home/jelmer/src/janitor/target/debug/deps/libmio-ea8f193d550eeb3d.rmeta --extern parking_lot=/home/jelmer/src/janitor/target/debug/deps/libparking_lot-e2b62b5be6a25198.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern signal_hook_registry=/home/jelmer/src/janitor/target/debug/deps/libsignal_hook_registry-0134a4b6a31e32fc.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-631c96f5856b2ef3.rmeta --extern tokio_macros=/home/jelmer/src/janitor/target/debug/deps/libtokio_macros-6d6e842210b98dca.so --cap-lints allow --cfg tokio_unstable`
       Fresh icu_locid v1.5.0
       Fresh pyo3-macros-backend v0.22.6
       Fresh pyo3-ffi v0.22.6
       Fresh getrandom v0.3.1
       Fresh anstream v0.6.18
       Fresh syn v1.0.109
       Fresh num-bigint v0.4.6
       Fresh phf_codegen v0.11.3
       Fresh parse-zoneinfo v0.3.1
       Fresh async-trait v0.1.88
       Fresh inotify-sys v0.1.5
       Fresh gimli v0.31.1
       Fresh ucd-trie v0.1.7
       Fresh strsim v0.11.1
       Fresh smawk v0.3.2
       Fresh unicode-linebreak v0.1.5
       Fresh unicode-xid v0.2.6
       Fresh clap_lex v0.7.4
       Fresh clap_derive v4.5.32
       Fresh crossbeam-epoch v0.9.18
       Fresh mio v0.8.11
       Fresh crossbeam-channel v0.5.15
       Fresh filetime v0.2.25
       Fresh icu_provider v1.5.0
       Fresh pyo3-macros v0.22.6
       Fresh anyhow v1.0.97
       Fresh clap_builder v4.5.36
       Fresh textwrap v0.16.2
       Fresh chrono-tz-build v0.3.0
       Fresh object v0.36.7
       Fresh addr2line v0.24.2
       Fresh pest v2.7.15
       Fresh protobuf v3.7.2
       Fresh inotify v0.9.6
       Fresh synstructure v0.12.6
       Fresh bstr v1.11.3
       Fresh dtor-proc-macro v0.0.5
       Fresh unic-common v0.9.0
       Fresh base64ct v1.7.1
       Fresh unic-char-range v0.9.0
       Fresh rustc-demangle v0.1.24
       Fresh crossbeam-deque v0.8.6
       Fresh tempfile v3.19.0
       Fresh rowan v0.15.16
       Fresh phf_shared v0.11.3
       Fresh itertools v0.13.0
       Fresh atty v0.2.14
       Fresh csv-core v0.1.12
       Fresh untrusted v0.9.0
       Fresh icu_locid_transform v1.5.0
       Fresh pyo3 v0.22.6
       Fresh failure_derive v0.1.8
       Fresh pest_meta v2.7.15
       Fresh clap v4.5.36
       Fresh protobuf v2.28.0
       Fresh backtrace v0.3.74
       Fresh pem-rfc7468 v0.7.0
       Fresh globset v0.4.16
       Fresh notify v6.1.1
       Fresh unic-ucd-version v0.9.0
       Fresh unic-char-property v0.9.0
       Fresh libm v0.2.11
       Fresh dtor v0.0.5
       Fresh const-oid v0.9.6
       Fresh urlencoding v2.1.3
       Fresh zeroize v1.8.1
       Fresh difflib v0.4.0
       Fresh minimal-lexical v0.2.1
       Fresh termcolor v1.4.1
       Fresh quick-error v1.2.3
       Fresh ctor-proc-macro v0.0.5
       Fresh csv v1.3.1
       Fresh phf v0.11.3
       Fresh protobuf-parse v3.7.2
       Fresh simple_asn1 v0.6.3
       Fresh rand_core v0.9.3
       Fresh icu_properties v1.5.1
       Fresh deb822-lossless v0.2.4
       Fresh ctor v0.4.1
       Fresh failure v0.1.8
       Fresh ring v0.17.13
       Fresh humansize v2.1.3
       Fresh der v0.7.9
       Fresh pest_generator v2.7.15
       Fresh nom v7.1.3
       Fresh unic-ucd-segment v0.9.0
       Fresh ignore v0.4.23
       Fresh dirty-tracker v0.3.0
       Fresh humantime v1.3.0
       Fresh protobuf-codegen v2.28.0
       Fresh pyo3-filelike v0.4.1
       Fresh protoc v2.28.0
       Fresh patchkit v0.2.1
       Fresh version-ranges v0.1.1
       Fresh futures-executor v0.3.31
       Fresh env_filter v0.1.3
       Fresh pem v3.0.5
       Fresh crc32fast v1.4.2
       Fresh unsafe-libyaml v0.2.11
       Fresh deunicode v1.6.0
       Fresh bit-vec v0.8.0
       Fresh winnow v0.7.3
       Fresh jiff v0.2.4
       Fresh maplit v1.0.2
       Fresh icu_normalizer v1.5.0
       Fresh toml_datetime v0.6.8
       Fresh unscanny v0.1.0
       Fresh globwalk v0.9.1
       Fresh futures v0.3.31
       Fresh distro-info v0.4.0
       Fresh env_logger v0.7.1
       Fresh chrono-tz v0.9.0
       Fresh bit-set v0.8.0
       Fresh protoc-rust v2.28.0
       Fresh jsonwebtoken v9.3.1
       Fresh pest_derive v2.7.15
       Fresh env_logger v0.11.7
       Fresh askama_parser v0.2.1
       Fresh serde_yaml v0.9.34+deprecated
       Fresh flate2 v1.1.0
       Fresh semver v1.0.26
       Fresh slug v0.1.6
       Fresh spki v0.7.3
       Fresh unic-segment v0.9.0
       Fresh mime_guess v2.0.5
       Fresh rand_chacha v0.9.0
       Fresh protobuf-codegen v3.7.2
       Fresh merge3 v0.2.0
       Fresh google-cloud-token v0.1.2
       Fresh makefile-lossless v0.1.7
       Fresh sha1 v0.10.6
       Fresh basic-toml v0.1.10
       Fresh idna_adapter v1.2.0
       Fresh toml_edit v0.22.24
       Fresh pep440_rs v0.7.3
       Fresh async-stream-impl v0.3.6
       Fresh boxcar v0.2.10
       Fresh configparser v3.1.0
       Fresh arc-swap v1.7.1
       Fresh humantime v2.1.0
       Fresh rustc-hash v2.1.1
       Fresh base64 v0.21.7
       Fresh askama_derive v0.12.5
       Fresh pretty_env_logger v0.4.0
       Fresh rand v0.9.0
       Fresh pkcs8 v0.10.2
       Fresh fancy-regex v0.14.0
       Fresh tera v1.20.0
       Fresh toml v0.5.11
       Fresh backon v1.4.0
       Fresh xdg v2.5.2
       Fresh askama_escape v0.10.3
       Fresh sha1_smol v1.0.1
       Fresh inventory v0.3.20
       Fresh serde_path_to_error v0.1.17
       Fresh self_cell v1.1.0
       Fresh matchit v0.8.4
       Fresh memo-map v0.3.3
       Fresh idna v1.0.3
       Fresh async-stream v0.3.6
       Fresh rustversion v1.0.20
       Fresh env_logger v0.9.3
       Fresh askama v0.12.1
       Dirty janitor v0.1.0 (/home/jelmer/src/janitor): the precalculated components changed
   Compiling janitor v0.1.0 (/home/jelmer/src/janitor)
       Fresh minijinja v2.9.0
       Fresh pyo3-log v0.11.0
       Fresh url v2.5.4
       Fresh stackdriver_logger v0.8.2
       Fresh axum-core v0.5.2
     Running `/home/jelmer/src/janitor/target/debug/build/janitor-5ee49d7480f4d8ea/build-script-build`
       Fresh sqlx-core v0.8.3
       Fresh dep3 v0.1.28
       Fresh pep508_rs v0.9.2
       Fresh sqlx-postgres v0.8.3
       Fresh sqlx-macros-core v0.8.3
       Fresh sqlx-macros v0.8.3
       Fresh sqlx v0.8.3
       Fresh debversion v0.4.4
       Fresh debian-control v0.1.41
       Fresh debian-changelog v0.2.0
       Fresh debian-copyright v0.1.27
   Compiling breezyshim v0.1.227
       Fresh buildlog-consultant v0.1.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name breezyshim --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/breezyshim-0.1.227/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="auto-initialize"' --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="dirty-tracker"' --cfg 'feature="sqlx"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("auto-initialize", "debian", "default", "dirty-tracker", "sqlx"))' -C metadata=5c13ba886387c468 -C extra-filename=-b29f2e914ed4f8f7 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern ctor=/home/jelmer/src/janitor/target/debug/deps/libctor-72258acac2d0b9ee.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-83daf3b37151d30c.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-9f2af6068106d2fb.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-40db761eabe70986.rmeta --extern difflib=/home/jelmer/src/janitor/target/debug/deps/libdifflib-20593172ebc6932e.rmeta --extern dirty_tracker=/home/jelmer/src/janitor/target/debug/deps/libdirty_tracker-15c2a709d36ea33e.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern patchkit=/home/jelmer/src/janitor/target/debug/deps/libpatchkit-045f9ba36150b582.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern pyo3_filelike=/home/jelmer/src/janitor/target/debug/deps/libpyo3_filelike-bc0667b965758a04.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-02615dedbab651d7.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-34844f972d674a37.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling tokio-util v0.7.14
   Compiling tokio-native-tls v0.3.1
   Compiling tower v0.5.2
   Compiling async-compression v0.4.23
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio_util --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-util-0.7.14/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(loom)' --check-cfg 'cfg(mio_unsupported_force_poll_poll)' --check-cfg 'cfg(tokio_allow_from_blocking_fd)' --check-cfg 'cfg(tokio_internal_mt_counters)' --check-cfg 'cfg(tokio_no_parking_lot)' --check-cfg 'cfg(tokio_no_tuning_tests)' --check-cfg 'cfg(tokio_taskdump)' --check-cfg 'cfg(tokio_unstable)' --cfg 'feature="codec"' --cfg 'feature="default"' --cfg 'feature="io"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__docs_rs", "codec", "compat", "default", "full", "futures-io", "futures-util", "hashbrown", "io", "io-util", "net", "rt", "slab", "time", "tracing"))' -C metadata=77ae6d30a55bb343 -C extra-filename=-fcc120ec1ab729cc --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_sink=/home/jelmer/src/janitor/target/debug/deps/libfutures_sink-0f1aae5d0426fde7.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio_native_tls --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-native-tls-0.3.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("vendored"))' -C metadata=ea5c4b90739c43b8 -C extra-filename=-961a615e45412395 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-41e5bdd64bc7cd9e.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tower --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tower-0.5.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="__common"' --cfg 'feature="futures-core"' --cfg 'feature="futures-util"' --cfg 'feature="log"' --cfg 'feature="make"' --cfg 'feature="pin-project-lite"' --cfg 'feature="sync_wrapper"' --cfg 'feature="timeout"' --cfg 'feature="tokio"' --cfg 'feature="tracing"' --cfg 'feature="util"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__common", "balance", "buffer", "discover", "filter", "full", "futures-core", "futures-util", "hdrhistogram", "hedge", "indexmap", "limit", "load", "load-shed", "log", "make", "pin-project-lite", "ready-cache", "reconnect", "retry", "slab", "spawn-ready", "steer", "sync_wrapper", "timeout", "tokio", "tokio-stream", "tokio-util", "tracing", "util"))' -C metadata=f693fae12b32bbc0 -C extra-filename=-83699d9bcea6aa21 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tower_layer=/home/jelmer/src/janitor/target/debug/deps/libtower_layer-fff117c1c28e545a.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name async_compression --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/async-compression-0.4.23/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="flate2"' --cfg 'feature="gzip"' --cfg 'feature="tokio"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("all", "all-algorithms", "all-implementations", "brotli", "bzip2", "deflate", "deflate64", "flate2", "futures-io", "gzip", "libzstd", "lz4", "lzma", "tokio", "xz", "xz2", "zlib", "zstd", "zstd-safe", "zstdmt"))' -C metadata=cde3c7de7a9cf557 -C extra-filename=-aa22db53322b438f --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling h2 v0.4.8
   Compiling combine v4.6.7
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name h2 --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/h2-0.4.8/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("stream", "unstable"))' -C metadata=e7e493406a6219a9 -C extra-filename=-7274ba537e8716e3 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern atomic_waker=/home/jelmer/src/janitor/target/debug/deps/libatomic_waker-21f0b624b8878034.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern fnv=/home/jelmer/src/janitor/target/debug/deps/libfnv-135eca59eff18b18.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_sink=/home/jelmer/src/janitor/target/debug/deps/libfutures_sink-0f1aae5d0426fde7.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-9a486ccb6575c0f1.rmeta --extern slab=/home/jelmer/src/janitor/target/debug/deps/libslab-58feeb60e58ddd09.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-fcc120ec1ab729cc.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name combine --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/combine-4.6.7/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="alloc"' --cfg 'feature="bytes"' --cfg 'feature="futures-core-03"' --cfg 'feature="pin-project-lite"' --cfg 'feature="std"' --cfg 'feature="tokio"' --cfg 'feature="tokio-dep"' --cfg 'feature="tokio-util"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alloc", "bytes", "bytes_05", "default", "futures-03", "futures-core-03", "futures-io-03", "mp4", "pin-project", "pin-project-lite", "regex", "std", "tokio", "tokio-02", "tokio-02-dep", "tokio-03", "tokio-03-dep", "tokio-dep", "tokio-util"))' -C metadata=6d79a804bbb50398 -C extra-filename=-044310925aab587b --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_core_03=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern tokio_dep=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-fcc120ec1ab729cc.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper v1.6.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-1.6.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(hyper_unstable_tracing)' --check-cfg 'cfg(hyper_unstable_ffi)' --cfg 'feature="client"' --cfg 'feature="default"' --cfg 'feature="http1"' --cfg 'feature="http2"' --cfg 'feature="server"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("capi", "client", "default", "ffi", "full", "http1", "http2", "nightly", "server", "tracing"))' -C metadata=0669d06602ea7adc -C extra-filename=-d43df0c63ef8fd16 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern h2=/home/jelmer/src/janitor/target/debug/deps/libh2-7274ba537e8716e3.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern httparse=/home/jelmer/src/janitor/target/debug/deps/libhttparse-de9e4dfe0f78db23.rmeta --extern httpdate=/home/jelmer/src/janitor/target/debug/deps/libhttpdate-66eb51e4c8d24adc.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-59505d94661b74c2.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern want=/home/jelmer/src/janitor/target/debug/deps/libwant-676b1650d2642fde.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling redis v0.27.6
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name redis --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/redis-0.27.6/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="acl"' --cfg 'feature="aio"' --cfg 'feature="async-trait"' --cfg 'feature="backon"' --cfg 'feature="bytes"' --cfg 'feature="connection-manager"' --cfg 'feature="default"' --cfg 'feature="futures"' --cfg 'feature="futures-util"' --cfg 'feature="geospatial"' --cfg 'feature="json"' --cfg 'feature="keep-alive"' --cfg 'feature="pin-project-lite"' --cfg 'feature="script"' --cfg 'feature="serde"' --cfg 'feature="serde_json"' --cfg 'feature="sha1_smol"' --cfg 'feature="socket2"' --cfg 'feature="streams"' --cfg 'feature="tokio"' --cfg 'feature="tokio-comp"' --cfg 'feature="tokio-util"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("acl", "ahash", "aio", "async-native-tls", "async-std", "async-std-comp", "async-std-native-tls-comp", "async-std-rustls-comp", "async-std-tls-comp", "async-trait", "backon", "bigdecimal", "bytes", "cluster", "cluster-async", "connection-manager", "crc16", "default", "disable-client-setinfo", "futures", "futures-rustls", "futures-util", "geospatial", "hashbrown", "json", "keep-alive", "log", "native-tls", "num-bigint", "pin-project-lite", "r2d2", "rand", "rust_decimal", "rustls", "rustls-native-certs", "rustls-pemfile", "rustls-pki-types", "script", "sentinel", "serde", "serde_json", "sha1_smol", "socket2", "streams", "tcp_nodelay", "tls", "tls-native-tls", "tls-rustls", "tls-rustls-insecure", "tls-rustls-webpki-roots", "tokio", "tokio-comp", "tokio-native-tls", "tokio-native-tls-comp", "tokio-rustls", "tokio-rustls-comp", "tokio-util", "uuid", "webpki-roots"))' -C metadata=bde3bc6b09205b96 -C extra-filename=-dbeeb5370ac9efe7 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern arc_swap=/home/jelmer/src/janitor/target/debug/deps/libarc_swap-bd5aa4a1e22f9e5d.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern backon=/home/jelmer/src/janitor/target/debug/deps/libbackon-9baa21c6e034bb14.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern combine=/home/jelmer/src/janitor/target/debug/deps/libcombine-044310925aab587b.rmeta --extern futures=/home/jelmer/src/janitor/target/debug/deps/libfutures-463765cbcde647a0.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern itertools=/home/jelmer/src/janitor/target/debug/deps/libitertools-a8fb045921351b76.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern num_bigint=/home/jelmer/src/janitor/target/debug/deps/libnum_bigint-7a7a2dd2f34962d4.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern ryu=/home/jelmer/src/janitor/target/debug/deps/libryu-245a84a5a509b3a3.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha1_smol=/home/jelmer/src/janitor/target/debug/deps/libsha1_smol-03061bab6b3928dd.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-631c96f5856b2ef3.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-fcc120ec1ab729cc.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper-util v0.1.10
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper_util --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-util-0.1.10/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="client"' --cfg 'feature="client-legacy"' --cfg 'feature="default"' --cfg 'feature="http1"' --cfg 'feature="http2"' --cfg 'feature="server"' --cfg 'feature="service"' --cfg 'feature="tokio"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__internal_happy_eyeballs_tests", "client", "client-legacy", "default", "full", "http1", "http2", "server", "server-auto", "server-graceful", "service", "tokio"))' -C metadata=061132e86d8d2496 -C extra-filename=-03ef3b1eee5c3f17 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-d43df0c63ef8fd16.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-631c96f5856b2ef3.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper-tls v0.6.0
   Compiling axum v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper_tls --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-tls-0.6.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alpn", "vendored"))' -C metadata=4495244228bc147a -C extra-filename=-0f210aecd6c4132c --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-d43df0c63ef8fd16.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-03ef3b1eee5c3f17.rmeta --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-41e5bdd64bc7cd9e.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tokio_native_tls=/home/jelmer/src/janitor/target/debug/deps/libtokio_native_tls-961a615e45412395.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name axum --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/axum-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::verbose_file_reads' '--warn=clippy::unused_self' --forbid=unsafe_code --warn=unreachable_pub '--warn=clippy::unnested_or_patterns' '--warn=clippy::uninlined_format_args' '--allow=clippy::type_complexity' '--warn=clippy::todo' '--warn=clippy::suboptimal_flops' '--warn=clippy::str_to_string' '--warn=clippy::rest_pat_in_fully_bound_structs' '--warn=clippy::option_option' '--warn=clippy::needless_continue' '--warn=clippy::needless_borrow' --warn=missing_docs --warn=missing_debug_implementations '--warn=clippy::mem_forget' '--warn=clippy::match_wildcard_for_single_variants' '--warn=clippy::match_on_vec_items' '--warn=clippy::macro_use_imports' '--warn=clippy::lossy_float_literal' '--warn=clippy::linkedlist' '--warn=clippy::inefficient_to_string' '--warn=clippy::imprecise_flops' '--warn=clippy::if_let_mutex' '--warn=clippy::fn_params_excessive_bools' '--warn=clippy::filter_map_next' '--warn=clippy::exit' '--warn=clippy::enum_glob_use' '--warn=clippy::empty_enum' '--warn=clippy::dbg_macro' '--warn=clippy::await_holding_lock' --cfg 'feature="default"' --cfg 'feature="form"' --cfg 'feature="http1"' --cfg 'feature="json"' --cfg 'feature="matched-path"' --cfg 'feature="original-uri"' --cfg 'feature="query"' --cfg 'feature="tokio"' --cfg 'feature="tower-log"' --cfg 'feature="tracing"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__private", "__private_docs", "default", "form", "http1", "http2", "json", "macros", "matched-path", "multipart", "original-uri", "query", "tokio", "tower-log", "tracing", "ws"))' -C metadata=23dfc0580e95c202 -C extra-filename=-e3e38764afeed268 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern axum_core=/home/jelmer/src/janitor/target/debug/deps/libaxum_core-c7084f1580c648d3.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern form_urlencoded=/home/jelmer/src/janitor/target/debug/deps/libform_urlencoded-072fa14f50efb53e.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-d43df0c63ef8fd16.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-03ef3b1eee5c3f17.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern matchit=/home/jelmer/src/janitor/target/debug/deps/libmatchit-0d71d298d63a0df3.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern mime=/home/jelmer/src/janitor/target/debug/deps/libmime-c74000db2aaf511f.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern rustversion=/home/jelmer/src/janitor/target/debug/deps/librustversion-494b2fd16358ba50.so --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_path_to_error=/home/jelmer/src/janitor/target/debug/deps/libserde_path_to_error-72e72ae8986ce543.rmeta --extern serde_urlencoded=/home/jelmer/src/janitor/target/debug/deps/libserde_urlencoded-e0562f68e1545f98.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tower=/home/jelmer/src/janitor/target/debug/deps/libtower-83699d9bcea6aa21.rmeta --extern tower_layer=/home/jelmer/src/janitor/target/debug/deps/libtower_layer-fff117c1c28e545a.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling reqwest v0.12.15
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name reqwest --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/reqwest-0.12.15/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(reqwest_unstable)' --cfg 'feature="__tls"' --cfg 'feature="blocking"' --cfg 'feature="charset"' --cfg 'feature="default"' --cfg 'feature="default-tls"' --cfg 'feature="h2"' --cfg 'feature="http2"' --cfg 'feature="json"' --cfg 'feature="macos-system-configuration"' --cfg 'feature="multipart"' --cfg 'feature="stream"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__rustls", "__rustls-ring", "__tls", "blocking", "brotli", "charset", "cookies", "default", "default-tls", "deflate", "gzip", "h2", "hickory-dns", "http2", "http3", "json", "macos-system-configuration", "multipart", "native-tls", "native-tls-alpn", "native-tls-vendored", "rustls-tls", "rustls-tls-manual-roots", "rustls-tls-manual-roots-no-provider", "rustls-tls-native-roots", "rustls-tls-native-roots-no-provider", "rustls-tls-no-provider", "rustls-tls-webpki-roots", "rustls-tls-webpki-roots-no-provider", "socks", "stream", "trust-dns", "zstd"))' -C metadata=214f713daf12cf65 -C extra-filename=-3d88c4f45db349e4 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-b1a4de7ed4da6927.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern encoding_rs=/home/jelmer/src/janitor/target/debug/deps/libencoding_rs-58c41e4932181cd7.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern h2=/home/jelmer/src/janitor/target/debug/deps/libh2-7274ba537e8716e3.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-d43df0c63ef8fd16.rmeta --extern hyper_tls=/home/jelmer/src/janitor/target/debug/deps/libhyper_tls-0f210aecd6c4132c.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-03ef3b1eee5c3f17.rmeta --extern ipnet=/home/jelmer/src/janitor/target/debug/deps/libipnet-5873e4e1530bf49f.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern mime=/home/jelmer/src/janitor/target/debug/deps/libmime-c74000db2aaf511f.rmeta --extern mime_guess=/home/jelmer/src/janitor/target/debug/deps/libmime_guess-7ee1813410f2722d.rmeta --extern native_tls_crate=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-41e5bdd64bc7cd9e.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern rustls_pemfile=/home/jelmer/src/janitor/target/debug/deps/librustls_pemfile-68bb2d10b5046659.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_urlencoded=/home/jelmer/src/janitor/target/debug/deps/libserde_urlencoded-e0562f68e1545f98.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tokio_native_tls=/home/jelmer/src/janitor/target/debug/deps/libtokio_native_tls-961a615e45412395.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-fcc120ec1ab729cc.rmeta --extern tower=/home/jelmer/src/janitor/target/debug/deps/libtower-83699d9bcea6aa21.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling google-cloud-metadata v0.5.1
   Compiling debian-analyzer v0.158.25
   Compiling reqwest-middleware v0.3.3
   Compiling prometheus v0.14.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name google_cloud_metadata --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/google-cloud-metadata-0.5.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=1126fa47bb1a9e77 -C extra-filename=-94607ca5b2db5b4b --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-3d88c4f45db349e4.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_analyzer --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-analyzer-0.158.25/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="merge3"' --cfg 'feature="python"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "default", "merge3", "python", "svp", "udd"))' -C metadata=8250b95d55a2623a -C extra-filename=-b445aec2656cd77f --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-b29f2e914ed4f8f7.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern configparser=/home/jelmer/src/janitor/target/debug/deps/libconfigparser-aaa60c0f437f3031.rmeta --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-85e845bd7914badc.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-83daf3b37151d30c.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-9f2af6068106d2fb.rmeta --extern debian_copyright=/home/jelmer/src/janitor/target/debug/deps/libdebian_copyright-48cae804b27ee72b.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-40db761eabe70986.rmeta --extern dep3=/home/jelmer/src/janitor/target/debug/deps/libdep3-cef4e33c810b0205.rmeta --extern difflib=/home/jelmer/src/janitor/target/debug/deps/libdifflib-20593172ebc6932e.rmeta --extern distro_info=/home/jelmer/src/janitor/target/debug/deps/libdistro_info-e2f22ea1e25dba0a.rmeta --extern filetime=/home/jelmer/src/janitor/target/debug/deps/libfiletime-74a11c28d8d6d9ef.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern makefile_lossless=/home/jelmer/src/janitor/target/debug/deps/libmakefile_lossless-f7c86ff44e7ff685.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern merge3=/home/jelmer/src/janitor/target/debug/deps/libmerge3-1c24ac3badc9ba5b.rmeta --extern patchkit=/home/jelmer/src/janitor/target/debug/deps/libpatchkit-045f9ba36150b582.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-3d88c4f45db349e4.rmeta --extern semver=/home/jelmer/src/janitor/target/debug/deps/libsemver-11fd74ac82b27f0f.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha1=/home/jelmer/src/janitor/target/debug/deps/libsha1-666ba0d12790bffa.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-34844f972d674a37.rmeta --extern toml_edit=/home/jelmer/src/janitor/target/debug/deps/libtoml_edit-51c35483d814c85f.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name reqwest_middleware --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/reqwest-middleware-0.3.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="json"' --cfg 'feature="multipart"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("charset", "http2", "json", "multipart", "rustls-tls"))' -C metadata=8d73ff730fbc07d1 -C extra-filename=-ea2cb764199d07c2 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern anyhow=/home/jelmer/src/janitor/target/debug/deps/libanyhow-904a89ff6dd1202e.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-3d88c4f45db349e4.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name prometheus --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/prometheus-0.14.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="protobuf"' --cfg 'feature="reqwest"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "gen", "libc", "nightly", "process", "procfs", "protobuf", "protobuf-codegen", "push", "reqwest"))' -C metadata=10f84236994e9c69 -C extra-filename=-42f5854af277805a --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern cfg_if=/home/jelmer/src/janitor/target/debug/deps/libcfg_if-c30e1cd0415bac84.rmeta --extern fnv=/home/jelmer/src/janitor/target/debug/deps/libfnv-135eca59eff18b18.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern parking_lot=/home/jelmer/src/janitor/target/debug/deps/libparking_lot-e2b62b5be6a25198.rmeta --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-b1e3dc89f529da22.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-3d88c4f45db349e4.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling google-cloud-auth v0.17.2
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name google_cloud_auth --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/google-cloud-auth-0.17.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="default-tls"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "default-tls", "external-account", "hex", "hickory-dns", "hmac", "path-clean", "percent-encoding", "rustls-tls", "sha2", "url"))' -C metadata=e352e333fd04aa31 -C extra-filename=-f584341388cf36d2 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-41324bb9dba3dad5.rmeta --extern google_cloud_metadata=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_metadata-94607ca5b2db5b4b.rmeta --extern google_cloud_token=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_token-6145b7093dd432ee.rmeta --extern home=/home/jelmer/src/janitor/target/debug/deps/libhome-fc8b76e8d1b394eb.rmeta --extern jsonwebtoken=/home/jelmer/src/janitor/target/debug/deps/libjsonwebtoken-85a75c0ee2ebcc8a.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-3d88c4f45db349e4.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-93a16e2f2de5f6ed.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern urlencoding=/home/jelmer/src/janitor/target/debug/deps/liburlencoding-0ba1b8b89d728edb.rmeta --cap-lints allow --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
   Compiling google-cloud-storage v0.22.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name google_cloud_storage --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/google-cloud-storage-0.22.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="auth"' --cfg 'feature="default"' --cfg 'feature="default-tls"' --cfg 'feature="google-cloud-auth"' --cfg 'feature="google-cloud-metadata"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("auth", "default", "default-tls", "external-account", "google-cloud-auth", "google-cloud-metadata", "hickory-dns", "rustls-tls", "trace"))' -C metadata=23972e948aaf1241 -C extra-filename=-b26a5df90c839847 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern anyhow=/home/jelmer/src/janitor/target/debug/deps/libanyhow-904a89ff6dd1202e.rmeta --extern async_stream=/home/jelmer/src/janitor/target/debug/deps/libasync_stream-f0f1e6ef812a7b6c.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-41324bb9dba3dad5.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern google_cloud_auth=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_auth-f584341388cf36d2.rmeta --extern google_cloud_metadata=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_metadata-94607ca5b2db5b4b.rmeta --extern google_cloud_token=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_token-6145b7093dd432ee.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pkcs8=/home/jelmer/src/janitor/target/debug/deps/libpkcs8-ef54810b56a401a1.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-3d88c4f45db349e4.rmeta --extern reqwest_middleware=/home/jelmer/src/janitor/target/debug/deps/libreqwest_middleware-ea2cb764199d07c2.rmeta --extern ring=/home/jelmer/src/janitor/target/debug/deps/libring-1446534c300ac753.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-93a16e2f2de5f6ed.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
   Compiling silver-platter v0.5.48
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name silver_platter --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/silver-platter-0.5.48/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="detect-update-changelog"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default", "detect-update-changelog", "gpg", "last-attempt-db", "pyo3"))' -C metadata=f3761f5847d328a3 -C extra-filename=-baa79bc0b06fb5ec --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-b29f2e914ed4f8f7.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-b445aec2656cd77f.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-83daf3b37151d30c.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-9f2af6068106d2fb.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-40db761eabe70986.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-808eaebb057ce37e.rmeta --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-5457d7c65e6ff7c6.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-6571000d5ff98899.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-3d88c4f45db349e4.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_yaml=/home/jelmer/src/janitor/target/debug/deps/libserde_yaml-d19b32863dd48a61.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-34844f972d674a37.rmeta --extern tera=/home/jelmer/src/janitor/target/debug/deps/libtera-39fe85b2b15ed66c.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --extern xdg=/home/jelmer/src/janitor/target/debug/deps/libxdg-23f110d46d019c5b.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling rslock v0.5.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name rslock --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rslock-0.5.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="tokio-comp"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("async-std-comp", "default", "tokio-comp"))' -C metadata=0f4308c28ea022bc -C extra-filename=-fbe515718b218184 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern futures=/home/jelmer/src/janitor/target/debug/deps/libfutures-463765cbcde647a0.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-c535a7a8ba116747.rmeta --extern redis=/home/jelmer/src/janitor/target/debug/deps/libredis-dbeeb5370ac9efe7.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor --edition=2021 src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="gcp"' --cfg 'feature="gcs"' --cfg 'feature="stackdriver_logger"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("debian", "default", "gcp", "gcs", "stackdriver_logger"))' -C metadata=be13852805d3b4a9 -C extra-filename=-3fd12e3995dc0052 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e239d113cd99a25a.rmeta --extern async_compression=/home/jelmer/src/janitor/target/debug/deps/libasync_compression-aa22db53322b438f.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-b29f2e914ed4f8f7.rmeta --extern buildlog_consultant=/home/jelmer/src/janitor/target/debug/deps/libbuildlog_consultant-87cd661227323dc7.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-9f2af6068106d2fb.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-40db761eabe70986.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-808eaebb057ce37e.rmeta --extern fancy_regex=/home/jelmer/src/janitor/target/debug/deps/libfancy_regex-0688edb11485e39a.rmeta --extern filetime=/home/jelmer/src/janitor/target/debug/deps/libfiletime-74a11c28d8d6d9ef.rmeta --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern futures=/home/jelmer/src/janitor/target/debug/deps/libfutures-463765cbcde647a0.rmeta --extern google_cloud_auth=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_auth-f584341388cf36d2.rmeta --extern google_cloud_storage=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_storage-b26a5df90c839847.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-42f5854af277805a.rmeta --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-b1e3dc89f529da22.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-3d88c4f45db349e4.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-baa79bc0b06fb5ec.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-02615dedbab651d7.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-b1311c083eaa4498.rmeta --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-928485792788ad80.rmeta --extern stackdriver_logger=/home/jelmer/src/janitor/target/debug/deps/libstackdriver_logger-ca9fc4b835919b4a.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-34844f972d674a37.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
warning: function `reprocess_run_logs` is never used
 --> src/reprocess_logs.rs:8:10
  |
8 | async fn reprocess_run_logs(
  |          ^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(dead_code)]` on by default

warning: field `branch_url` is never read
  --> src/schedule.rs:32:5
   |
30 | pub struct ScheduleRequest {
   |            --------------- field in this struct
31 |     codebase: String,
32 |     branch_url: String,
   |     ^^^^^^^^^^

warning: function `has_cotenants` is never used
  --> src/state.rs:80:10
   |
80 | async fn has_cotenants(
   |          ^^^^^^^^^^^^^

warning: field `name` is never read
  --> src/state.rs:87:13
   |
86 |     struct Codebase {
   |            -------- field in this struct
87 |         pub name: String,
   |             ^^^^
   |
   = note: `Codebase` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: function `iter_publishable_suites` is never used
   --> src/state.rs:113:10
    |
113 | async fn iter_publishable_suites(
    |          ^^^^^^^^^^^^^^^^^^^^^^^

   Compiling janitor-publish v0.0.0 (/home/jelmer/src/janitor/publish)
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor_publish --edition=2021 publish/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=cd4381fe664574f0 -C extra-filename=-2b810b3188a47ecf --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-e3e38764afeed268.rmeta --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-b29f2e914ed4f8f7.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-83daf3b37151d30c.rmeta --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-3fd12e3995dc0052.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern minijinja=/home/jelmer/src/janitor/target/debug/deps/libminijinja-6c444bb921e91f39.rmeta --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-42f5854af277805a.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern redis=/home/jelmer/src/janitor/target/debug/deps/libredis-dbeeb5370ac9efe7.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-3d88c4f45db349e4.rmeta --extern rslock=/home/jelmer/src/janitor/target/debug/deps/librslock-fbe515718b218184.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-baa79bc0b06fb5ec.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-02615dedbab651d7.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
warning: unused import: `Mode`
 --> publish/src/proposal_info.rs:3:45
  |
3 | use janitor::publish::{MergeProposalStatus, Mode};
  |                                             ^^^^
  |
  = note: `#[warn(unused_imports)]` on by default

warning: unused import: `url::Url`
 --> publish/src/proposal_info.rs:6:5
  |
6 | use url::Url;
  |     ^^^^^^^^

warning: unused import: `breezyshim::forge::Forge`
 --> publish/src/web.rs:9:5
  |
9 | use breezyshim::forge::Forge;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused imports: `VcsManager` and `VcsType`
  --> publish/src/web.rs:11:20
   |
11 | use janitor::vcs::{VcsManager, VcsType};
   |                    ^^^^^^^^^^  ^^^^^^^

warning: unused import: `Mutex`
  --> publish/src/web.rs:14:22
   |
14 | use std::sync::{Arc, Mutex};
   |                      ^^^^^

warning: use of deprecated function `std::env::home_dir`: This function's behavior may be unexpected on Windows. Consider using a crate from crates.io instead.
   --> publish/src/web.rs:210:29
    |
210 |     let ssh_dir = std::env::home_dir().unwrap().join(".ssh");
    |                             ^^^^^^^^
    |
    = note: `#[warn(deprecated)]` on by default

warning: unused variable: `state`
   --> publish/src/lib.rs:725:5
    |
725 |     state: Arc<AppState>,
    |     ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`
    |
    = note: `#[warn(unused_variables)]` on by default

warning: unused variable: `interval`
   --> publish/src/lib.rs:726:5
    |
726 |     interval: chrono::Duration,
    |     ^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_interval`

warning: unused variable: `auto_publish`
   --> publish/src/lib.rs:727:5
    |
727 |     auto_publish: bool,
    |     ^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_auto_publish`

warning: unused variable: `state`
   --> publish/src/lib.rs:739:36
    |
739 | pub async fn publish_pending_ready(state: Arc<AppState>) -> Result<(), PublishError> {
    |                                    ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `state`
   --> publish/src/lib.rs:785:31
    |
785 | pub async fn listen_to_runner(state: Arc<AppState>) {
    |                               ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `conn`
   --> publish/src/lib.rs:890:5
    |
890 |     conn: &sqlx::PgPool,
    |     ^^^^ help: if this is intentional, prefix it with an underscore: `_conn`

warning: unused variable: `redis`
   --> publish/src/lib.rs:891:5
    |
891 |     redis: Option<redis::aio::ConnectionManager>,
    |     ^^^^^ help: if this is intentional, prefix it with an underscore: `_redis`

warning: unused variable: `config`
   --> publish/src/lib.rs:892:5
    |
892 |     config: &janitor::config::Config,
    |     ^^^^^^ help: if this is intentional, prefix it with an underscore: `_config`

warning: unused variable: `publish_worker`
   --> publish/src/lib.rs:893:5
    |
893 |     publish_worker: &crate::PublishWorker,
    |     ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_publish_worker`

warning: unused variable: `mp`
   --> publish/src/lib.rs:894:5
    |
894 |     mp: &breezyshim::forge::MergeProposal,
    |     ^^ help: if this is intentional, prefix it with an underscore: `_mp`

warning: unused variable: `status`
   --> publish/src/lib.rs:895:5
    |
895 |     status: breezyshim::forge::MergeProposalStatus,
    |     ^^^^^^ help: if this is intentional, prefix it with an underscore: `_status`

warning: unused variable: `vcs_managers`
   --> publish/src/lib.rs:896:5
    |
896 |     vcs_managers: &HashMap<VcsType, Box<dyn VcsManager>>,
    |     ^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_vcs_managers`

warning: unused variable: `bucket_rate_limiter`
   --> publish/src/lib.rs:897:5
    |
897 |     bucket_rate_limiter: &Mutex<Box<dyn crate::rate_limiter::RateLimiter>>,
    |     ^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_bucket_rate_limiter`

warning: unused variable: `check_only`
   --> publish/src/lib.rs:898:5
    |
898 |     check_only: bool,
    |     ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_check_only`

warning: unused variable: `mps_per_bucket`
   --> publish/src/lib.rs:899:5
    |
899 |     mps_per_bucket: Option<
    |     ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_mps_per_bucket`

warning: unused variable: `possible_transports`
   --> publish/src/lib.rs:902:5
    |
902 |     possible_transports: Option<&mut Vec<breezyshim::transport::Transport>>,
    |     ^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_possible_transports`

warning: unused variable: `bucket`
    --> publish/src/lib.rs:1062:14
     |
1062 |         for (bucket, count) in mps_per_bucket
     |              ^^^^^^ help: if this is intentional, prefix it with an underscore: `_bucket`

warning: unused variable: `conn`
    --> publish/src/lib.rs:1078:5
     |
1078 |     conn: &sqlx::PgPool,
     |     ^^^^ help: if this is intentional, prefix it with an underscore: `_conn`

warning: unused variable: `redis`
    --> publish/src/lib.rs:1079:5
     |
1079 |     redis: Option<redis::aio::ConnectionManager>,
     |     ^^^^^ help: if this is intentional, prefix it with an underscore: `_redis`

warning: unused variable: `config`
    --> publish/src/lib.rs:1080:5
     |
1080 |     config: &janitor::config::Config,
     |     ^^^^^^ help: if this is intentional, prefix it with an underscore: `_config`

warning: unused variable: `publish_worker`
    --> publish/src/lib.rs:1081:5
     |
1081 |     publish_worker: &crate::PublishWorker,
     |     ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_publish_worker`

warning: unused variable: `vcs_managers`
    --> publish/src/lib.rs:1082:5
     |
1082 |     vcs_managers: &HashMap<VcsType, Box<dyn VcsManager>>,
     |     ^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_vcs_managers`

warning: unused variable: `bucket_rate_limiter`
    --> publish/src/lib.rs:1083:5
     |
1083 |     bucket_rate_limiter: &Mutex<Box<dyn crate::rate_limiter::RateLimiter>>,
     |     ^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_bucket_rate_limiter`

warning: unused variable: `run`
    --> publish/src/lib.rs:1084:5
     |
1084 |     run: &janitor::state::Run,
     |     ^^^ help: if this is intentional, prefix it with an underscore: `_run`

warning: unused variable: `rate_limit_bucket`
    --> publish/src/lib.rs:1085:5
     |
1085 |     rate_limit_bucket: &str,
     |     ^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_rate_limit_bucket`

warning: unused variable: `unpublished_branches`
    --> publish/src/lib.rs:1086:5
     |
1086 |     unpublished_branches: &[crate::state::UnpublishedBranch],
     |     ^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_unpublished_branches`

warning: unused variable: `command`
    --> publish/src/lib.rs:1087:5
     |
1087 |     command: &str,
     |     ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_command`

warning: unused variable: `push_limit`
    --> publish/src/lib.rs:1088:5
     |
1088 |     push_limit: Option<usize>,
     |     ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_push_limit`

warning: unused variable: `require_binary_diff`
    --> publish/src/lib.rs:1089:5
     |
1089 |     require_binary_diff: bool,
     |     ^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_require_binary_diff`

warning: unused variable: `possible_transports`
   --> publish/src/state.rs:151:5
    |
151 |     possible_transports: Option<&mut Vec<Transport>>,
    |     ^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_possible_transports`

warning: unused variable: `retry_after`
   --> publish/src/web.rs:394:58
    |
394 |             Err(crate::CheckMpError::BranchRateLimited { retry_after }) => {
    |                                                          ^^^^^^^^^^^ help: try ignoring the field: `retry_after: _`

warning: unused import: `crate::rate_limiter::RateLimiter`
 --> publish/src/web.rs:1:5
  |
1 | use crate::rate_limiter::RateLimiter;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused variable: `bucket`
   --> publish/src/rate_limiter.rs:102:28
    |
102 |     fn get_max_open(&self, bucket: &str) -> Option<usize> {
    |                            ^^^^^^ help: if this is intentional, prefix it with an underscore: `_bucket`

warning: unused variable: `retry_after`
   --> publish/src/lib.rs:880:47
    |
880 |             CheckMpError::BranchRateLimited { retry_after } => write!(f, "Branch is rate limited"),
    |                                               ^^^^^^^^^^^ help: try ignoring the field: `retry_after: _`

warning: type `ProposalInfo` is more private than the item `ProposalInfoManager::get_proposal_info`
  --> publish/src/proposal_info.rs:51:5
   |
51 | /     pub async fn get_proposal_info(
52 | |         &self,
53 | |         url: &url::Url,
54 | |     ) -> Result<Option<ProposalInfo>, sqlx::Error> {
   | |__________________________________________________^ method `ProposalInfoManager::get_proposal_info` is reachable at visibility `pub`
   |
note: but type `ProposalInfo` is only usable at visibility `pub(self)`
  --> publish/src/proposal_info.rs:10:1
   |
10 | struct ProposalInfo {
   | ^^^^^^^^^^^^^^^^^^^
   = note: `#[warn(private_interfaces)]` on by default

warning: function `run_worker_process` is never used
   --> publish/src/lib.rs:369:10
    |
369 | async fn run_worker_process(
    |          ^^^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` on by default

warning: method `publish_one` is never used
   --> publish/src/lib.rs:453:14
    |
420 | impl PublishWorker {
    | ------------------ method in this implementation
...
453 |     async fn publish_one(
    |              ^^^^^^^^^^^

warning: fields `can_be_merged`, `status`, `revision`, `target_branch_url`, `rate_limit_bucket`, and `codebase` are never read
  --> publish/src/proposal_info.rs:11:5
   |
10 | struct ProposalInfo {
   |        ------------ fields in this struct
11 |     can_be_merged: Option<bool>,
   |     ^^^^^^^^^^^^^
12 |     status: String,
   |     ^^^^^^
13 |     revision: RevisionId,
   |     ^^^^^^^^
14 |     target_branch_url: Option<String>,
   |     ^^^^^^^^^^^^^^^^^
15 |     rate_limit_bucket: Option<String>,
   |     ^^^^^^^^^^^^^^^^^
16 |     codebase: Option<String>,
   |     ^^^^^^^^
   |
   = note: `ProposalInfo` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis

warning: field `redis` is never read
  --> publish/src/proposal_info.rs:22:5
   |
20 | pub struct ProposalInfoManager {
   |            ------------------- field in this struct
21 |     conn: PgPool,
22 |     redis: Option<redis::aio::ConnectionManager>,
   |     ^^^^^

warning: method `update_proposal_info` is never used
   --> publish/src/proposal_info.rs:110:14
    |
25  | impl ProposalInfoManager {
    | ------------------------ method in this implementation
...
110 |     async fn update_proposal_info(
    |              ^^^^^^^^^^^^^^^^^^^^

warning: function `store_publish` is never used
 --> publish/src/state.rs:7:10
  |
7 | async fn store_publish(
  |          ^^^^^^^^^^^^^

warning: function `already_published` is never used
  --> publish/src/state.rs:83:10
   |
83 | async fn already_published(
   |          ^^^^^^^^^^^^^^^^^

warning: function `get_open_merge_proposal` is never used
  --> publish/src/state.rs:96:10
   |
96 | async fn get_open_merge_proposal(
   |          ^^^^^^^^^^^^^^^^^^^^^^^

warning: function `check_last_published` is never used
   --> publish/src/state.rs:129:10
    |
129 | async fn check_last_published(
    |          ^^^^^^^^^^^^^^^^^^^^

warning: function `guess_codebase_from_branch_url` is never used
   --> publish/src/state.rs:148:10
    |
148 | async fn guess_codebase_from_branch_url(
    |          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: multiple fields are never read
   --> publish/src/state.rs:218:5
    |
217 | pub struct MergeProposalRun {
    |            ---------------- fields in this struct
218 |     id: String,
    |     ^^
219 |     campaign: String,
    |     ^^^^^^^^
220 |     branch_url: String,
    |     ^^^^^^^^^^
221 |     command: String,
    |     ^^^^^^^
222 |     value: i64,
    |     ^^^^^
223 |     role: String,
    |     ^^^^
224 |     remote_branch_name: String,
    |     ^^^^^^^^^^^^^^^^^^
225 |     revision: RevisionId,
    |     ^^^^^^^^
226 |     codebase: String,
    |     ^^^^^^^^
227 |     change_set: String,
    |     ^^^^^^^^^^

warning: function `get_merge_proposal_run` is never used
   --> publish/src/state.rs:230:10
    |
230 | async fn get_merge_proposal_run(
    |          ^^^^^^^^^^^^^^^^^^^^^^

warning: function `get_last_effective_run` is never used
   --> publish/src/state.rs:260:10
    |
260 | async fn get_last_effective_run(
    |          ^^^^^^^^^^^^^^^^^^^^^^

warning: field `id` is never read
   --> publish/src/web.rs:590:9
    |
589 |     struct RunDetails {
    |            ---------- field in this struct
590 |         id: String,
    |         ^^

warning: `janitor` (lib) generated 5 warnings
warning: `janitor-publish` (lib) generated 55 warnings (run `cargo fix --lib -p janitor-publish` to apply 5 suggestions)
       Dirty publish-py v0.0.0 (/home/jelmer/src/janitor/publish-py): dependency info changed
   Compiling publish-py v0.0.0 (/home/jelmer/src/janitor/publish-py)
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name publish_py --edition=2021 publish-py/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type cdylib --emit=dep-info,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="extension-module"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("extension-module"))' -C metadata=c282dbdb74070f4d --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-b29f2e914ed4f8f7.rlib --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rlib --extern janitor_publish=/home/jelmer/src/janitor/target/debug/deps/libjanitor_publish-2b810b3188a47ecf.rlib --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rlib --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rlib --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-f11d63d32a114e1a.rlib --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-baa79bc0b06fb5ec.rlib --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rlib --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
warning: unused import: `std::str::FromStr`
   --> publish-py/src/lib.rs:129:13
    |
129 |         use std::str::FromStr;
    |             ^^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(unused_imports)]` on by default

warning: `publish-py` (lib) generated 1 warning (run `cargo fix --lib -p publish-py` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 18.13s
Copying rust artifact from target/debug/libpublish_py.so to py/janitor/_publish.cpython-313-x86_64-linux-gnu.so
cargo rustc --lib --message-format=json-render-diagnostics --manifest-path runner-py/Cargo.toml -v --features extension-module pyo3/extension-module --crate-type cdylib --
       Fresh unicode-ident v1.0.18
       Fresh cfg-if v1.0.0
       Fresh autocfg v1.4.0
       Fresh memchr v2.7.4
       Fresh once_cell v1.21.0
       Fresh value-bag v1.10.0
       Fresh pin-project-lite v0.2.16
       Fresh futures-core v0.3.31
       Fresh bitflags v2.9.0
       Fresh futures-io v0.3.31
       Fresh itoa v1.0.15
       Fresh stable_deref_trait v1.2.0
       Fresh scopeguard v1.2.0
       Fresh fastrand v2.3.0
       Fresh bytes v1.10.1
       Fresh regex-syntax v0.8.5
       Fresh equivalent v1.0.2
       Fresh writeable v0.5.5
       Fresh foldhash v0.1.4
       Fresh litemap v0.7.5
       Fresh allocator-api2 v0.2.21
       Fresh parking v2.2.1
       Fresh linux-raw-sys v0.4.15
       Fresh icu_locid_transform_data v1.5.0
       Fresh pin-utils v0.1.0
       Fresh icu_properties_data v1.5.0
       Fresh proc-macro2 v1.0.94
       Fresh tracing-core v0.1.33
       Fresh hashbrown v0.15.2
       Fresh log v0.4.27
       Fresh icu_normalizer_data v1.5.0
       Fresh shlex v1.3.0
       Fresh write16 v1.0.0
       Fresh utf16_iter v1.0.5
       Fresh percent-encoding v2.3.1
       Fresh utf8_iter v1.0.4
       Fresh atomic-waker v1.1.2
       Fresh pkg-config v0.3.32
       Fresh ryu v1.0.20
       Fresh futures-task v0.3.31
       Fresh vcpkg v0.2.15
       Fresh iana-time-zone v0.1.61
       Fresh version_check v0.9.5
       Fresh futures-lite v2.6.0
       Fresh quote v1.0.39
       Fresh libc v0.2.170
       Fresh crossbeam-utils v0.8.21
       Fresh rustix v0.38.44
       Fresh indexmap v2.8.0
       Fresh cc v1.2.16
       Fresh foreign-types-shared v0.1.1
       Fresh aho-corasick v1.1.3
       Fresh subtle v2.6.1
       Fresh heck v0.5.0
       Fresh event-listener v2.5.3
       Fresh openssl-probe v0.1.6
       Fresh bitflags v1.3.2
       Fresh async-task v4.7.1
       Fresh piper v0.2.4
       Fresh linux-raw-sys v0.9.2
       Fresh syn v2.0.100
       Fresh slab v0.4.9
       Fresh lock_api v0.4.12
       Fresh concurrent-queue v2.5.0
       Fresh zerocopy v0.8.23
       Fresh typenum v1.18.0
       Fresh target-lexicon v0.12.16
       Fresh getrandom v0.2.15
       Fresh foreign-types v0.3.2
       Fresh regex-automata v0.4.9
       Fresh home v0.5.11
       Fresh waker-fn v1.2.0
       Fresh cpufeatures v0.2.17
       Fresh linux-raw-sys v0.3.8
       Fresh fnv v1.0.7
       Fresh fastrand v1.9.0
       Fresh async-lock v2.8.0
       Fresh socket2 v0.4.10
       Fresh socket2 v0.5.8
       Fresh serde_derive v1.0.219
       Fresh synstructure v0.13.1
       Fresh zerovec-derive v0.10.3
       Fresh displaydoc v0.2.5
       Fresh tracing-attributes v0.1.28
       Fresh icu_provider_macros v1.5.0
       Fresh ppv-lite86 v0.2.21
       Fresh event-listener v5.4.0
       Fresh generic-array v0.14.7
       Fresh rand_core v0.6.4
       Fresh openssl-macros v0.1.1
       Fresh thiserror-impl v2.0.12
       Fresh futures-macro v0.3.31
       Fresh rustix v1.0.2
       Fresh async-executor v1.13.1
       Fresh io-lifetimes v1.0.11
       Fresh futures-lite v1.13.0
       Fresh async-channel v1.9.0
       Fresh tokio-macros v2.5.0
       Fresh regex v1.11.1
       Fresh mio v1.0.3
       Fresh signal-hook-registry v1.4.2
       Fresh tinyvec_macros v0.1.1
       Fresh crc-catalog v2.4.0
       Fresh crossbeam-queue v0.3.12
       Fresh serde v1.0.219
       Fresh zerofrom-derive v0.1.6
       Fresh yoke-derive v0.7.5
       Fresh event-listener-strategy v0.5.3
       Fresh rand_chacha v0.3.1
       Fresh crypto-common v0.1.6
       Fresh openssl-sys v0.9.107
       Fresh block-buffer v0.10.4
       Fresh thiserror v2.0.12
       Fresh tracing v0.1.41
       Fresh rustix v0.37.28
       Fresh tinyvec v1.9.0
       Fresh num-traits v0.2.19
       Fresh crc v3.2.1
       Fresh hashlink v0.10.0
       Fresh futures-sink v0.3.31
       Fresh form_urlencoded v1.2.1
       Fresh unicode-bidi v0.3.18
       Fresh unicode-properties v0.1.3
       Fresh dotenvy v0.15.7
       Fresh hex v0.4.3
       Fresh byteorder v1.5.0
       Fresh indoc v2.0.6
       Fresh zerofrom v0.1.6
       Fresh pyo3-build-config v0.22.6
       Fresh serde_json v1.0.140
       Fresh digest v0.10.7
       Fresh openssl v0.10.72
       Fresh async-lock v3.4.0
       Fresh smallvec v1.14.0
       Fresh async-channel v2.3.1
       Fresh unicode-normalization v0.1.24
       Fresh either v1.15.0
       Fresh polling v3.7.4
       Fresh futures-util v0.3.31
       Fresh chrono v0.4.40
       Fresh time-core v0.1.3
       Fresh unindent v0.2.4
       Fresh whoami v1.5.2
       Fresh num-conv v0.1.0
       Fresh powerfmt v0.2.0
       Fresh futures-channel v0.3.31
       Fresh http v1.2.0
       Fresh polling v2.8.0
       Fresh kv-log-macro v1.0.7
       Fresh hashbrown v0.14.5
       Fresh yoke v0.7.5
       Fresh blocking v1.6.1
       Fresh parking_lot_core v0.9.10
       Fresh hmac v0.12.1
       Fresh native-tls v0.2.14
       Fresh memoffset v0.9.1
       Fresh md-5 v0.10.6
       Fresh async-io v2.4.0
       Fresh stringprep v0.1.5
       Fresh time-macros v0.2.20
       Fresh deranged v0.3.11
       Fresh text-size v1.1.1
       Fresh siphasher v1.0.1
       Fresh rustc-hash v1.1.0
       Fresh base64 v0.22.1
       Fresh countme v3.0.1
       Fresh async-io v1.13.0
       Fresh sha2 v0.10.8
       Fresh rand v0.8.5
       Fresh strsim v0.11.1
       Fresh http-body v1.0.1
       Fresh lazy-regex-proc_macros v3.4.1
       Fresh atoi v2.0.0
       Fresh zerovec v0.10.4
       Fresh parking_lot v0.12.3
       Fresh hkdf v0.12.4
       Fresh async-global-executor v2.4.1
       Fresh time v0.3.39
       Fresh thiserror-impl v1.0.69
       Fresh rowan v0.16.1
       Fresh lazy-regex v3.4.1
       Fresh try-lock v0.2.5
       Fresh mime v0.3.17
       Fresh tower-service v0.3.3
       Fresh same-file v1.0.6
       Fresh utf8parse v0.2.2
       Fresh deb822-derive v0.2.0
       Fresh colorchoice v1.0.3
       Fresh anstyle-query v1.1.2
       Fresh unicode-width v0.2.0
       Fresh anstyle v1.0.10
       Fresh httpdate v1.0.3
       Fresh adler2 v2.0.0
       Fresh is_terminal_polyfill v1.70.1
       Fresh http-body-util v0.1.3
       Fresh sync_wrapper v1.0.2
       Fresh tinystr v0.7.6
       Fresh icu_collections v1.5.0
       Fresh tokio v1.44.2
       Fresh pyo3-macros-backend v0.22.6
       Fresh pyo3-ffi v0.22.6
       Fresh futures-intrusive v0.5.0
       Fresh async-std v1.13.1
       Fresh thiserror v1.0.69
       Fresh want v0.3.1
       Fresh anstyle-parse v0.2.6
       Fresh walkdir v2.5.0
       Fresh miniz_oxide v0.8.5
       Fresh phf_generator v0.11.3
       Fresh tower-layer v0.3.3
       Fresh lazy_static v1.5.0
       Fresh which v4.4.2
       Fresh parse-zoneinfo v0.3.1
       Fresh inotify-sys v0.1.5
       Fresh unicode-linebreak v0.1.5
       Fresh unicode-xid v0.2.6
       Fresh smawk v0.3.2
       Fresh gimli v0.31.1
       Fresh icu_locid v1.5.0
       Fresh pyo3-macros v0.22.6
       Fresh tokio-util v0.7.14
       Fresh httparse v1.10.1
       Fresh getrandom v0.3.1
       Fresh anstream v0.6.18
       Fresh protobuf-support v3.7.2
       Fresh phf_codegen v0.11.3
       Fresh syn v1.0.109
       Fresh ucd-trie v0.1.7
       Fresh clap_lex v0.7.4
   Compiling tower v0.5.2
       Fresh textwrap v0.16.2
       Fresh addr2line v0.24.2
       Fresh tokio-native-tls v0.3.1
       Fresh inotify v0.9.6
       Fresh serde_urlencoded v0.7.1
       Fresh clap_derive v4.5.32
       Fresh mio v0.8.11
       Fresh crossbeam-channel v0.5.15
       Fresh filetime v0.2.25
       Fresh crossbeam-epoch v0.9.18
       Fresh bstr v1.11.3
       Fresh rustc-demangle v0.1.24
       Fresh rustls-pki-types v1.11.0
       Fresh unic-common v0.9.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tower --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tower-0.5.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="__common"' --cfg 'feature="futures-core"' --cfg 'feature="futures-util"' --cfg 'feature="log"' --cfg 'feature="make"' --cfg 'feature="pin-project-lite"' --cfg 'feature="sync_wrapper"' --cfg 'feature="timeout"' --cfg 'feature="tokio"' --cfg 'feature="tracing"' --cfg 'feature="util"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__common", "balance", "buffer", "discover", "filter", "full", "futures-core", "futures-util", "hdrhistogram", "hedge", "indexmap", "limit", "load", "load-shed", "log", "make", "pin-project-lite", "ready-cache", "reconnect", "retry", "slab", "spawn-ready", "steer", "sync_wrapper", "timeout", "tokio", "tokio-stream", "tokio-util", "tracing", "util"))' -C metadata=1d2da9694b2da6db -C extra-filename=-051ada069bee8653 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-639e19ba3d2b09a4.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tower_layer=/home/jelmer/src/janitor/target/debug/deps/libtower_layer-fff117c1c28e545a.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
       Fresh icu_provider v1.5.0
       Fresh pyo3 v0.22.6
       Fresh h2 v0.4.8
       Fresh chrono-tz-build v0.3.0
       Fresh clap_builder v4.5.36
       Fresh pest v2.7.15
       Fresh synstructure v0.12.6
       Fresh protobuf v3.7.2
       Fresh object v0.36.7
       Fresh dtor-proc-macro v0.0.5
       Fresh unic-char-range v0.9.0
       Fresh unicase v2.8.1
       Fresh rustls-pemfile v2.2.0
       Fresh crossbeam-deque v0.8.6
       Fresh tempfile v3.19.0
       Fresh anyhow v1.0.97
       Fresh notify v6.1.1
       Fresh globset v0.4.16
       Fresh unic-ucd-version v0.9.0
       Fresh phf_shared v0.11.3
       Fresh rowan v0.15.16
       Fresh encoding_rs v0.8.35
       Fresh csv-core v0.1.12
       Fresh ident_case v1.0.1
       Fresh minimal-lexical v0.2.1
       Fresh ctor-proc-macro v0.0.5
       Fresh difflib v0.4.0
       Fresh ipnet v2.11.0
       Fresh icu_locid_transform v1.5.0
   Compiling hyper v1.6.0
       Fresh deb822-lossless v0.2.4
       Fresh dtor v0.0.5
       Fresh pest_meta v2.7.15
       Fresh unic-char-property v0.9.0
       Fresh failure_derive v0.1.8
       Fresh backtrace v0.3.74
       Fresh libm v0.2.11
       Fresh protobuf v2.28.0
       Fresh clap v4.5.36
       Fresh darling_core v0.20.10
       Fresh csv v1.3.1
       Fresh nom v7.1.3
       Fresh pyo3-filelike v0.4.1
       Fresh protobuf-parse v3.7.2
       Fresh phf v0.11.3
       Fresh dirty-tracker v0.3.0
       Fresh ignore v0.4.23
       Fresh rand_core v0.9.3
       Fresh patchkit v0.2.1
       Fresh protoc v2.28.0
       Fresh itertools v0.13.0
       Fresh version-ranges v0.1.1
       Fresh env_filter v0.1.3
       Fresh crc32fast v1.4.2
       Fresh maplit v1.0.2
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-1.6.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(hyper_unstable_tracing)' --check-cfg 'cfg(hyper_unstable_ffi)' --cfg 'feature="client"' --cfg 'feature="default"' --cfg 'feature="http1"' --cfg 'feature="http2"' --cfg 'feature="server"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("capi", "client", "default", "ffi", "full", "http1", "http2", "nightly", "server", "tracing"))' -C metadata=f36b55e573673310 -C extra-filename=-a272ca6ae4bc1d3a --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-639e19ba3d2b09a4.rmeta --extern h2=/home/jelmer/src/janitor/target/debug/deps/libh2-7274ba537e8716e3.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern httparse=/home/jelmer/src/janitor/target/debug/deps/libhttparse-de9e4dfe0f78db23.rmeta --extern httpdate=/home/jelmer/src/janitor/target/debug/deps/libhttpdate-66eb51e4c8d24adc.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-59505d94661b74c2.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern want=/home/jelmer/src/janitor/target/debug/deps/libwant-676b1650d2642fde.rmeta --cap-lints allow --cfg tokio_unstable`
       Fresh icu_properties v1.5.1
       Fresh pest_generator v2.7.15
       Fresh failure v0.1.8
       Fresh protobuf-codegen v2.28.0
       Fresh unic-ucd-segment v0.9.0
       Fresh humansize v2.1.3
       Fresh ctor v0.4.1
       Fresh jiff v0.2.4
       Fresh bit-vec v0.8.0
       Fresh winnow v0.7.3
       Fresh unsafe-libyaml v0.2.11
       Fresh toml_datetime v0.6.8
       Fresh deunicode v1.6.0
       Fresh unscanny v0.1.0
       Fresh merge3 v0.2.0
       Fresh flate2 v1.1.0
       Fresh darling_macro v0.20.10
       Fresh askama_parser v0.2.1
       Fresh globwalk v0.9.1
       Fresh protobuf-codegen v3.7.2
       Fresh rand_chacha v0.9.0
       Fresh makefile-lossless v0.1.7
       Fresh sha1 v0.10.6
       Fresh basic-toml v0.1.10
       Fresh icu_normalizer v1.5.0
       Fresh distro-info v0.4.0
       Fresh mime_guess v2.0.5
       Fresh chrono-tz v0.9.0
       Fresh pep440_rs v0.7.3
       Fresh bit-set v0.8.0
       Fresh pest_derive v2.7.15
       Fresh protoc-rust v2.28.0
       Fresh semver v1.0.26
       Fresh env_logger v0.11.7
       Fresh slug v0.1.6
       Fresh unic-segment v0.9.0
       Fresh serde_yaml v0.9.34+deprecated
       Fresh toml_edit v0.22.24
       Fresh configparser v3.1.0
       Fresh boxcar v0.2.10
       Fresh rustc-hash v2.1.1
       Fresh urlencoding v2.1.3
       Fresh rand v0.9.0
       Fresh darling v0.20.10
       Fresh futures-executor v0.3.31
       Fresh async-trait v0.1.88
       Fresh num-integer v0.1.46
       Fresh askama_escape v0.10.3
       Fresh arc-swap v1.7.1
       Fresh inventory v0.3.20
       Fresh xdg v2.5.2
       Fresh idna_adapter v1.2.0
       Fresh fancy-regex v0.14.0
       Fresh tera v1.20.0
       Fresh askama_derive v0.12.5
       Fresh rustversion v1.0.20
   Compiling janitor v0.1.0 (/home/jelmer/src/janitor)
       Fresh serde_with_macros v3.12.0
       Fresh futures v0.3.31
       Fresh num-bigint v0.4.6
       Fresh async-compression v0.4.23
       Fresh combine v4.6.7
       Fresh serde_path_to_error v0.1.17
       Fresh matchit v0.8.4
       Fresh sha1_smol v1.0.1
       Fresh pyo3-log v0.11.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name build_script_build --edition=2021 build.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no --cfg 'feature="debian"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("debian", "default", "gcp", "gcs", "stackdriver_logger"))' -C metadata=0768975666f8e3de -C extra-filename=-0ecd700503a9d21a --out-dir /home/jelmer/src/janitor/target/debug/build/janitor-0ecd700503a9d21a -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern protobuf_codegen=/home/jelmer/src/janitor/target/debug/deps/libprotobuf_codegen-2fb0a08953e095b2.rlib --extern protoc_rust=/home/jelmer/src/janitor/target/debug/deps/libprotoc_rust-aaf82484d7bd6338.rlib --cfg tokio_unstable`
       Fresh idna v1.0.3
       Fresh axum-core v0.5.2
       Fresh askama v0.12.1
       Fresh serde_with v3.12.0
       Fresh url v2.5.4
   Compiling sqlx-core v0.8.3
       Fresh dep3 v0.1.28
       Fresh pep508_rs v0.9.2
   Compiling redis v0.27.6
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_core --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-core-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="_rt-async-std"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="any"' --cfg 'feature="async-io"' --cfg 'feature="async-std"' --cfg 'feature="chrono"' --cfg 'feature="crc"' --cfg 'feature="default"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="native-tls"' --cfg 'feature="offline"' --cfg 'feature="serde"' --cfg 'feature="serde_json"' --cfg 'feature="sha2"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_tls-native-tls", "_tls-none", "_tls-rustls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "any", "async-io", "async-std", "bigdecimal", "bit-vec", "bstr", "chrono", "crc", "default", "ipnetwork", "json", "mac_address", "migrate", "native-tls", "offline", "regex", "rust_decimal", "rustls", "rustls-native-certs", "rustls-pemfile", "serde", "serde_json", "sha2", "time", "tokio", "tokio-stream", "uuid", "webpki-roots"))' -C metadata=3dd5d1fd5cd30514 -C extra-filename=-952deb4c3052cf2d --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_io=/home/jelmer/src/janitor/target/debug/deps/libasync_io-835a56f561c864c0.rmeta --extern async_std=/home/jelmer/src/janitor/target/debug/deps/libasync_std-220d57f9d1d250bf.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-4eb151582e08ecdb.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern crossbeam_queue=/home/jelmer/src/janitor/target/debug/deps/libcrossbeam_queue-577e4d13a58a6351.rmeta --extern either=/home/jelmer/src/janitor/target/debug/deps/libeither-accfca6c6f5e11c6.rmeta --extern event_listener=/home/jelmer/src/janitor/target/debug/deps/libevent_listener-55331feab369961e.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_intrusive=/home/jelmer/src/janitor/target/debug/deps/libfutures_intrusive-d61fea0a40cf5f80.rmeta --extern futures_io=/home/jelmer/src/janitor/target/debug/deps/libfutures_io-40db0a981b134123.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-e2fb2d440b459c82.rmeta --extern hashbrown=/home/jelmer/src/janitor/target/debug/deps/libhashbrown-06a88afb9eab536b.rmeta --extern hashlink=/home/jelmer/src/janitor/target/debug/deps/libhashlink-59dbdb8fc63c8797.rmeta --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-9a486ccb6575c0f1.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-400fd68aa602ed65.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-e28561433c9ccd8a.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-805c5ad09980d0c9.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-9969bfe2b2f70651.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-f6f36781d1866faf.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-93d7573978769e30.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_core --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-core-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="_rt-async-std"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="any"' --cfg 'feature="async-io"' --cfg 'feature="async-std"' --cfg 'feature="chrono"' --cfg 'feature="crc"' --cfg 'feature="default"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="native-tls"' --cfg 'feature="offline"' --cfg 'feature="serde"' --cfg 'feature="serde_json"' --cfg 'feature="sha2"' --cfg 'feature="time"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_tls-native-tls", "_tls-none", "_tls-rustls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "any", "async-io", "async-std", "bigdecimal", "bit-vec", "bstr", "chrono", "crc", "default", "ipnetwork", "json", "mac_address", "migrate", "native-tls", "offline", "regex", "rust_decimal", "rustls", "rustls-native-certs", "rustls-pemfile", "serde", "serde_json", "sha2", "time", "tokio", "tokio-stream", "uuid", "webpki-roots"))' -C metadata=1226f1d2d71da552 -C extra-filename=-92c35d7cdf3ca55a --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_io=/home/jelmer/src/janitor/target/debug/deps/libasync_io-ab3ec6953b241562.rmeta --extern async_std=/home/jelmer/src/janitor/target/debug/deps/libasync_std-538cf550a53fa4e6.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern crossbeam_queue=/home/jelmer/src/janitor/target/debug/deps/libcrossbeam_queue-577e4d13a58a6351.rmeta --extern either=/home/jelmer/src/janitor/target/debug/deps/libeither-5d949479ced69761.rmeta --extern event_listener=/home/jelmer/src/janitor/target/debug/deps/libevent_listener-55331feab369961e.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_intrusive=/home/jelmer/src/janitor/target/debug/deps/libfutures_intrusive-2f7a96ce78bbdca9.rmeta --extern futures_io=/home/jelmer/src/janitor/target/debug/deps/libfutures_io-40db0a981b134123.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-639e19ba3d2b09a4.rmeta --extern hashbrown=/home/jelmer/src/janitor/target/debug/deps/libhashbrown-06a88afb9eab536b.rmeta --extern hashlink=/home/jelmer/src/janitor/target/debug/deps/libhashlink-59dbdb8fc63c8797.rmeta --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-9a486ccb6575c0f1.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-41e5bdd64bc7cd9e.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-98dff14998e6c879.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-59505d94661b74c2.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-44db4e5c0579f731.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name redis --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/redis-0.27.6/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="acl"' --cfg 'feature="aio"' --cfg 'feature="async-trait"' --cfg 'feature="bytes"' --cfg 'feature="default"' --cfg 'feature="futures-util"' --cfg 'feature="geospatial"' --cfg 'feature="keep-alive"' --cfg 'feature="pin-project-lite"' --cfg 'feature="script"' --cfg 'feature="sha1_smol"' --cfg 'feature="socket2"' --cfg 'feature="streams"' --cfg 'feature="tokio"' --cfg 'feature="tokio-comp"' --cfg 'feature="tokio-util"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("acl", "ahash", "aio", "async-native-tls", "async-std", "async-std-comp", "async-std-native-tls-comp", "async-std-rustls-comp", "async-std-tls-comp", "async-trait", "backon", "bigdecimal", "bytes", "cluster", "cluster-async", "connection-manager", "crc16", "default", "disable-client-setinfo", "futures", "futures-rustls", "futures-util", "geospatial", "hashbrown", "json", "keep-alive", "log", "native-tls", "num-bigint", "pin-project-lite", "r2d2", "rand", "rust_decimal", "rustls", "rustls-native-certs", "rustls-pemfile", "rustls-pki-types", "script", "sentinel", "serde", "serde_json", "sha1_smol", "socket2", "streams", "tcp_nodelay", "tls", "tls-native-tls", "tls-rustls", "tls-rustls-insecure", "tls-rustls-webpki-roots", "tokio", "tokio-comp", "tokio-native-tls", "tokio-native-tls-comp", "tokio-rustls", "tokio-rustls-comp", "tokio-util", "uuid", "webpki-roots"))' -C metadata=adb7d543a5b984bb -C extra-filename=-510744bc06555847 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern arc_swap=/home/jelmer/src/janitor/target/debug/deps/libarc_swap-bd5aa4a1e22f9e5d.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern combine=/home/jelmer/src/janitor/target/debug/deps/libcombine-044310925aab587b.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-639e19ba3d2b09a4.rmeta --extern itertools=/home/jelmer/src/janitor/target/debug/deps/libitertools-a8fb045921351b76.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern num_bigint=/home/jelmer/src/janitor/target/debug/deps/libnum_bigint-7a7a2dd2f34962d4.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern ryu=/home/jelmer/src/janitor/target/debug/deps/libryu-245a84a5a509b3a3.rmeta --extern sha1_smol=/home/jelmer/src/janitor/target/debug/deps/libsha1_smol-03061bab6b3928dd.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-631c96f5856b2ef3.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-fcc120ec1ab729cc.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/src/janitor/target/debug/build/janitor-0ecd700503a9d21a/build-script-build`
   Compiling hyper-util v0.1.10
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper_util --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-util-0.1.10/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="client"' --cfg 'feature="client-legacy"' --cfg 'feature="default"' --cfg 'feature="http1"' --cfg 'feature="http2"' --cfg 'feature="server"' --cfg 'feature="service"' --cfg 'feature="tokio"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__internal_happy_eyeballs_tests", "client", "client-legacy", "default", "full", "http1", "http2", "server", "server-auto", "server-graceful", "service", "tokio"))' -C metadata=5e17bd66e916020a -C extra-filename=-6b9aca8f1b8c16df --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-639e19ba3d2b09a4.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-a272ca6ae4bc1d3a.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-631c96f5856b2ef3.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling sqlx-postgres v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_postgres --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-postgres-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="chrono"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="offline"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("any", "bigdecimal", "bit-vec", "chrono", "ipnetwork", "json", "mac_address", "migrate", "offline", "rust_decimal", "time", "uuid"))' -C metadata=418b475fc6921686 -C extra-filename=-3dac6e92dc3cc5f0 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern atoi=/home/jelmer/src/janitor/target/debug/deps/libatoi-f8455101c6ea3fc4.rmeta --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-bf6eccdff131582a.rmeta --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern byteorder=/home/jelmer/src/janitor/target/debug/deps/libbyteorder-99211d86bad9f8bb.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-4eb151582e08ecdb.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern dotenvy=/home/jelmer/src/janitor/target/debug/deps/libdotenvy-c5aead2ec43889b7.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-1f4beae7161f5951.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-e2fb2d440b459c82.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern hkdf=/home/jelmer/src/janitor/target/debug/deps/libhkdf-46ecc70a36f2ad04.rmeta --extern hmac=/home/jelmer/src/janitor/target/debug/deps/libhmac-d886aec669ee88c9.rmeta --extern home=/home/jelmer/src/janitor/target/debug/deps/libhome-fc8b76e8d1b394eb.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-400fd68aa602ed65.rmeta --extern md5=/home/jelmer/src/janitor/target/debug/deps/libmd5-e96a7bc866d34328.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-c5d47a5e42694f78.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-805c5ad09980d0c9.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-9969bfe2b2f70651.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-952deb4c3052cf2d.rmeta --extern stringprep=/home/jelmer/src/janitor/target/debug/deps/libstringprep-3e84ecdea7cbe138.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-f6f36781d1866faf.rmeta --extern whoami=/home/jelmer/src/janitor/target/debug/deps/libwhoami-aa57d5d7654aa522.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper-tls v0.6.0
   Compiling axum v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper_tls --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-tls-0.6.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alpn", "vendored"))' -C metadata=90779060efdd14fc -C extra-filename=-2728cd7f54a8478e --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-a272ca6ae4bc1d3a.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-6b9aca8f1b8c16df.rmeta --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-41e5bdd64bc7cd9e.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tokio_native_tls=/home/jelmer/src/janitor/target/debug/deps/libtokio_native_tls-961a615e45412395.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name axum --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/axum-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::verbose_file_reads' '--warn=clippy::unused_self' --forbid=unsafe_code --warn=unreachable_pub '--warn=clippy::unnested_or_patterns' '--warn=clippy::uninlined_format_args' '--allow=clippy::type_complexity' '--warn=clippy::todo' '--warn=clippy::suboptimal_flops' '--warn=clippy::str_to_string' '--warn=clippy::rest_pat_in_fully_bound_structs' '--warn=clippy::option_option' '--warn=clippy::needless_continue' '--warn=clippy::needless_borrow' --warn=missing_docs --warn=missing_debug_implementations '--warn=clippy::mem_forget' '--warn=clippy::match_wildcard_for_single_variants' '--warn=clippy::match_on_vec_items' '--warn=clippy::macro_use_imports' '--warn=clippy::lossy_float_literal' '--warn=clippy::linkedlist' '--warn=clippy::inefficient_to_string' '--warn=clippy::imprecise_flops' '--warn=clippy::if_let_mutex' '--warn=clippy::fn_params_excessive_bools' '--warn=clippy::filter_map_next' '--warn=clippy::exit' '--warn=clippy::enum_glob_use' '--warn=clippy::empty_enum' '--warn=clippy::dbg_macro' '--warn=clippy::await_holding_lock' --cfg 'feature="default"' --cfg 'feature="form"' --cfg 'feature="http1"' --cfg 'feature="json"' --cfg 'feature="matched-path"' --cfg 'feature="original-uri"' --cfg 'feature="query"' --cfg 'feature="tokio"' --cfg 'feature="tower-log"' --cfg 'feature="tracing"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__private", "__private_docs", "default", "form", "http1", "http2", "json", "macros", "matched-path", "multipart", "original-uri", "query", "tokio", "tower-log", "tracing", "ws"))' -C metadata=bce8a59d64b1019c -C extra-filename=-76c738f7052be89f --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern axum_core=/home/jelmer/src/janitor/target/debug/deps/libaxum_core-c7084f1580c648d3.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern form_urlencoded=/home/jelmer/src/janitor/target/debug/deps/libform_urlencoded-072fa14f50efb53e.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-639e19ba3d2b09a4.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-a272ca6ae4bc1d3a.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-6b9aca8f1b8c16df.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern matchit=/home/jelmer/src/janitor/target/debug/deps/libmatchit-0d71d298d63a0df3.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern mime=/home/jelmer/src/janitor/target/debug/deps/libmime-c74000db2aaf511f.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern rustversion=/home/jelmer/src/janitor/target/debug/deps/librustversion-494b2fd16358ba50.so --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_path_to_error=/home/jelmer/src/janitor/target/debug/deps/libserde_path_to_error-72e72ae8986ce543.rmeta --extern serde_urlencoded=/home/jelmer/src/janitor/target/debug/deps/libserde_urlencoded-e0562f68e1545f98.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tower=/home/jelmer/src/janitor/target/debug/deps/libtower-051ada069bee8653.rmeta --extern tower_layer=/home/jelmer/src/janitor/target/debug/deps/libtower_layer-fff117c1c28e545a.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling reqwest v0.12.15
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name reqwest --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/reqwest-0.12.15/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(reqwest_unstable)' --cfg 'feature="__tls"' --cfg 'feature="blocking"' --cfg 'feature="charset"' --cfg 'feature="default"' --cfg 'feature="default-tls"' --cfg 'feature="h2"' --cfg 'feature="http2"' --cfg 'feature="json"' --cfg 'feature="macos-system-configuration"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__rustls", "__rustls-ring", "__tls", "blocking", "brotli", "charset", "cookies", "default", "default-tls", "deflate", "gzip", "h2", "hickory-dns", "http2", "http3", "json", "macos-system-configuration", "multipart", "native-tls", "native-tls-alpn", "native-tls-vendored", "rustls-tls", "rustls-tls-manual-roots", "rustls-tls-manual-roots-no-provider", "rustls-tls-native-roots", "rustls-tls-native-roots-no-provider", "rustls-tls-no-provider", "rustls-tls-webpki-roots", "rustls-tls-webpki-roots-no-provider", "socks", "stream", "trust-dns", "zstd"))' -C metadata=4d18d63f1a0c8cd6 -C extra-filename=-fee837cde7af54f6 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-b1a4de7ed4da6927.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern encoding_rs=/home/jelmer/src/janitor/target/debug/deps/libencoding_rs-58c41e4932181cd7.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-639e19ba3d2b09a4.rmeta --extern h2=/home/jelmer/src/janitor/target/debug/deps/libh2-7274ba537e8716e3.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-a272ca6ae4bc1d3a.rmeta --extern hyper_tls=/home/jelmer/src/janitor/target/debug/deps/libhyper_tls-2728cd7f54a8478e.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-6b9aca8f1b8c16df.rmeta --extern ipnet=/home/jelmer/src/janitor/target/debug/deps/libipnet-5873e4e1530bf49f.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern mime=/home/jelmer/src/janitor/target/debug/deps/libmime-c74000db2aaf511f.rmeta --extern native_tls_crate=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-41e5bdd64bc7cd9e.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern rustls_pemfile=/home/jelmer/src/janitor/target/debug/deps/librustls_pemfile-68bb2d10b5046659.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_urlencoded=/home/jelmer/src/janitor/target/debug/deps/libserde_urlencoded-e0562f68e1545f98.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tokio_native_tls=/home/jelmer/src/janitor/target/debug/deps/libtokio_native_tls-961a615e45412395.rmeta --extern tower=/home/jelmer/src/janitor/target/debug/deps/libtower-051ada069bee8653.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_postgres --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-postgres-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="any"' --cfg 'feature="chrono"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="time"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("any", "bigdecimal", "bit-vec", "chrono", "ipnetwork", "json", "mac_address", "migrate", "offline", "rust_decimal", "time", "uuid"))' -C metadata=d7f1c1f4fecb48a8 -C extra-filename=-304244ca052378cd --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern atoi=/home/jelmer/src/janitor/target/debug/deps/libatoi-08701d6ef2ff6341.rmeta --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-b1a4de7ed4da6927.rmeta --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern byteorder=/home/jelmer/src/janitor/target/debug/deps/libbyteorder-99211d86bad9f8bb.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern dotenvy=/home/jelmer/src/janitor/target/debug/deps/libdotenvy-c5aead2ec43889b7.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-639e19ba3d2b09a4.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern hkdf=/home/jelmer/src/janitor/target/debug/deps/libhkdf-46ecc70a36f2ad04.rmeta --extern hmac=/home/jelmer/src/janitor/target/debug/deps/libhmac-d886aec669ee88c9.rmeta --extern home=/home/jelmer/src/janitor/target/debug/deps/libhome-fc8b76e8d1b394eb.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern md5=/home/jelmer/src/janitor/target/debug/deps/libmd5-e96a7bc866d34328.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-c535a7a8ba116747.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-98dff14998e6c879.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-59505d94661b74c2.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-92c35d7cdf3ca55a.rmeta --extern stringprep=/home/jelmer/src/janitor/target/debug/deps/libstringprep-3e84ecdea7cbe138.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-44db4e5c0579f731.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern whoami=/home/jelmer/src/janitor/target/debug/deps/libwhoami-aa57d5d7654aa522.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling prometheus v0.14.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name prometheus --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/prometheus-0.14.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="protobuf"' --cfg 'feature="reqwest"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "gen", "libc", "nightly", "process", "procfs", "protobuf", "protobuf-codegen", "push", "reqwest"))' -C metadata=1c81e9f8b61dfcff -C extra-filename=-582abc32e89b07b8 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern cfg_if=/home/jelmer/src/janitor/target/debug/deps/libcfg_if-c30e1cd0415bac84.rmeta --extern fnv=/home/jelmer/src/janitor/target/debug/deps/libfnv-135eca59eff18b18.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern parking_lot=/home/jelmer/src/janitor/target/debug/deps/libparking_lot-e2b62b5be6a25198.rmeta --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-b1e3dc89f529da22.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-fee837cde7af54f6.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling sqlx-macros-core v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_macros_core --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-macros-core-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no --warn=unexpected_cfgs --cfg 'feature="_rt-async-std"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="async-std"' --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="json"' --cfg 'feature="macros"' --cfg 'feature="migrate"' --cfg 'feature="postgres"' --cfg 'feature="sqlx-postgres"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_sqlite", "_tls-native-tls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "async-std", "bigdecimal", "bit-vec", "chrono", "default", "derive", "ipnetwork", "json", "mac_address", "macros", "migrate", "mysql", "postgres", "rust_decimal", "sqlite", "sqlite-unbundled", "sqlx-mysql", "sqlx-postgres", "sqlx-sqlite", "time", "tokio", "uuid"))' -C metadata=5f3fa32bb0cac37b -C extra-filename=-50c14dff78fd0995 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_std=/home/jelmer/src/janitor/target/debug/deps/libasync_std-220d57f9d1d250bf.rmeta --extern dotenvy=/home/jelmer/src/janitor/target/debug/deps/libdotenvy-c5aead2ec43889b7.rmeta --extern either=/home/jelmer/src/janitor/target/debug/deps/libeither-accfca6c6f5e11c6.rmeta --extern heck=/home/jelmer/src/janitor/target/debug/deps/libheck-4d6a9c8516811f18.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern proc_macro2=/home/jelmer/src/janitor/target/debug/deps/libproc_macro2-a7e2001652539cec.rmeta --extern quote=/home/jelmer/src/janitor/target/debug/deps/libquote-8533776b6f1db290.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-805c5ad09980d0c9.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-952deb4c3052cf2d.rmeta --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-3dac6e92dc3cc5f0.rmeta --extern syn=/home/jelmer/src/janitor/target/debug/deps/libsyn-7fe0b75e1b133791.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-b5647751c7f60687.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-93d7573978769e30.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling sqlx-macros v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_macros --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-macros-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type proc-macro --emit=dep-info,link -C prefer-dynamic -C embed-bitcode=no '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="_rt-async-std"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="json"' --cfg 'feature="macros"' --cfg 'feature="migrate"' --cfg 'feature="postgres"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_tls-native-tls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "bigdecimal", "bit-vec", "chrono", "default", "derive", "ipnetwork", "json", "mac_address", "macros", "migrate", "mysql", "postgres", "rust_decimal", "sqlite", "sqlite-unbundled", "time", "uuid"))' -C metadata=1a11cca9bc80ea2a -C extra-filename=-4ccc58fcd73349ec --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern proc_macro2=/home/jelmer/src/janitor/target/debug/deps/libproc_macro2-a7e2001652539cec.rlib --extern quote=/home/jelmer/src/janitor/target/debug/deps/libquote-8533776b6f1db290.rlib --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-952deb4c3052cf2d.rlib --extern sqlx_macros_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_macros_core-50c14dff78fd0995.rlib --extern syn=/home/jelmer/src/janitor/target/debug/deps/libsyn-7fe0b75e1b133791.rlib --extern proc_macro --cap-lints allow --cfg tokio_unstable`
   Compiling sqlx v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="_rt-async-std"' --cfg 'feature="any"' --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="json"' --cfg 'feature="macros"' --cfg 'feature="migrate"' --cfg 'feature="postgres"' --cfg 'feature="runtime-async-std"' --cfg 'feature="runtime-async-std-native-tls"' --cfg 'feature="sqlx-macros"' --cfg 'feature="sqlx-postgres"' --cfg 'feature="tls-native-tls"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_sqlite", "_unstable-all-types", "all-databases", "any", "bigdecimal", "bit-vec", "chrono", "default", "derive", "ipnetwork", "json", "mac_address", "macros", "migrate", "mysql", "postgres", "regexp", "runtime-async-std", "runtime-async-std-native-tls", "runtime-async-std-rustls", "runtime-tokio", "runtime-tokio-native-tls", "runtime-tokio-rustls", "rust_decimal", "sqlite", "sqlite-unbundled", "sqlx-macros", "sqlx-mysql", "sqlx-postgres", "sqlx-sqlite", "time", "tls-native-tls", "tls-none", "tls-rustls", "tls-rustls-aws-lc-rs", "tls-rustls-ring", "tls-rustls-ring-native-roots", "tls-rustls-ring-webpki", "uuid"))' -C metadata=497b2976b51a8fc4 -C extra-filename=-68b312052ef8aafa --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-92c35d7cdf3ca55a.rmeta --extern sqlx_macros=/home/jelmer/src/janitor/target/debug/deps/libsqlx_macros-4ccc58fcd73349ec.so --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-304244ca052378cd.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling debversion v0.4.4
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debversion --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debversion-0.4.4/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="python-debian"' --cfg 'feature="serde"' --cfg 'feature="sqlx"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "python-debian", "serde", "sqlx"))' -C metadata=c328d390e1096818 -C extra-filename=-394d0669e92df524 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-68b312052ef8aafa.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling debian-control v0.1.41
   Compiling debian-changelog v0.2.0
   Compiling debian-copyright v0.1.27
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_control --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-control-0.1.41/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="lossless"' --cfg 'feature="python-debian"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("chrono", "default", "lossless", "python-debian", "serde"))' -C metadata=75159bb2927b3896 -C extra-filename=-a13352f3fb66f966 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-85e845bd7914badc.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-394d0669e92df524.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern rowan=/home/jelmer/src/janitor/target/debug/deps/librowan-e05f61e8ea0a6615.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_changelog --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-changelog-0.2.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=87366c05197db021 -C extra-filename=-c2a11b15f057ea0d --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-394d0669e92df524.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern rowan=/home/jelmer/src/janitor/target/debug/deps/librowan-c1fdca08b3081a85.rmeta --extern textwrap=/home/jelmer/src/janitor/target/debug/deps/libtextwrap-5e0992fd5b607969.rmeta --extern whoami=/home/jelmer/src/janitor/target/debug/deps/libwhoami-aa57d5d7654aa522.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_copyright --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-copyright-0.1.27/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=f35b225a7e73539f -C extra-filename=-f4a73b26887490fa --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-85e845bd7914badc.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-394d0669e92df524.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling breezyshim v0.1.227
   Compiling buildlog-consultant v0.1.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name breezyshim --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/breezyshim-0.1.227/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="auto-initialize"' --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="dirty-tracker"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("auto-initialize", "debian", "default", "dirty-tracker", "sqlx"))' -C metadata=2e57f28d41aab26d -C extra-filename=-b41b20b42906ee29 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern ctor=/home/jelmer/src/janitor/target/debug/deps/libctor-72258acac2d0b9ee.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-c2a11b15f057ea0d.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-a13352f3fb66f966.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-394d0669e92df524.rmeta --extern difflib=/home/jelmer/src/janitor/target/debug/deps/libdifflib-20593172ebc6932e.rmeta --extern dirty_tracker=/home/jelmer/src/janitor/target/debug/deps/libdirty_tracker-15c2a709d36ea33e.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern patchkit=/home/jelmer/src/janitor/target/debug/deps/libpatchkit-045f9ba36150b582.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern pyo3_filelike=/home/jelmer/src/janitor/target/debug/deps/libpyo3_filelike-bc0667b965758a04.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-34844f972d674a37.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name buildlog_consultant --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/buildlog-consultant-0.1.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("chatgpt", "cli", "default", "tokio"))' -C metadata=bacc03f4398d6150 -C extra-filename=-043b24c5767f7161 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-a13352f3fb66f966.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-394d0669e92df524.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-808eaebb057ce37e.rmeta --extern fancy_regex=/home/jelmer/src/janitor/target/debug/deps/libfancy_regex-0688edb11485e39a.rmeta --extern inventory=/home/jelmer/src/janitor/target/debug/deps/libinventory-97a54ddffe78909c.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern pep508_rs=/home/jelmer/src/janitor/target/debug/deps/libpep508_rs-9aa259a9ee5b2c33.rlib --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_yaml=/home/jelmer/src/janitor/target/debug/deps/libserde_yaml-d19b32863dd48a61.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern text_size=/home/jelmer/src/janitor/target/debug/deps/libtext_size-68834c6d82d5a146.rmeta --extern textwrap=/home/jelmer/src/janitor/target/debug/deps/libtextwrap-5e0992fd5b607969.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling debian-analyzer v0.158.25
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_analyzer --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-analyzer-0.158.25/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="merge3"' --cfg 'feature="python"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "default", "merge3", "python", "svp", "udd"))' -C metadata=80b22781d1cd8017 -C extra-filename=-fc484caa1e1d16fa --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-b41b20b42906ee29.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern configparser=/home/jelmer/src/janitor/target/debug/deps/libconfigparser-aaa60c0f437f3031.rmeta --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-85e845bd7914badc.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-c2a11b15f057ea0d.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-a13352f3fb66f966.rmeta --extern debian_copyright=/home/jelmer/src/janitor/target/debug/deps/libdebian_copyright-f4a73b26887490fa.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-394d0669e92df524.rmeta --extern dep3=/home/jelmer/src/janitor/target/debug/deps/libdep3-cef4e33c810b0205.rmeta --extern difflib=/home/jelmer/src/janitor/target/debug/deps/libdifflib-20593172ebc6932e.rmeta --extern distro_info=/home/jelmer/src/janitor/target/debug/deps/libdistro_info-e2f22ea1e25dba0a.rmeta --extern filetime=/home/jelmer/src/janitor/target/debug/deps/libfiletime-74a11c28d8d6d9ef.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern makefile_lossless=/home/jelmer/src/janitor/target/debug/deps/libmakefile_lossless-f7c86ff44e7ff685.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern merge3=/home/jelmer/src/janitor/target/debug/deps/libmerge3-1c24ac3badc9ba5b.rmeta --extern patchkit=/home/jelmer/src/janitor/target/debug/deps/libpatchkit-045f9ba36150b582.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-fee837cde7af54f6.rmeta --extern semver=/home/jelmer/src/janitor/target/debug/deps/libsemver-11fd74ac82b27f0f.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha1=/home/jelmer/src/janitor/target/debug/deps/libsha1-666ba0d12790bffa.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-34844f972d674a37.rmeta --extern toml_edit=/home/jelmer/src/janitor/target/debug/deps/libtoml_edit-51c35483d814c85f.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling silver-platter v0.5.48
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name silver_platter --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/silver-platter-0.5.48/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="detect-update-changelog"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default", "detect-update-changelog", "gpg", "last-attempt-db", "pyo3"))' -C metadata=34bfc14355e64f6f -C extra-filename=-c5acb40799f6b2a7 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-b41b20b42906ee29.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-fc484caa1e1d16fa.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-c2a11b15f057ea0d.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-a13352f3fb66f966.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-394d0669e92df524.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-808eaebb057ce37e.rmeta --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-5457d7c65e6ff7c6.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-6571000d5ff98899.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-fee837cde7af54f6.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_yaml=/home/jelmer/src/janitor/target/debug/deps/libserde_yaml-d19b32863dd48a61.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-34844f972d674a37.rmeta --extern tera=/home/jelmer/src/janitor/target/debug/deps/libtera-39fe85b2b15ed66c.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --extern xdg=/home/jelmer/src/janitor/target/debug/deps/libxdg-23f110d46d019c5b.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor --edition=2021 src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="debian"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("debian", "default", "gcp", "gcs", "stackdriver_logger"))' -C metadata=f855119bcaa7142a -C extra-filename=-0850277fa17493a4 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-74b18faa72ce45a2.rmeta --extern async_compression=/home/jelmer/src/janitor/target/debug/deps/libasync_compression-aa22db53322b438f.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-b41b20b42906ee29.rmeta --extern buildlog_consultant=/home/jelmer/src/janitor/target/debug/deps/libbuildlog_consultant-043b24c5767f7161.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-a13352f3fb66f966.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-394d0669e92df524.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-808eaebb057ce37e.rmeta --extern fancy_regex=/home/jelmer/src/janitor/target/debug/deps/libfancy_regex-0688edb11485e39a.rmeta --extern filetime=/home/jelmer/src/janitor/target/debug/deps/libfiletime-74a11c28d8d6d9ef.rmeta --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern futures=/home/jelmer/src/janitor/target/debug/deps/libfutures-fb62b6dea467bdde.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-582abc32e89b07b8.rmeta --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-b1e3dc89f529da22.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-fee837cde7af54f6.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-c5acb40799f6b2a7.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-68b312052ef8aafa.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-92c35d7cdf3ca55a.rmeta --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-304244ca052378cd.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-34844f972d674a37.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cfg tokio_unstable`
warning: function `reprocess_run_logs` is never used
 --> src/reprocess_logs.rs:8:10
  |
8 | async fn reprocess_run_logs(
  |          ^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(dead_code)]` on by default

warning: field `branch_url` is never read
  --> src/schedule.rs:32:5
   |
30 | pub struct ScheduleRequest {
   |            --------------- field in this struct
31 |     codebase: String,
32 |     branch_url: String,
   |     ^^^^^^^^^^

warning: function `has_cotenants` is never used
  --> src/state.rs:80:10
   |
80 | async fn has_cotenants(
   |          ^^^^^^^^^^^^^

warning: field `name` is never read
  --> src/state.rs:87:13
   |
86 |     struct Codebase {
   |            -------- field in this struct
87 |         pub name: String,
   |             ^^^^
   |
   = note: `Codebase` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: function `iter_publishable_suites` is never used
   --> src/state.rs:113:10
    |
113 | async fn iter_publishable_suites(
    |          ^^^^^^^^^^^^^^^^^^^^^^^

   Compiling janitor-runner v0.1.0 (/home/jelmer/src/janitor/runner)
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor_runner --edition=2021 runner/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="debian"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default"))' -C metadata=e511c4ae7ce5040c -C extra-filename=-bae234545465a8de --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-76c738f7052be89f.rmeta --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-b41b20b42906ee29.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-a13352f3fb66f966.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-394d0669e92df524.rmeta --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-0850277fa17493a4.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern redis=/home/jelmer/src/janitor/target/debug/deps/libredis-510744bc06555847.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-fee837cde7af54f6.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_with=/home/jelmer/src/janitor/target/debug/deps/libserde_with-9fc37280aeb06a21.rmeta --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-c5acb40799f6b2a7.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-68b312052ef8aafa.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-92c35d7cdf3ca55a.rmeta --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-304244ca052378cd.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-f832f4eaea3b145b.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-f592545570768eb3.rmeta --cfg tokio_unstable`
warning: unused variable: `state`
 --> runner/src/web.rs:8:31
  |
8 | async fn queue_position(State(state): State<Arc<AppState>>) {
  |                               ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`
  |
  = note: `#[warn(unused_variables)]` on by default

warning: unused variable: `state`
  --> runner/src/web.rs:12:33
   |
12 | async fn schedule_control(State(state): State<Arc<AppState>>) {
   |                                 ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `state`
  --> runner/src/web.rs:16:25
   |
16 | async fn schedule(State(state): State<Arc<AppState>>) {
   |                         ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `state`
  --> runner/src/web.rs:20:23
   |
20 | async fn status(State(state): State<Arc<AppState>>) {
   |                       ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `state`
  --> runner/src/web.rs:24:26
   |
24 | async fn log_index(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                          ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `id`
  --> runner/src/web.rs:24:61
   |
24 | async fn log_index(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                                                             ^^ help: if this is intentional, prefix it with an underscore: `_id`

warning: unused variable: `state`
  --> runner/src/web.rs:29:11
   |
29 |     State(state): State<Arc<AppState>>,
   |           ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `id`
  --> runner/src/web.rs:30:10
   |
30 |     Path(id): Path<String>,
   |          ^^ help: if this is intentional, prefix it with an underscore: `_id`

warning: unused variable: `filename`
  --> runner/src/web.rs:31:10
   |
31 |     Path(filename): Path<String>,
   |          ^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_filename`

warning: unused variable: `state`
  --> runner/src/web.rs:36:21
   |
36 | async fn kill(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                     ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `id`
  --> runner/src/web.rs:36:56
   |
36 | async fn kill(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                                                        ^^ help: if this is intentional, prefix it with an underscore: `_id`

warning: unused variable: `state`
  --> runner/src/web.rs:40:30
   |
40 | async fn get_codebases(State(state): State<Arc<AppState>>) {
   |                              ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `state`
  --> runner/src/web.rs:44:33
   |
44 | async fn update_codebases(State(state): State<Arc<AppState>>) {
   |                                 ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `state`
  --> runner/src/web.rs:48:33
   |
48 | async fn delete_candidate(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                                 ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `id`
  --> runner/src/web.rs:48:68
   |
48 | async fn delete_candidate(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                                                                    ^^ help: if this is intentional, prefix it with an underscore: `_id`

warning: unused variable: `state`
  --> runner/src/web.rs:52:24
   |
52 | async fn get_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                        ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `id`
  --> runner/src/web.rs:52:59
   |
52 | async fn get_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                                                           ^^ help: if this is intentional, prefix it with an underscore: `_id`

warning: unused variable: `state`
  --> runner/src/web.rs:56:27
   |
56 | async fn update_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                           ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `id`
  --> runner/src/web.rs:56:62
   |
56 | async fn update_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                                                              ^^ help: if this is intentional, prefix it with an underscore: `_id`

warning: unused variable: `state`
  --> runner/src/web.rs:60:32
   |
60 | async fn get_active_runs(State(state): State<Arc<AppState>>) {
   |                                ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `state`
  --> runner/src/web.rs:64:31
   |
64 | async fn get_active_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                               ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `id`
  --> runner/src/web.rs:64:66
   |
64 | async fn get_active_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                                                                  ^^ help: if this is intentional, prefix it with an underscore: `_id`

warning: unused variable: `state`
  --> runner/src/web.rs:68:32
   |
68 | async fn peek_active_run(State(state): State<Arc<AppState>>) {
   |                                ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `state`
  --> runner/src/web.rs:72:26
   |
72 | async fn get_queue(State(state): State<Arc<AppState>>) {
   |                          ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `state`
  --> runner/src/web.rs:84:34
   |
84 | async fn finish_active_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                                  ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `id`
  --> runner/src/web.rs:84:69
   |
84 | async fn finish_active_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                                                                     ^^ help: if this is intentional, prefix it with an underscore: `_id`

warning: unused variable: `state`
  --> runner/src/web.rs:92:30
   |
92 | async fn public_assign(State(state): State<Arc<AppState>>) {
   |                              ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `state`
  --> runner/src/web.rs:96:30
   |
96 | async fn public_finish(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                              ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `id`
  --> runner/src/web.rs:96:65
   |
96 | async fn public_finish(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
   |                                                                 ^^ help: if this is intentional, prefix it with an underscore: `_id`

warning: unused variable: `state`
   --> runner/src/web.rs:100:38
    |
100 | async fn public_get_active_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
    |                                      ^^^^^ help: if this is intentional, prefix it with an underscore: `_state`

warning: unused variable: `id`
   --> runner/src/web.rs:100:73
    |
100 | async fn public_get_active_run(State(state): State<Arc<AppState>>, Path(id): Path<String>) {
    |                                                                         ^^ help: if this is intentional, prefix it with an underscore: `_id`

warning: `janitor` (lib) generated 5 warnings
warning: `janitor-runner` (lib) generated 31 warnings
       Dirty runner-py v0.0.0 (/home/jelmer/src/janitor/runner-py): dependency info changed
   Compiling runner-py v0.0.0 (/home/jelmer/src/janitor/runner-py)
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name runner_py --edition=2021 runner-py/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type cdylib --emit=dep-info,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="extension-module"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("extension-module"))' -C metadata=d6179e631861fca6 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-b41b20b42906ee29.rlib --extern janitor_runner=/home/jelmer/src/janitor/target/debug/deps/libjanitor_runner-bae234545465a8de.rlib --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-630fe32e68d387c8.rlib --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-f11d63d32a114e1a.rlib --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-c5acb40799f6b2a7.rlib --cfg tokio_unstable`
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.11s
Copying rust artifact from target/debug/librunner_py.so to py/janitor/_runner.cpython-313-x86_64-linux-gnu.so
cargo rustc --lib --message-format=json-render-diagnostics --manifest-path site-py/Cargo.toml -v --features extension-module pyo3/extension-module --crate-type cdylib --
       Fresh unicode-ident v1.0.18
       Fresh proc-macro2 v1.0.94
       Fresh once_cell v1.21.0
       Fresh stable_deref_trait v1.2.0
       Fresh memchr v2.7.4
       Fresh regex-syntax v0.8.5
       Fresh autocfg v1.4.0
       Fresh litemap v0.7.5
       Fresh writeable v0.5.5
       Fresh icu_locid_transform_data v1.5.0
       Fresh smallvec v1.14.0
       Fresh icu_properties_data v1.5.0
       Fresh utf16_iter v1.0.5
       Fresh icu_normalizer_data v1.5.0
       Fresh write16 v1.0.0
       Fresh utf8_iter v1.0.4
       Fresh iana-time-zone v0.1.61
       Fresh percent-encoding v2.3.1
       Fresh text-size v1.1.1
       Fresh rustc-hash v1.1.0
       Fresh hashbrown v0.15.2
       Fresh unicode-width v0.2.0
       Fresh countme v3.0.1
       Fresh hashbrown v0.14.5
       Fresh equivalent v1.0.2
       Fresh ryu v1.0.20
       Fresh itoa v1.0.15
       Fresh heck v0.5.0
       Fresh unscanny v0.1.0
       Fresh quote v1.0.39
       Fresh aho-corasick v1.1.3
       Fresh form_urlencoded v1.2.1
       Fresh rowan v0.16.1
       Fresh indexmap v2.8.0
       Fresh version-ranges v0.1.1
       Fresh bit-vec v0.8.0
       Fresh either v1.15.0
       Fresh boxcar v0.2.10
       Fresh unicode-linebreak v0.1.5
       Fresh unsafe-libyaml v0.2.11
       Fresh smawk v0.3.2
       Fresh urlencoding v2.1.3
       Fresh log v0.4.27
       Fresh rustc-hash v2.1.1
       Fresh lazy_static v1.5.0
       Fresh maplit v1.0.2
       Fresh indoc v2.0.6
       Fresh cfg-if v1.0.0
       Fresh unindent v0.2.4
       Fresh inventory v0.3.20
       Fresh shlex v1.3.0
       Fresh arc-swap v1.7.1
       Fresh syn v2.0.100
       Fresh target-lexicon v0.12.16
       Fresh regex-automata v0.4.9
       Fresh itertools v0.13.0
       Fresh bit-set v0.8.0
       Fresh libc v0.2.170
       Fresh textwrap v0.16.2
       Fresh synstructure v0.13.1
       Fresh zerovec-derive v0.10.3
       Fresh displaydoc v0.2.5
       Fresh serde_derive v1.0.219
       Fresh icu_provider_macros v1.5.0
       Fresh num-traits v0.2.19
       Fresh regex v1.11.1
       Fresh thiserror-impl v1.0.69
       Fresh deb822-derive v0.2.0
       Fresh fancy-regex v0.14.0
       Fresh memoffset v0.9.1
       Fresh zerofrom-derive v0.1.6
       Fresh yoke-derive v0.7.5
       Fresh serde v1.0.219
       Fresh chrono v0.4.40
       Fresh lazy-regex-proc_macros v3.4.1
       Fresh thiserror v1.0.69
       Fresh zerofrom v0.1.6
       Fresh pyo3-build-config v0.22.6
       Fresh lazy-regex v3.4.1
       Fresh deb822-lossless v0.2.4
       Fresh pep440_rs v0.7.3
       Fresh serde_yaml v0.9.34+deprecated
       Fresh serde_json v1.0.140
       Fresh yoke v0.7.5
       Fresh debversion v0.4.4
       Fresh zerovec v0.10.4
       Fresh tinystr v0.7.6
       Fresh icu_collections v1.5.0
       Fresh pyo3-macros-backend v0.22.6
       Fresh pyo3-ffi v0.22.6
       Fresh icu_locid v1.5.0
       Fresh pyo3-macros v0.22.6
       Fresh icu_provider v1.5.0
       Fresh pyo3 v0.22.6
       Fresh icu_locid_transform v1.5.0
       Fresh pyo3-log v0.11.0
       Fresh icu_properties v1.5.1
       Fresh icu_normalizer v1.5.0
       Fresh idna_adapter v1.2.0
       Fresh idna v1.0.3
       Fresh url v2.5.4
       Fresh debian-control v0.1.41
       Fresh pep508_rs v0.9.2
       Fresh buildlog-consultant v0.1.1
       Dirty janitor-site v0.0.0 (/home/jelmer/src/janitor/site): the file `site/src/lib.rs` has changed (1745670935.214710567s, 1month 13h 33m 4s after last build at 1742992135.204595543s)
   Compiling janitor-site v0.0.0 (/home/jelmer/src/janitor/site)
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor_site --edition=2021 site/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=922e8a9878be3e5f -C extra-filename=-0c1886e8a61e51aa --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern buildlog_consultant=/home/jelmer/src/janitor/target/debug/deps/libbuildlog_consultant-62083f7f20020f32.rmeta --cfg tokio_unstable`
       Dirty site-py v0.0.0 (/home/jelmer/src/janitor/site-py): the dependency janitor_site was rebuilt
   Compiling site-py v0.0.0 (/home/jelmer/src/janitor/site-py)
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name site_py --edition=2021 site-py/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type cdylib --emit=dep-info,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="extension-module"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("extension-module"))' -C metadata=5c3849833840b4fd --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern janitor_site=/home/jelmer/src/janitor/target/debug/deps/libjanitor_site-0c1886e8a61e51aa.rlib --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-71b21327a59713e8.rlib --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-508c37875811be39.rlib --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-9c06b445219c2c97.rlib --cfg tokio_unstable`
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.51s
Copying rust artifact from target/debug/libsite_py.so to py/janitor/_site.cpython-313-x86_64-linux-gnu.so
cargo build --manifest-path mail-filter/Cargo.toml --message-format=json-render-diagnostics -v --features cmdline
       Fresh unicode-ident v1.0.18
       Fresh cfg-if v1.0.0
       Fresh autocfg v1.4.0
       Fresh pin-project-lite v0.2.16
       Fresh shlex v1.3.0
       Fresh value-bag v1.10.0
       Fresh futures-core v0.3.31
       Fresh pkg-config v0.3.32
       Fresh vcpkg v0.2.15
       Fresh smallvec v1.14.0
       Fresh once_cell v1.21.0
       Fresh futures-io v0.3.31
       Fresh stable_deref_trait v1.2.0
       Fresh bytes v1.10.1
       Fresh parking v2.2.1
       Fresh siphasher v0.3.11
       Fresh itoa v1.0.15
       Fresh siphasher v1.0.1
       Fresh bitflags v2.9.0
       Fresh litemap v0.7.5
       Fresh memchr v2.7.4
       Fresh fnv v1.0.7
       Fresh writeable v0.5.5
       Fresh futures-sink v0.3.31
       Fresh atomic-waker v1.1.2
       Fresh proc-macro2 v1.0.94
       Fresh cc v1.2.16
       Fresh log v0.4.27
       Fresh tracing-core v0.1.33
       Fresh phf_shared v0.11.3
       Fresh scopeguard v1.2.0
       Fresh icu_locid_transform_data v1.5.0
       Fresh new_debug_unreachable v1.0.6
       Fresh http v1.2.0
       Fresh fastrand v2.3.0
       Fresh icu_properties_data v1.5.0
       Fresh pin-utils v0.1.0
       Fresh mac v0.1.1
       Fresh phf_shared v0.10.0
       Fresh equivalent v1.0.2
       Fresh hashbrown v0.15.2
       Fresh openssl-probe v0.1.6
       Fresh foreign-types-shared v0.1.1
       Fresh utf16_iter v1.0.5
       Fresh utf-8 v0.7.6
       Fresh precomputed-hash v0.1.1
       Fresh quote v1.0.39
       Fresh libc v0.2.170
   Compiling openssl-sys v0.9.107
       Fresh zerocopy v0.8.23
       Fresh crossbeam-utils v0.8.21
       Fresh serde v1.0.219
       Fresh futf v0.1.5
       Fresh icu_normalizer_data v1.5.0
       Fresh utf8_iter v1.0.4
       Fresh write16 v1.0.0
       Fresh futures-task v0.3.31
       Fresh phf v0.10.1
       Fresh http-body v1.0.1
       Fresh indexmap v2.8.0
       Fresh foreign-types v0.3.2
       Fresh futures-lite v2.6.0
       Fresh percent-encoding v2.3.1
       Fresh try-lock v0.2.5
       Fresh linux-raw-sys v0.4.15
       Fresh encoding_rs v0.8.35
       Fresh futures-channel v0.3.31
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name build_script_main --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/openssl-sys-0.9.107/build/main.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("aws-lc", "bindgen", "bssl-sys", "openssl-src", "unstable_boringssl", "vendored"))' -C metadata=836acf7da0fd6410 -C extra-filename=-d9cad12f5b470f35 --out-dir /home/jelmer/src/janitor/target/debug/build/openssl-sys-d9cad12f5b470f35 -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern cc=/home/jelmer/src/janitor/target/debug/deps/libcc-abfd8c5d408f28c5.rlib --extern pkg_config=/home/jelmer/src/janitor/target/debug/deps/libpkg_config-4ab588afd44f44b3.rlib --extern vcpkg=/home/jelmer/src/janitor/target/debug/deps/libvcpkg-a8ffa4005601983f.rlib --cap-lints allow --cfg tokio_unstable`
       Fresh syn v2.0.100
       Fresh getrandom v0.2.15
       Fresh ppv-lite86 v0.2.21
       Fresh concurrent-queue v2.5.0
       Fresh slab v0.4.9
       Fresh socket2 v0.5.8
       Fresh mio v1.0.3
       Fresh parking_lot_core v0.9.10
       Fresh lock_api v0.4.12
       Fresh tendril v0.4.3
       Fresh syn v1.0.109
       Fresh want v0.3.1
       Fresh form_urlencoded v1.2.1
       Fresh httparse v1.10.1
       Fresh tower-service v0.3.3
       Fresh utf8parse v0.2.2
       Fresh async-task v4.7.1
       Fresh event-listener v2.5.3
       Fresh piper v0.2.4
       Fresh is_terminal_polyfill v1.70.1
       Fresh base64 v0.22.1
       Fresh ryu v1.0.20
       Fresh colorchoice v1.0.3
       Fresh anstyle-query v1.1.2
       Fresh anstyle v1.0.10
       Fresh synstructure v0.13.1
       Fresh rand_core v0.6.4
       Fresh zerovec-derive v0.10.3
       Fresh displaydoc v0.2.5
       Fresh tracing-attributes v0.1.28
       Fresh icu_provider_macros v1.5.0
   Compiling tokio v1.44.2
       Fresh event-listener v5.4.0
       Fresh parking_lot v0.12.3
       Fresh openssl-macros v0.1.1
       Fresh futures-util v0.3.31
       Fresh rustix v0.38.44
       Fresh libz-sys v1.1.21
       Fresh pin-project-internal v1.1.10
       Fresh libnghttp2-sys v0.1.11+1.64.0
       Fresh async-channel v1.9.0
       Fresh anstyle-parse v0.2.6
       Fresh async-executor v1.13.1
       Fresh http-body-util v0.1.3
       Fresh sync_wrapper v1.0.2
       Fresh waker-fn v1.2.0
       Fresh heck v0.5.0
       Fresh rustls-pki-types v1.11.0
       Fresh clap_lex v0.7.4
       Fresh fastrand v1.9.0
       Fresh strsim v0.11.1
       Fresh bit-vec v0.6.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.44.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(loom)' --check-cfg 'cfg(mio_unsupported_force_poll_poll)' --check-cfg 'cfg(tokio_allow_from_blocking_fd)' --check-cfg 'cfg(tokio_internal_mt_counters)' --check-cfg 'cfg(tokio_no_parking_lot)' --check-cfg 'cfg(tokio_no_tuning_tests)' --check-cfg 'cfg(tokio_taskdump)' --check-cfg 'cfg(tokio_unstable)' --cfg 'feature="bytes"' --cfg 'feature="default"' --cfg 'feature="io-util"' --cfg 'feature="libc"' --cfg 'feature="mio"' --cfg 'feature="net"' --cfg 'feature="rt"' --cfg 'feature="socket2"' --cfg 'feature="sync"' --cfg 'feature="time"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("bytes", "default", "fs", "full", "io-std", "io-util", "libc", "macros", "mio", "net", "parking_lot", "process", "rt", "rt-multi-thread", "signal", "signal-hook-registry", "socket2", "sync", "test-util", "time", "tokio-macros", "tracing", "windows-sys"))' -C metadata=891ce18631b435eb -C extra-filename=-9641ac8798494502 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-5457d7c65e6ff7c6.rmeta --extern mio=/home/jelmer/src/janitor/target/debug/deps/libmio-ea8f193d550eeb3d.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-631c96f5856b2ef3.rmeta --cap-lints allow --cfg tokio_unstable`
       Fresh zerofrom-derive v0.1.6
       Fresh yoke-derive v0.7.5
       Fresh rand_chacha v0.3.1
       Fresh tracing v0.1.41
       Fresh event-listener-strategy v0.5.3
       Fresh string_cache v0.8.8
       Fresh pin-project v1.1.10
       Fresh anstream v0.6.18
       Fresh mime v0.3.17
       Fresh tower-layer v0.3.3
       Fresh serde_json v1.0.140
       Fresh futures-lite v1.13.0
       Fresh bit-set v0.5.3
       Fresh clap_derive v4.5.32
       Fresh rustls-pemfile v2.2.0
       Fresh polling v2.8.0
       Fresh sluice v0.5.5
       Fresh charset v0.1.5
       Fresh serde_urlencoded v0.7.1
       Fresh kv-log-macro v1.0.7
       Fresh http v0.2.12
       Fresh ipnet v2.11.0
       Fresh quoted_printable v0.5.1
       Fresh castaway v0.1.2
       Fresh data-encoding v2.8.0
       Fresh zerofrom v0.1.6
       Fresh rand v0.8.5
       Fresh polling v3.7.4
       Fresh async-lock v3.4.0
       Fresh async-channel v2.3.1
       Fresh tracing-futures v0.2.5
   Compiling clap_builder v4.5.36
       Fresh mailparse v0.16.1
       Fresh yoke v0.7.5
       Fresh phf_generator v0.11.3
       Fresh phf_generator v0.10.0
       Fresh async-io v2.4.0
       Fresh blocking v1.6.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name clap_builder --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/clap_builder-4.5.36/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::zero_sized_map_values' '--warn=clippy::verbose_file_reads' --warn=unused_qualifications --warn=unused_macro_rules --warn=unused_lifetimes --warn=unsafe_op_in_unsafe_fn --warn=unreachable_pub '--warn=clippy::uninlined_format_args' '--warn=clippy::trait_duplication_in_bounds' '--warn=clippy::todo' '--warn=clippy::string_lit_as_bytes' '--warn=clippy::string_add_assign' '--warn=clippy::semicolon_if_nothing_returned' '--warn=clippy::self_named_module_files' '--warn=clippy::same_functions_in_if_condition' '--warn=clippy::rest_pat_in_fully_bound_structs' '--warn=clippy::ref_option_ref' '--warn=clippy::redundant_feature_names' '--warn=clippy::rc_mutex' '--warn=clippy::ptr_as_ptr' '--warn=clippy::path_buf_push_overwrite' '--warn=clippy::negative_feature_names' '--warn=clippy::needless_for_each' '--warn=clippy::needless_continue' '--warn=clippy::mutex_integer' '--allow=clippy::multiple_bound_locations' '--warn=clippy::mem_forget' '--warn=clippy::macro_use_imports' '--warn=clippy::lossy_float_literal' '--warn=clippy::linkedlist' '--allow=clippy::let_and_return' '--warn=clippy::large_types_passed_by_value' '--warn=clippy::large_stack_arrays' '--warn=clippy::large_digit_groups' '--warn=clippy::invalid_upcast_comparisons' '--warn=clippy::infinite_loop' '--warn=clippy::inefficient_to_string' '--warn=clippy::inconsistent_struct_constructor' '--warn=clippy::imprecise_flops' '--warn=clippy::implicit_clone' '--allow=clippy::if_same_then_else' '--warn=clippy::from_iter_instead_of_collect' '--warn=clippy::fn_params_excessive_bools' '--warn=clippy::float_cmp_const' '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::fallible_impl_from' '--warn=clippy::explicit_into_iter_loop' '--warn=clippy::explicit_deref_methods' '--warn=clippy::expl_impl_clone_on_copy' '--warn=clippy::enum_glob_use' '--warn=clippy::empty_enum' '--warn=clippy::doc_markdown' '--warn=clippy::debug_assert_with_mut_call' '--warn=clippy::dbg_macro' '--warn=clippy::create_dir' '--allow=clippy::collapsible_else_if' '--warn=clippy::checked_conversions' '--allow=clippy::branches_sharing_code' '--allow=clippy::bool_assert_comparison' '--allow=clippy::blocks_in_conditions' '--allow=clippy::assigning_clones' --cfg 'feature="color"' --cfg 'feature="error-context"' --cfg 'feature="help"' --cfg 'feature="std"' --cfg 'feature="suggestions"' --cfg 'feature="usage"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cargo", "color", "debug", "default", "deprecated", "env", "error-context", "help", "std", "string", "suggestions", "unicode", "unstable-doc", "unstable-ext", "unstable-styles", "unstable-v5", "usage", "wrap_help"))' -C metadata=7e089358a67bf4ff -C extra-filename=-5febedd6f7e7d9ae --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern anstream=/home/jelmer/src/janitor/target/debug/deps/libanstream-2daa15f4ec64371e.rmeta --extern anstyle=/home/jelmer/src/janitor/target/debug/deps/libanstyle-3491f347c6e7c6e0.rmeta --extern clap_lex=/home/jelmer/src/janitor/target/debug/deps/libclap_lex-8eff0cda03ec45d2.rmeta --extern strsim=/home/jelmer/src/janitor/target/debug/deps/libstrsim-73001e6240a43464.rmeta --cap-lints allow --cfg tokio_unstable`
       Fresh zerovec v0.10.4
       Fresh phf_codegen v0.10.0
       Fresh string_cache_codegen v0.5.4
       Fresh async-global-executor v2.4.1
       Fresh tinystr v0.7.6
       Fresh icu_collections v1.5.0
       Fresh async-std v1.13.1
       Fresh icu_locid v1.5.0
       Fresh icu_provider v1.5.0
       Fresh markup5ever v0.11.0
       Fresh icu_locid_transform v1.5.0
       Fresh html5ever v0.26.0
       Fresh xml5ever v0.17.0
       Fresh icu_properties v1.5.1
       Fresh markup5ever_rcdom v0.2.0
       Fresh icu_normalizer v1.5.0
       Fresh select v0.6.1
       Fresh idna_adapter v1.2.0
       Fresh idna v1.0.3
       Fresh url v2.5.4
     Running `/home/jelmer/src/janitor/target/debug/build/openssl-sys-d9cad12f5b470f35/build-script-main`
   Compiling curl-sys v0.4.80+curl-8.12.1
   Compiling openssl v0.10.72
   Compiling native-tls v0.2.14
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name openssl_sys --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/openssl-sys-0.9.107/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("aws-lc", "bindgen", "bssl-sys", "openssl-src", "unstable_boringssl", "vendored"))' -C metadata=d4d193d1e09c08a8 -C extra-filename=-f61f9be00a3c1afa --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-5457d7c65e6ff7c6.rmeta --cap-lints allow --cfg tokio_unstable -l ssl -l crypto --cfg 'osslconf="OPENSSL_NO_IDEA"' --cfg 'osslconf="OPENSSL_NO_SSL3_METHOD"' --cfg openssl --cfg ossl340 --cfg ossl330 --cfg ossl320 --cfg ossl300 --cfg ossl101 --cfg ossl102 --cfg ossl102f --cfg ossl102h --cfg ossl110 --cfg ossl110f --cfg ossl110g --cfg ossl110h --cfg ossl111 --cfg ossl111b --cfg ossl111c --cfg ossl111d --check-cfg 'cfg(osslconf, values("OPENSSL_NO_OCB", "OPENSSL_NO_SM4", "OPENSSL_NO_SEED", "OPENSSL_NO_CHACHA", "OPENSSL_NO_CAST", "OPENSSL_NO_IDEA", "OPENSSL_NO_CAMELLIA", "OPENSSL_NO_RC4", "OPENSSL_NO_BF", "OPENSSL_NO_PSK", "OPENSSL_NO_DEPRECATED_3_0", "OPENSSL_NO_SCRYPT", "OPENSSL_NO_SM3", "OPENSSL_NO_RMD160", "OPENSSL_NO_EC2M", "OPENSSL_NO_OCSP", "OPENSSL_NO_CMS", "OPENSSL_NO_COMP", "OPENSSL_NO_SOCK", "OPENSSL_NO_STDIO", "OPENSSL_NO_EC", "OPENSSL_NO_SSL3_METHOD", "OPENSSL_NO_KRB5", "OPENSSL_NO_TLSEXT", "OPENSSL_NO_SRP", "OPENSSL_NO_RFC3779", "OPENSSL_NO_SHA", "OPENSSL_NO_NEXTPROTONEG", "OPENSSL_NO_ENGINE", "OPENSSL_NO_BUF_FREELISTS", "OPENSSL_NO_RC2"))' --check-cfg 'cfg(openssl)' --check-cfg 'cfg(libressl)' --check-cfg 'cfg(boringssl)' --check-cfg 'cfg(awslc)' --check-cfg 'cfg(libressl250)' --check-cfg 'cfg(libressl251)' --check-cfg 'cfg(libressl252)' --check-cfg 'cfg(libressl261)' --check-cfg 'cfg(libressl270)' --check-cfg 'cfg(libressl271)' --check-cfg 'cfg(libressl273)' --check-cfg 'cfg(libressl280)' --check-cfg 'cfg(libressl281)' --check-cfg 'cfg(libressl291)' --check-cfg 'cfg(libressl310)' --check-cfg 'cfg(libressl321)' --check-cfg 'cfg(libressl332)' --check-cfg 'cfg(libressl340)' --check-cfg 'cfg(libressl350)' --check-cfg 'cfg(libressl360)' --check-cfg 'cfg(libressl361)' --check-cfg 'cfg(libressl370)' --check-cfg 'cfg(libressl380)' --check-cfg 'cfg(libressl381)' --check-cfg 'cfg(libressl382)' --check-cfg 'cfg(libressl390)' --check-cfg 'cfg(libressl400)' --check-cfg 'cfg(libressl410)' --check-cfg 'cfg(ossl101)' --check-cfg 'cfg(ossl102)' --check-cfg 'cfg(ossl102f)' --check-cfg 'cfg(ossl102h)' --check-cfg 'cfg(ossl110)' --check-cfg 'cfg(ossl110f)' --check-cfg 'cfg(ossl110g)' --check-cfg 'cfg(ossl110h)' --check-cfg 'cfg(ossl111)' --check-cfg 'cfg(ossl111b)' --check-cfg 'cfg(ossl111c)' --check-cfg 'cfg(ossl111d)' --check-cfg 'cfg(ossl300)' --check-cfg 'cfg(ossl310)' --check-cfg 'cfg(ossl320)' --check-cfg 'cfg(ossl330)' --check-cfg 'cfg(ossl340)'`
     Running `/home/jelmer/src/janitor/target/debug/build/curl-sys-705dee47660d3cbf/build-script-build`
     Running `/home/jelmer/src/janitor/target/debug/build/openssl-a00aa62adb2944e3/build-script-build`
     Running `/home/jelmer/src/janitor/target/debug/build/native-tls-edef69d5d1949eff/build-script-build`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name openssl --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/openssl-0.10.72/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("aws-lc", "bindgen", "default", "unstable_boringssl", "v101", "v102", "v110", "v111", "vendored"))' -C metadata=608e7205c7e0f6d9 -C extra-filename=-6f456163f9b18c32 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern cfg_if=/home/jelmer/src/janitor/target/debug/deps/libcfg_if-c30e1cd0415bac84.rmeta --extern foreign_types=/home/jelmer/src/janitor/target/debug/deps/libforeign_types-0bf9645f98990128.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-5457d7c65e6ff7c6.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern openssl_macros=/home/jelmer/src/janitor/target/debug/deps/libopenssl_macros-89150665c9ae34c2.so --extern ffi=/home/jelmer/src/janitor/target/debug/deps/libopenssl_sys-f61f9be00a3c1afa.rmeta --cap-lints allow --cfg tokio_unstable --cfg 'osslconf="OPENSSL_NO_IDEA"' --cfg 'osslconf="OPENSSL_NO_SSL3_METHOD"' --cfg ossl101 --cfg ossl102 --cfg ossl110 --cfg ossl110g --cfg ossl110h --cfg ossl111 --cfg ossl111d --cfg ossl300 --cfg ossl310 --cfg ossl320 --cfg ossl330 --check-cfg 'cfg(osslconf, values("OPENSSL_NO_OCB", "OPENSSL_NO_SM4", "OPENSSL_NO_SEED", "OPENSSL_NO_CHACHA", "OPENSSL_NO_CAST", "OPENSSL_NO_IDEA", "OPENSSL_NO_CAMELLIA", "OPENSSL_NO_RC4", "OPENSSL_NO_BF", "OPENSSL_NO_PSK", "OPENSSL_NO_DEPRECATED_3_0", "OPENSSL_NO_SCRYPT", "OPENSSL_NO_SM3", "OPENSSL_NO_RMD160", "OPENSSL_NO_EC2M", "OPENSSL_NO_OCSP", "OPENSSL_NO_CMS", "OPENSSL_NO_EC", "OPENSSL_NO_ARGON2", "OPENSSL_NO_RC2"))' --check-cfg 'cfg(libressl)' --check-cfg 'cfg(boringssl)' --check-cfg 'cfg(awslc)' --check-cfg 'cfg(libressl250)' --check-cfg 'cfg(libressl251)' --check-cfg 'cfg(libressl261)' --check-cfg 'cfg(libressl270)' --check-cfg 'cfg(libressl271)' --check-cfg 'cfg(libressl273)' --check-cfg 'cfg(libressl280)' --check-cfg 'cfg(libressl291)' --check-cfg 'cfg(libressl310)' --check-cfg 'cfg(libressl321)' --check-cfg 'cfg(libressl332)' --check-cfg 'cfg(libressl340)' --check-cfg 'cfg(libressl350)' --check-cfg 'cfg(libressl360)' --check-cfg 'cfg(libressl361)' --check-cfg 'cfg(libressl370)' --check-cfg 'cfg(libressl380)' --check-cfg 'cfg(libressl382)' --check-cfg 'cfg(libressl390)' --check-cfg 'cfg(libressl400)' --check-cfg 'cfg(libressl410)' --check-cfg 'cfg(ossl101)' --check-cfg 'cfg(ossl102)' --check-cfg 'cfg(ossl110)' --check-cfg 'cfg(ossl110g)' --check-cfg 'cfg(ossl110h)' --check-cfg 'cfg(ossl111)' --check-cfg 'cfg(ossl111d)' --check-cfg 'cfg(ossl300)' --check-cfg 'cfg(ossl310)' --check-cfg 'cfg(ossl320)' --check-cfg 'cfg(ossl330)'`
   Compiling clap v4.5.36
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name clap --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/clap-4.5.36/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::zero_sized_map_values' '--warn=clippy::verbose_file_reads' --warn=unused_qualifications --warn=unused_macro_rules --warn=unused_lifetimes --warn=unsafe_op_in_unsafe_fn --warn=unreachable_pub '--warn=clippy::uninlined_format_args' '--warn=clippy::trait_duplication_in_bounds' '--warn=clippy::todo' '--warn=clippy::string_lit_as_bytes' '--warn=clippy::string_add_assign' '--warn=clippy::semicolon_if_nothing_returned' '--warn=clippy::self_named_module_files' '--warn=clippy::same_functions_in_if_condition' '--warn=clippy::rest_pat_in_fully_bound_structs' '--warn=clippy::ref_option_ref' '--warn=clippy::redundant_feature_names' '--warn=clippy::rc_mutex' '--warn=clippy::ptr_as_ptr' '--warn=clippy::path_buf_push_overwrite' '--warn=clippy::negative_feature_names' '--warn=clippy::needless_for_each' '--warn=clippy::needless_continue' '--warn=clippy::mutex_integer' '--allow=clippy::multiple_bound_locations' '--warn=clippy::mem_forget' '--warn=clippy::macro_use_imports' '--warn=clippy::lossy_float_literal' '--warn=clippy::linkedlist' '--allow=clippy::let_and_return' '--warn=clippy::large_types_passed_by_value' '--warn=clippy::large_stack_arrays' '--warn=clippy::large_digit_groups' '--warn=clippy::invalid_upcast_comparisons' '--warn=clippy::infinite_loop' '--warn=clippy::inefficient_to_string' '--warn=clippy::inconsistent_struct_constructor' '--warn=clippy::imprecise_flops' '--warn=clippy::implicit_clone' '--allow=clippy::if_same_then_else' '--warn=clippy::from_iter_instead_of_collect' '--warn=clippy::fn_params_excessive_bools' '--warn=clippy::float_cmp_const' '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::fallible_impl_from' '--warn=clippy::explicit_into_iter_loop' '--warn=clippy::explicit_deref_methods' '--warn=clippy::expl_impl_clone_on_copy' '--warn=clippy::enum_glob_use' '--warn=clippy::empty_enum' '--warn=clippy::doc_markdown' '--warn=clippy::debug_assert_with_mut_call' '--warn=clippy::dbg_macro' '--warn=clippy::create_dir' '--allow=clippy::collapsible_else_if' '--warn=clippy::checked_conversions' '--allow=clippy::branches_sharing_code' '--allow=clippy::bool_assert_comparison' '--allow=clippy::blocks_in_conditions' '--allow=clippy::assigning_clones' --cfg 'feature="color"' --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="error-context"' --cfg 'feature="help"' --cfg 'feature="std"' --cfg 'feature="suggestions"' --cfg 'feature="usage"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cargo", "color", "debug", "default", "deprecated", "derive", "env", "error-context", "help", "std", "string", "suggestions", "unicode", "unstable-derive-ui-tests", "unstable-doc", "unstable-ext", "unstable-markdown", "unstable-styles", "unstable-v5", "usage", "wrap_help"))' -C metadata=46437831075c812f -C extra-filename=-4e084c1adea8794a --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern clap_builder=/home/jelmer/src/janitor/target/debug/deps/libclap_builder-5febedd6f7e7d9ae.rmeta --extern clap_derive=/home/jelmer/src/janitor/target/debug/deps/libclap_derive-daf434ff39723ea2.so --cap-lints allow --cfg tokio_unstable`
   Compiling tokio-util v0.7.14
   Compiling tower v0.5.2
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio_util --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-util-0.7.14/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(loom)' --check-cfg 'cfg(mio_unsupported_force_poll_poll)' --check-cfg 'cfg(tokio_allow_from_blocking_fd)' --check-cfg 'cfg(tokio_internal_mt_counters)' --check-cfg 'cfg(tokio_no_parking_lot)' --check-cfg 'cfg(tokio_no_tuning_tests)' --check-cfg 'cfg(tokio_taskdump)' --check-cfg 'cfg(tokio_unstable)' --cfg 'feature="codec"' --cfg 'feature="default"' --cfg 'feature="io"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__docs_rs", "codec", "compat", "default", "full", "futures-io", "futures-util", "hashbrown", "io", "io-util", "net", "rt", "slab", "time", "tracing"))' -C metadata=aac590dc03ba98ec -C extra-filename=-2771d36e1a111c8f --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_sink=/home/jelmer/src/janitor/target/debug/deps/libfutures_sink-0f1aae5d0426fde7.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-9641ac8798494502.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tower --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tower-0.5.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="__common"' --cfg 'feature="futures-core"' --cfg 'feature="futures-util"' --cfg 'feature="pin-project-lite"' --cfg 'feature="sync_wrapper"' --cfg 'feature="timeout"' --cfg 'feature="tokio"' --cfg 'feature="util"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__common", "balance", "buffer", "discover", "filter", "full", "futures-core", "futures-util", "hdrhistogram", "hedge", "indexmap", "limit", "load", "load-shed", "log", "make", "pin-project-lite", "ready-cache", "reconnect", "retry", "slab", "spawn-ready", "steer", "sync_wrapper", "timeout", "tokio", "tokio-stream", "tokio-util", "tracing", "util"))' -C metadata=d051cef10fec7d7b -C extra-filename=-f80a328f3478bb2d --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-d98db11362074d08.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-9641ac8798494502.rmeta --extern tower_layer=/home/jelmer/src/janitor/target/debug/deps/libtower_layer-fff117c1c28e545a.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling h2 v0.4.8
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name h2 --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/h2-0.4.8/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("stream", "unstable"))' -C metadata=b7691e3031f0c920 -C extra-filename=-f77af36903cf4eb5 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern atomic_waker=/home/jelmer/src/janitor/target/debug/deps/libatomic_waker-21f0b624b8878034.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern fnv=/home/jelmer/src/janitor/target/debug/deps/libfnv-135eca59eff18b18.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_sink=/home/jelmer/src/janitor/target/debug/deps/libfutures_sink-0f1aae5d0426fde7.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-36e44d773cc53af4.rmeta --extern slab=/home/jelmer/src/janitor/target/debug/deps/libslab-58feeb60e58ddd09.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-9641ac8798494502.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-2771d36e1a111c8f.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-5541f43ada743dcd.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name native_tls --edition=2015 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/native-tls-0.2.14/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alpn", "vendored"))' -C metadata=962435fb33f075ba -C extra-filename=-8a2c7082e6261fec --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-f00d3747a0e185e9.rmeta --extern openssl=/home/jelmer/src/janitor/target/debug/deps/libopenssl-6f456163f9b18c32.rmeta --extern openssl_probe=/home/jelmer/src/janitor/target/debug/deps/libopenssl_probe-81c031c110cf4218.rmeta --extern openssl_sys=/home/jelmer/src/janitor/target/debug/deps/libopenssl_sys-f61f9be00a3c1afa.rmeta --cap-lints allow --cfg tokio_unstable --cfg have_min_max_version --check-cfg 'cfg(have_min_max_version)'`
   Compiling tokio-native-tls v0.3.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio_native_tls --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-native-tls-0.3.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("vendored"))' -C metadata=42b9c03505ed34a8 -C extra-filename=-fdac86b1109102e9 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-8a2c7082e6261fec.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-9641ac8798494502.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper v1.6.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-1.6.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(hyper_unstable_tracing)' --check-cfg 'cfg(hyper_unstable_ffi)' --cfg 'feature="client"' --cfg 'feature="default"' --cfg 'feature="http1"' --cfg 'feature="http2"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("capi", "client", "default", "ffi", "full", "http1", "http2", "nightly", "server", "tracing"))' -C metadata=28837e402e8f4290 -C extra-filename=-b3c0e53357aa832e --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-d98db11362074d08.rmeta --extern h2=/home/jelmer/src/janitor/target/debug/deps/libh2-f77af36903cf4eb5.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern httparse=/home/jelmer/src/janitor/target/debug/deps/libhttparse-de9e4dfe0f78db23.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-2f50ad762aed9c64.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-9641ac8798494502.rmeta --extern want=/home/jelmer/src/janitor/target/debug/deps/libwant-676b1650d2642fde.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper-util v0.1.10
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper_util --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-util-0.1.10/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="client"' --cfg 'feature="client-legacy"' --cfg 'feature="default"' --cfg 'feature="http1"' --cfg 'feature="http2"' --cfg 'feature="tokio"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__internal_happy_eyeballs_tests", "client", "client-legacy", "default", "full", "http1", "http2", "server", "server-auto", "server-graceful", "service", "tokio"))' -C metadata=0bcf27bf5dac0a91 -C extra-filename=-92fbee128f57ce7d --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-d98db11362074d08.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-b3c0e53357aa832e.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-631c96f5856b2ef3.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-9641ac8798494502.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-5541f43ada743dcd.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper-tls v0.6.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper_tls --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-tls-0.6.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alpn", "vendored"))' -C metadata=cb237507707a6686 -C extra-filename=-6d0d334afa3e9a4a --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-b3c0e53357aa832e.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-92fbee128f57ce7d.rmeta --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-8a2c7082e6261fec.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-9641ac8798494502.rmeta --extern tokio_native_tls=/home/jelmer/src/janitor/target/debug/deps/libtokio_native_tls-fdac86b1109102e9.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling reqwest v0.12.15
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name reqwest --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/reqwest-0.12.15/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(reqwest_unstable)' --cfg 'feature="__tls"' --cfg 'feature="blocking"' --cfg 'feature="charset"' --cfg 'feature="default"' --cfg 'feature="default-tls"' --cfg 'feature="h2"' --cfg 'feature="http2"' --cfg 'feature="json"' --cfg 'feature="macos-system-configuration"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__rustls", "__rustls-ring", "__tls", "blocking", "brotli", "charset", "cookies", "default", "default-tls", "deflate", "gzip", "h2", "hickory-dns", "http2", "http3", "json", "macos-system-configuration", "multipart", "native-tls", "native-tls-alpn", "native-tls-vendored", "rustls-tls", "rustls-tls-manual-roots", "rustls-tls-manual-roots-no-provider", "rustls-tls-native-roots", "rustls-tls-native-roots-no-provider", "rustls-tls-no-provider", "rustls-tls-webpki-roots", "rustls-tls-webpki-roots-no-provider", "socks", "stream", "trust-dns", "zstd"))' -C metadata=5357cf9ceadbae95 -C extra-filename=-bc00594267655746 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-b1a4de7ed4da6927.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern encoding_rs=/home/jelmer/src/janitor/target/debug/deps/libencoding_rs-58c41e4932181cd7.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-d98db11362074d08.rmeta --extern h2=/home/jelmer/src/janitor/target/debug/deps/libh2-f77af36903cf4eb5.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-b3c0e53357aa832e.rmeta --extern hyper_tls=/home/jelmer/src/janitor/target/debug/deps/libhyper_tls-6d0d334afa3e9a4a.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-92fbee128f57ce7d.rmeta --extern ipnet=/home/jelmer/src/janitor/target/debug/deps/libipnet-5873e4e1530bf49f.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-f00d3747a0e185e9.rmeta --extern mime=/home/jelmer/src/janitor/target/debug/deps/libmime-c74000db2aaf511f.rmeta --extern native_tls_crate=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-8a2c7082e6261fec.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern rustls_pemfile=/home/jelmer/src/janitor/target/debug/deps/librustls_pemfile-68bb2d10b5046659.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-636157338e0dea3a.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-e69e443854f2283b.rmeta --extern serde_urlencoded=/home/jelmer/src/janitor/target/debug/deps/libserde_urlencoded-dc12738b33e1eea7.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-9641ac8798494502.rmeta --extern tokio_native_tls=/home/jelmer/src/janitor/target/debug/deps/libtokio_native_tls-fdac86b1109102e9.rmeta --extern tower=/home/jelmer/src/janitor/target/debug/deps/libtower-f80a328f3478bb2d.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b000779e280d7f39.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling curl v0.4.47
   Compiling isahc v1.7.2
     Running `/home/jelmer/src/janitor/target/debug/build/curl-c6732ec821000beb/build-script-build`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name curl_sys --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/curl-sys-0.4.80+curl-8.12.1/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="http2"' --cfg 'feature="libnghttp2-sys"' --cfg 'feature="openssl-sys"' --cfg 'feature="ssl"' --cfg 'feature="static-curl"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "force-system-lib-on-osx", "http2", "libnghttp2-sys", "mesalink", "ntlm", "openssl-sys", "poll_7_68_0", "protocol-ftp", "rustls", "rustls-ffi", "spnego", "ssl", "static-curl", "static-ssl", "upkeep_7_62_0", "windows-static-ssl", "zlib-ng-compat"))' -C metadata=a0e37e592d599196 -C extra-filename=-2290525b080f08ca --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-5457d7c65e6ff7c6.rmeta --extern libnghttp2_sys=/home/jelmer/src/janitor/target/debug/deps/liblibnghttp2_sys-aba2c8079e443356.rmeta --extern libz_sys=/home/jelmer/src/janitor/target/debug/deps/liblibz_sys-5646d94efb319687.rmeta --extern openssl_sys=/home/jelmer/src/janitor/target/debug/deps/libopenssl_sys-f61f9be00a3c1afa.rmeta --cap-lints allow --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/curl-sys-bc36d7082bf9141b/out/build -l static=curl -L native=/home/jelmer/src/janitor/target/debug/build/libnghttp2-sys-b74aa128e9499124/out/i/lib --cfg libcurl_vendored --cfg link_libnghttp2 --cfg link_libz --cfg link_openssl --check-cfg 'cfg(libcurl_vendored,link_libnghttp2,link_libz,link_openssl,)'`
     Running `/home/jelmer/src/janitor/target/debug/build/isahc-05a306b9d3aaa46a/build-script-build`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name curl --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/curl-0.4.47/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="http2"' --cfg 'feature="openssl-probe"' --cfg 'feature="openssl-sys"' --cfg 'feature="ssl"' --cfg 'feature="static-curl"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "force-system-lib-on-osx", "http2", "mesalink", "ntlm", "openssl-probe", "openssl-sys", "poll_7_68_0", "protocol-ftp", "rustls", "spnego", "ssl", "static-curl", "static-ssl", "upkeep_7_62_0", "windows-static-ssl", "zlib-ng-compat"))' -C metadata=3c1a4a9b068881b1 -C extra-filename=-83a34a5a479f73c9 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern curl_sys=/home/jelmer/src/janitor/target/debug/deps/libcurl_sys-2290525b080f08ca.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-5457d7c65e6ff7c6.rmeta --extern openssl_probe=/home/jelmer/src/janitor/target/debug/deps/libopenssl_probe-81c031c110cf4218.rmeta --extern openssl_sys=/home/jelmer/src/janitor/target/debug/deps/libopenssl_sys-f61f9be00a3c1afa.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-631c96f5856b2ef3.rmeta --cap-lints allow --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/curl-sys-bc36d7082bf9141b/out/build -L native=/home/jelmer/src/janitor/target/debug/build/libnghttp2-sys-b74aa128e9499124/out/i/lib --cfg need_openssl_probe --check-cfg 'cfg(need_openssl_init,need_openssl_probe,)'`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name isahc --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/isahc-1.7.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="encoding_rs"' --cfg 'feature="http2"' --cfg 'feature="mime"' --cfg 'feature="static-curl"' --cfg 'feature="text-decoding"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cookies", "default", "encoding_rs", "http2", "httpdate", "json", "mime", "nightly", "parking_lot", "psl", "publicsuffix", "serde", "serde_json", "spnego", "static-curl", "static-ssl", "text-decoding", "unstable-interceptors"))' -C metadata=9e1cbd4153bd5c4e -C extra-filename=-40c53ec8a53d00c8 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_channel=/home/jelmer/src/janitor/target/debug/deps/libasync_channel-bbd57acae1e02279.rmeta --extern castaway=/home/jelmer/src/janitor/target/debug/deps/libcastaway-0fa3a13eb6cf92d0.rmeta --extern crossbeam_utils=/home/jelmer/src/janitor/target/debug/deps/libcrossbeam_utils-b30db245c4fdf551.rmeta --extern curl=/home/jelmer/src/janitor/target/debug/deps/libcurl-83a34a5a479f73c9.rmeta --extern curl_sys=/home/jelmer/src/janitor/target/debug/deps/libcurl_sys-2290525b080f08ca.rmeta --extern encoding_rs=/home/jelmer/src/janitor/target/debug/deps/libencoding_rs-58c41e4932181cd7.rmeta --extern event_listener=/home/jelmer/src/janitor/target/debug/deps/libevent_listener-6a1b983fc3f9caff.rmeta --extern futures_lite=/home/jelmer/src/janitor/target/debug/deps/libfutures_lite-ac5a4e8938ba787f.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-e5cb99eaf31be3ae.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-f00d3747a0e185e9.rmeta --extern mime=/home/jelmer/src/janitor/target/debug/deps/libmime-c74000db2aaf511f.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern polling=/home/jelmer/src/janitor/target/debug/deps/libpolling-5d03b5107544c845.rmeta --extern slab=/home/jelmer/src/janitor/target/debug/deps/libslab-58feeb60e58ddd09.rmeta --extern sluice=/home/jelmer/src/janitor/target/debug/deps/libsluice-a63894663e8473b2.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-5541f43ada743dcd.rmeta --extern tracing_futures=/home/jelmer/src/janitor/target/debug/deps/libtracing_futures-727bd91edbb204ce.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b000779e280d7f39.rmeta --extern waker_fn=/home/jelmer/src/janitor/target/debug/deps/libwaker_fn-59f12e39ebdb57a2.rmeta --cap-lints allow --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/curl-sys-bc36d7082bf9141b/out/build -L native=/home/jelmer/src/janitor/target/debug/build/libnghttp2-sys-b74aa128e9499124/out/i/lib`
   Compiling janitor-mail-filter v0.0.0 (/home/jelmer/src/janitor/mail-filter)
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor_mail_filter --edition=2021 mail-filter/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cmdline"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cmdline", "default"))' -C metadata=c9b81e36870583e7 -C extra-filename=-8ec978d60af920a2 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_std=/home/jelmer/src/janitor/target/debug/deps/libasync_std-75f61bba181cf779.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-4e084c1adea8794a.rmeta --extern isahc=/home/jelmer/src/janitor/target/debug/deps/libisahc-40c53ec8a53d00c8.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-f00d3747a0e185e9.rmeta --extern mailparse=/home/jelmer/src/janitor/target/debug/deps/libmailparse-f823cc812c6be48f.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-bc00594267655746.rmeta --extern select=/home/jelmer/src/janitor/target/debug/deps/libselect-18e0abd32157a5e5.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-e69e443854f2283b.rmeta --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/curl-sys-bc36d7082bf9141b/out/build -L native=/home/jelmer/src/janitor/target/debug/build/libnghttp2-sys-b74aa128e9499124/out/i/lib`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor_mail_filter --edition=2021 mail-filter/src/bin/janitor-mail-filter.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cmdline"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cmdline", "default"))' -C metadata=3043c805e5d877da -C extra-filename=-d0cd74fc703407f5 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_std=/home/jelmer/src/janitor/target/debug/deps/libasync_std-75f61bba181cf779.rlib --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-4e084c1adea8794a.rlib --extern isahc=/home/jelmer/src/janitor/target/debug/deps/libisahc-40c53ec8a53d00c8.rlib --extern janitor_mail_filter=/home/jelmer/src/janitor/target/debug/deps/libjanitor_mail_filter-8ec978d60af920a2.rlib --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-f00d3747a0e185e9.rlib --extern mailparse=/home/jelmer/src/janitor/target/debug/deps/libmailparse-f823cc812c6be48f.rlib --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-bc00594267655746.rlib --extern select=/home/jelmer/src/janitor/target/debug/deps/libselect-18e0abd32157a5e5.rlib --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-e69e443854f2283b.rlib --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/curl-sys-bc36d7082bf9141b/out/build -L native=/home/jelmer/src/janitor/target/debug/build/libnghttp2-sys-b74aa128e9499124/out/i/lib`
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.98s
/usr/lib/python3/dist-packages/setuptools/_distutils/cmd.py:90: SetuptoolsDeprecationWarning: setup.py install is deprecated.
!!

        ********************************************************************************
        Please avoid running ``setup.py`` directly.
        Instead, use pypa/build, pypa/installer or other
        standards-based tools.

        See https://blog.ganssle.io/articles/2021/10/setup-py-deprecated.html for details.
        ********************************************************************************

!!
  self.initialize_options()
Copying rust artifact from target/debug/janitor-mail-filter to build/scripts-3.13/janitor-mail-filter
cargo build --manifest-path worker/Cargo.toml --message-format=json-render-diagnostics -v --features cli debian
       Fresh unicode-ident v1.0.18
       Fresh cfg-if v1.0.0
       Fresh memchr v2.7.4
       Fresh autocfg v1.4.0
       Fresh once_cell v1.21.0
       Fresh value-bag v1.10.0
       Fresh regex-syntax v0.8.5
       Fresh bitflags v2.9.0
       Fresh allocator-api2 v0.2.21
       Fresh pin-project-lite v0.2.16
       Fresh scopeguard v1.2.0
       Fresh futures-core v0.3.31
       Fresh itoa v1.0.15
       Fresh equivalent v1.0.2
       Fresh foldhash v0.1.4
       Fresh version_check v0.9.5
       Fresh shlex v1.3.0
       Fresh futures-io v0.3.31
       Fresh fastrand v2.3.0
       Fresh stable_deref_trait v1.2.0
       Fresh bytes v1.10.1
       Fresh writeable v0.5.5
       Fresh litemap v0.7.5
       Fresh icu_locid_transform_data v1.5.0
       Fresh proc-macro2 v1.0.94
       Fresh hashbrown v0.15.2
       Fresh cc v1.2.16
       Fresh tracing-core v0.1.33
       Fresh percent-encoding v2.3.1
       Fresh icu_properties_data v1.5.0
       Fresh utf16_iter v1.0.5
       Fresh icu_normalizer_data v1.5.0
       Fresh write16 v1.0.0
       Fresh utf8_iter v1.0.4
       Fresh ryu v1.0.20
       Fresh home v0.5.11
       Fresh pin-utils v0.1.0
       Fresh log v0.4.27
       Fresh pkg-config v0.3.32
       Fresh vcpkg v0.2.15
       Fresh futures-task v0.3.31
       Fresh atomic-waker v1.1.2
       Fresh quote v1.0.39
       Fresh libc v0.2.170
       Fresh zerocopy v0.8.23
       Fresh crossbeam-utils v0.8.21
       Fresh parking v2.2.1
       Fresh subtle v2.6.1
       Fresh iana-time-zone v0.1.61
       Fresh tinyvec_macros v0.1.1
       Fresh foreign-types-shared v0.1.1
       Fresh aho-corasick v1.1.3
       Fresh linux-raw-sys v0.9.2
       Fresh siphasher v1.0.1
       Fresh openssl-probe v0.1.6
       Fresh cpufeatures v0.2.17
       Fresh syn v2.0.100
       Fresh lock_api v0.4.12
       Fresh slab v0.4.9
       Fresh ppv-lite86 v0.2.21
       Fresh typenum v1.18.0
       Fresh concurrent-queue v2.5.0
       Fresh signal-hook-registry v1.4.2
       Fresh foreign-types v0.3.2
       Fresh tinyvec v1.9.0
       Fresh phf_shared v0.11.3
       Fresh regex-automata v0.4.9
       Fresh futures-lite v2.6.0
       Fresh zerocopy v0.7.35
       Fresh heck v0.5.0
       Fresh bitflags v1.3.2
       Fresh async-task v4.7.1
       Fresh event-listener v2.5.3
       Fresh serde_derive v1.0.219
       Fresh synstructure v0.13.1
       Fresh thiserror-impl v2.0.12
       Fresh displaydoc v0.2.5
       Fresh zerovec-derive v0.10.3
       Fresh icu_provider_macros v1.5.0
       Fresh tracing-attributes v0.1.28
       Fresh generic-array v0.14.7
       Fresh tokio-macros v2.5.0
       Fresh openssl-macros v0.1.1
       Fresh futures-macro v0.3.31
       Fresh rustix v1.0.2
       Fresh unicode-normalization v0.1.24
       Fresh target-lexicon v0.12.16
       Fresh regex v1.11.1
       Fresh event-listener v5.4.0
       Fresh ahash v0.8.11
       Fresh piper v0.2.4
       Fresh async-executor v1.13.1
       Fresh getrandom v0.2.15
       Fresh linux-raw-sys v0.3.8
   Compiling bstr v1.11.3
       Fresh serde v1.0.219
       Fresh zerofrom-derive v0.1.6
       Fresh yoke-derive v0.7.5
       Fresh thiserror v2.0.12
       Fresh crypto-common v0.1.6
       Fresh block-buffer v0.10.4
       Fresh event-listener-strategy v0.5.3
       Fresh hashbrown v0.14.5
       Fresh fastrand v1.9.0
       Fresh waker-fn v1.2.0
       Fresh tracing v0.1.41
       Fresh async-channel v1.9.0
       Fresh async-lock v2.8.0
       Fresh num-conv v0.1.0
       Fresh unicase v2.8.1
       Fresh crc-catalog v2.4.0
       Fresh same-file v1.0.6
       Fresh linux-raw-sys v0.4.15
       Fresh time-core v0.1.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name bstr --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/bstr-1.11.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="alloc"' --cfg 'feature="default"' --cfg 'feature="std"' --cfg 'feature="unicode"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alloc", "default", "serde", "std", "unicode"))' -C metadata=3de318afd25c5e78 -C extra-filename=-6f503c0062ccbef8 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern regex_automata=/home/jelmer/src/janitor/target/debug/deps/libregex_automata-afc98b980bfba415.rmeta --cap-lints allow --cfg tokio_unstable`
       Fresh zerofrom v0.1.6
       Fresh indexmap v2.8.0
       Fresh serde_json v1.0.140
       Fresh either v1.15.0
       Fresh digest v0.10.7
       Fresh smallvec v1.14.0
       Dirty pyo3-build-config v0.22.6: the env variable PYO3_PYTHON changed
   Compiling pyo3-build-config v0.22.6
       Fresh async-lock v3.4.0
       Fresh async-channel v2.3.1
       Fresh futures-lite v1.13.0
       Fresh fnv v1.0.7
       Fresh powerfmt v0.2.0
       Fresh time-macros v0.2.20
       Fresh walkdir v2.5.0
       Fresh crc v3.2.1
       Fresh thiserror-impl v1.0.69
       Fresh openssl-sys v0.9.107
       Fresh socket2 v0.5.8
       Fresh crossbeam-queue v0.3.12
       Fresh mio v1.0.3
     Running `/home/jelmer/src/janitor/target/debug/build/pyo3-build-config-2bdb8503f35a3e93/build-script-build`
       Fresh yoke v0.7.5
       Fresh parking_lot_core v0.9.10
       Fresh blocking v1.6.1
       Fresh phf_generator v0.11.3
       Fresh deranged v0.3.11
       Fresh sha2 v0.10.8
       Fresh http v1.2.0
       Fresh rustix v0.38.44
       Fresh num-traits v0.2.19
       Fresh thiserror v1.0.69
       Fresh hashlink v0.10.0
       Fresh futures-sink v0.3.31
       Fresh openssl v0.10.72
       Fresh hmac v0.12.1
   Compiling tokio v1.44.2
       Fresh rand_core v0.6.4
       Fresh form_urlencoded v1.2.1
       Fresh unindent v0.2.4
       Fresh unicode-properties v0.1.3
       Fresh mime v0.3.17
       Fresh unicode-bidi v0.3.18
       Fresh indoc v2.0.6
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.44.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(loom)' --check-cfg 'cfg(mio_unsupported_force_poll_poll)' --check-cfg 'cfg(tokio_allow_from_blocking_fd)' --check-cfg 'cfg(tokio_internal_mt_counters)' --check-cfg 'cfg(tokio_no_parking_lot)' --check-cfg 'cfg(tokio_no_tuning_tests)' --check-cfg 'cfg(tokio_taskdump)' --check-cfg 'cfg(tokio_unstable)' --cfg 'feature="bytes"' --cfg 'feature="default"' --cfg 'feature="fs"' --cfg 'feature="io-util"' --cfg 'feature="libc"' --cfg 'feature="mio"' --cfg 'feature="net"' --cfg 'feature="rt"' --cfg 'feature="socket2"' --cfg 'feature="sync"' --cfg 'feature="time"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("bytes", "default", "fs", "full", "io-std", "io-util", "libc", "macros", "mio", "net", "parking_lot", "process", "rt", "rt-multi-thread", "signal", "signal-hook-registry", "socket2", "sync", "test-util", "time", "tokio-macros", "tracing", "windows-sys"))' -C metadata=b8cfdd62bd444dab -C extra-filename=-00820b57743a40c7 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-b68f60cf32f6788d.rmeta --extern mio=/home/jelmer/src/janitor/target/debug/deps/libmio-17184171b9011342.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-4ecefa5141f16dfd.rmeta --cap-lints allow --cfg tokio_unstable`
       Fresh zerovec v0.10.4
       Fresh parking_lot v0.12.3
       Fresh time v0.3.39
       Fresh polling v3.7.4
       Fresh http-body v1.0.1
       Fresh memoffset v0.9.1
       Fresh hex v0.4.3
       Fresh rand_chacha v0.3.1
       Fresh futures-util v0.3.31
       Fresh hkdf v0.12.4
       Fresh chrono v0.4.40
       Fresh stringprep v0.1.5
       Fresh native-tls v0.2.14
       Fresh md-5 v0.10.6
       Fresh io-lifetimes v1.0.11
       Fresh whoami v1.5.2
       Fresh dotenvy v0.15.7
       Fresh tower-service v0.3.3
       Fresh futures-channel v0.3.31
       Fresh polling v2.8.0
       Fresh socket2 v0.4.10
       Fresh kv-log-macro v1.0.7
       Fresh countme v3.0.1
       Fresh tinystr v0.7.6
       Fresh icu_collections v1.5.0
       Fresh async-io v2.4.0
       Fresh rand v0.8.5
       Fresh rustix v0.37.28
       Fresh base64 v0.22.1
       Fresh text-size v1.1.1
       Fresh new_debug_unreachable v1.0.6
       Fresh rustc-hash v1.1.0
       Fresh try-lock v0.2.5
       Fresh http-body-util v0.1.3
       Fresh futures-intrusive v0.5.0
       Fresh encoding_rs v0.8.35
       Fresh sync_wrapper v1.0.2
       Fresh httpdate v1.0.3
       Fresh byteorder v1.5.0
       Fresh tower-layer v0.3.3
       Fresh lazy-regex-proc_macros v3.4.1
       Fresh atoi v2.0.0
       Fresh precomputed-hash v0.1.1
       Fresh gix-trace v0.1.12
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.44.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(loom)' --check-cfg 'cfg(mio_unsupported_force_poll_poll)' --check-cfg 'cfg(tokio_allow_from_blocking_fd)' --check-cfg 'cfg(tokio_internal_mt_counters)' --check-cfg 'cfg(tokio_no_parking_lot)' --check-cfg 'cfg(tokio_no_tuning_tests)' --check-cfg 'cfg(tokio_taskdump)' --check-cfg 'cfg(tokio_unstable)' --cfg 'feature="bytes"' --cfg 'feature="default"' --cfg 'feature="fs"' --cfg 'feature="full"' --cfg 'feature="io-std"' --cfg 'feature="io-util"' --cfg 'feature="libc"' --cfg 'feature="macros"' --cfg 'feature="mio"' --cfg 'feature="net"' --cfg 'feature="parking_lot"' --cfg 'feature="process"' --cfg 'feature="rt"' --cfg 'feature="rt-multi-thread"' --cfg 'feature="signal"' --cfg 'feature="signal-hook-registry"' --cfg 'feature="socket2"' --cfg 'feature="sync"' --cfg 'feature="time"' --cfg 'feature="tokio-macros"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("bytes", "default", "fs", "full", "io-std", "io-util", "libc", "macros", "mio", "net", "parking_lot", "process", "rt", "rt-multi-thread", "signal", "signal-hook-registry", "socket2", "sync", "test-util", "time", "tokio-macros", "tracing", "windows-sys"))' -C metadata=56c149e89c0c885e -C extra-filename=-70d97630dfe17319 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-f96927166a8959d2.rmeta --extern mio=/home/jelmer/src/janitor/target/debug/deps/libmio-4243848b43cf6eaa.rmeta --extern parking_lot=/home/jelmer/src/janitor/target/debug/deps/libparking_lot-014cc28dd9f2a440.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern signal_hook_registry=/home/jelmer/src/janitor/target/debug/deps/libsignal_hook_registry-07869e6e8c107085.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-9be7658adf7d58e8.rmeta --extern tokio_macros=/home/jelmer/src/janitor/target/debug/deps/libtokio_macros-6d6e842210b98dca.so --cap-lints allow --cfg tokio_unstable`
       Fresh icu_locid v1.5.0
       Fresh async-global-executor v2.4.1
       Fresh httparse v1.10.1
       Fresh want v0.3.1
       Fresh async-io v1.13.0
       Fresh rowan v0.16.1
       Fresh lazy-regex v3.4.1
       Fresh siphasher v0.3.11
       Fresh phf_codegen v0.11.3
       Fresh string_cache_codegen v0.5.4
       Fresh serde_urlencoded v0.7.1
   Compiling gix-utils v0.2.0
       Fresh rustls-pki-types v1.11.0
       Fresh adler2 v2.0.0
       Fresh unicode-xid v0.2.6
   Compiling prodash v29.0.1
       Fresh deb822-derive v0.2.0
   Compiling winnow v0.7.3
       Fresh ipnet v2.11.0
       Fresh unicode-width v0.2.0
   Compiling sha1 v0.10.6
       Fresh mac v0.1.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_utils --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-utils-0.2.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("bstr"))' -C metadata=4124fbb233f95320 -C extra-filename=-4fa084b49260f158 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern fastrand=/home/jelmer/src/janitor/target/debug/deps/libfastrand-85fe81c02209319d.rmeta --extern unicode_normalization=/home/jelmer/src/janitor/target/debug/deps/libunicode_normalization-98ea40e9c4e3e320.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name prodash --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/prodash-29.0.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="log"' --cfg 'feature="parking_lot"' --cfg 'feature="progress-tree"' --cfg 'feature="progress-tree-log"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("async-io", "bytesize", "crosstermion", "ctrlc", "dashmap", "default", "futures-core", "futures-lite", "human_format", "is-terminal", "jiff", "local-time", "log", "parking_lot", "progress-log", "progress-tree", "progress-tree-hp-hashmap", "progress-tree-log", "render-line", "render-line-autoconfigure", "render-line-crossterm", "render-tui", "render-tui-crossterm", "signal-hook", "tui", "tui-react", "unicode-segmentation", "unicode-width", "unit-bytes", "unit-duration", "unit-human"))' -C metadata=1890bc5289bf0d57 -C extra-filename=-653078e7c61b7d03 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern parking_lot=/home/jelmer/src/janitor/target/debug/deps/libparking_lot-014cc28dd9f2a440.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name winnow --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winnow-0.7.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::zero_sized_map_values' '--allow=clippy::wildcard_imports' '--warn=clippy::verbose_file_reads' --warn=unused_macro_rules --warn=unused_lifetimes --warn=unsafe_op_in_unsafe_fn --warn=unreachable_pub '--warn=clippy::uninlined_format_args' '--warn=clippy::trait_duplication_in_bounds' '--warn=clippy::todo' '--warn=clippy::string_to_string' '--warn=clippy::string_lit_as_bytes' '--warn=clippy::string_add_assign' '--warn=clippy::string_add' '--warn=clippy::str_to_string' '--warn=clippy::semicolon_if_nothing_returned' '--warn=clippy::same_functions_in_if_condition' '--allow=clippy::result_large_err' '--warn=clippy::rest_pat_in_fully_bound_structs' '--warn=clippy::ref_option_ref' '--warn=clippy::redundant_feature_names' '--warn=clippy::rc_mutex' '--warn=clippy::ptr_as_ptr' '--warn=clippy::path_buf_push_overwrite' '--warn=clippy::negative_feature_names' '--warn=clippy::needless_for_each' '--warn=clippy::needless_continue' '--warn=clippy::mutex_integer' '--warn=clippy::mem_forget' '--warn=clippy::macro_use_imports' '--warn=clippy::lossy_float_literal' '--warn=clippy::linkedlist' '--allow=clippy::let_and_return' '--warn=clippy::large_types_passed_by_value' '--warn=clippy::large_stack_arrays' '--warn=clippy::large_digit_groups' '--warn=clippy::invalid_upcast_comparisons' '--warn=clippy::infinite_loop' '--warn=clippy::inefficient_to_string' '--warn=clippy::inconsistent_struct_constructor' '--warn=clippy::imprecise_flops' '--warn=clippy::implicit_clone' '--allow=clippy::if_same_then_else' '--warn=clippy::from_iter_instead_of_collect' '--warn=clippy::fn_params_excessive_bools' '--warn=clippy::float_cmp_const' '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::fallible_impl_from' '--warn=clippy::explicit_into_iter_loop' '--warn=clippy::explicit_deref_methods' '--warn=clippy::expl_impl_clone_on_copy' '--warn=clippy::enum_glob_use' '--warn=clippy::empty_enum' '--warn=clippy::doc_markdown' '--warn=clippy::debug_assert_with_mut_call' '--warn=clippy::dbg_macro' '--warn=clippy::create_dir' '--allow=clippy::collapsible_else_if' '--warn=clippy::checked_conversions' '--allow=clippy::branches_sharing_code' '--allow=clippy::bool_assert_comparison' --cfg 'feature="alloc"' --cfg 'feature="default"' --cfg 'feature="simd"' --cfg 'feature="std"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alloc", "debug", "default", "simd", "std", "unstable-doc", "unstable-recover"))' -C metadata=46d479221b6ee196 -C extra-filename=-41cc7c75049a70dd --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --cap-lints allow --cfg tokio_unstable`
       Fresh icu_provider v1.5.0
       Fresh async-std v1.13.1
       Fresh getrandom v0.3.1
       Fresh syn v1.0.109
       Fresh rustls-pemfile v2.2.0
       Fresh miniz_oxide v0.8.5
       Fresh crunchy v0.2.3
       Fresh utf8parse v0.2.2
   Compiling jiff v0.2.4
       Fresh futf v0.1.5
       Fresh async-trait v0.1.88
       Fresh filetime v0.2.25
       Fresh anstyle v1.0.10
       Fresh is_terminal_polyfill v1.70.1
       Fresh anstyle-query v1.1.2
       Fresh utf-8 v0.7.6
       Fresh colorchoice v1.0.3
       Fresh ucd-trie v0.1.7
       Fresh lazy_static v1.5.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sha1 --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sha1-0.10.6/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="compress"' --cfg 'feature="default"' --cfg 'feature="std"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("asm", "compress", "default", "force-soft", "loongarch64_asm", "oid", "sha1-asm", "std"))' -C metadata=c0be47f7cee60c6a -C extra-filename=-b17c4ac71af9bf14 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern cfg_if=/home/jelmer/src/janitor/target/debug/deps/libcfg_if-c30e1cd0415bac84.rmeta --extern cpufeatures=/home/jelmer/src/janitor/target/debug/deps/libcpufeatures-ffb8d7c2fdf10c54.rmeta --extern digest=/home/jelmer/src/janitor/target/debug/deps/libdigest-beac815ccfdce274.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name jiff --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/jiff-0.2.4/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="alloc"' --cfg 'feature="default"' --cfg 'feature="std"' --cfg 'feature="tz-fat"' --cfg 'feature="tz-system"' --cfg 'feature="tzdb-bundle-platform"' --cfg 'feature="tzdb-concatenated"' --cfg 'feature="tzdb-zoneinfo"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alloc", "default", "js", "logging", "serde", "static", "static-tz", "std", "tz-fat", "tz-system", "tzdb-bundle-always", "tzdb-bundle-platform", "tzdb-concatenated", "tzdb-zoneinfo"))' -C metadata=65b9544993e1efd4 -C extra-filename=-199e5d92b021876a --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --cap-lints allow --cfg tokio_unstable`
       Fresh icu_locid_transform v1.5.0
       Fresh anyhow v1.0.97
       Fresh tiny-keccak v2.0.2
       Fresh tempfile v3.19.0
       Fresh anstyle-parse v0.2.6
       Fresh phf_generator v0.10.0
       Fresh pest v2.7.15
       Fresh tendril v0.4.3
       Fresh inotify-sys v0.1.5
       Fresh dirs-sys-next v0.1.2
       Fresh crc32fast v1.4.2
       Fresh smawk v0.3.2
       Fresh unicode-linebreak v0.1.5
       Fresh gimli v0.31.1
       Fresh synstructure v0.12.6
       Fresh phf_shared v0.10.0
       Fresh serde_spanned v0.6.8
       Fresh toml_datetime v0.6.8
       Fresh crossbeam-channel v0.5.15
       Fresh icu_properties v1.5.1
       Fresh phf_codegen v0.10.0
       Fresh anstream v0.6.18
       Fresh inotify v0.9.6
       Fresh object v0.36.7
       Fresh textwrap v0.16.2
       Fresh dirs-next v2.0.0
       Fresh addr2line v0.24.2
       Fresh pest_meta v2.7.15
       Fresh mio v0.8.11
       Fresh rustc-demangle v0.1.24
       Fresh unsafe-libyaml v0.2.11
       Fresh dtor-proc-macro v0.0.5
       Fresh strsim v0.11.1
       Fresh clap_lex v0.7.4
       Fresh urlencoding v2.1.3
       Fresh flate2 v1.1.0
       Fresh phf v0.10.1
       Fresh rowan v0.15.16
       Fresh icu_normalizer v1.5.0
       Fresh term v0.7.0
       Fresh pest_generator v2.7.15
       Fresh dtor v0.0.5
       Fresh failure_derive v0.1.8
       Fresh backtrace v0.3.74
       Fresh notify v6.1.1
       Fresh serde_yaml v0.9.34+deprecated
       Fresh clap_builder v4.5.36
       Fresh protobuf-support v3.7.2
       Fresh version-ranges v0.1.1
       Fresh which v4.4.2
       Fresh faster-hex v0.9.0
       Fresh parse-zoneinfo v0.3.1
       Fresh phf v0.11.3
       Fresh clap_derive v4.5.32
       Fresh csv-core v0.1.12
       Fresh idna_adapter v1.2.0
       Fresh maplit v1.0.2
       Fresh deunicode v1.6.0
       Fresh unscanny v0.1.0
       Fresh ctor-proc-macro v0.0.5
       Fresh fixedbitset v0.4.2
       Fresh difflib v0.4.0
       Fresh pest_derive v2.7.15
       Fresh psm v0.1.25
       Fresh ascii-canvas v3.0.0
       Fresh dirty-tracker v0.3.0
       Fresh chrono-tz-build v0.3.0
       Fresh protobuf v3.7.2
       Fresh clap v4.5.36
       Fresh csv v1.3.1
       Fresh libm v0.2.11
       Fresh failure v0.1.8
       Fresh idna v1.0.3
       Fresh slug v0.1.6
       Fresh petgraph v0.6.5
       Fresh markup5ever v0.11.0
       Fresh pep440_rs v0.7.3
       Fresh ctor v0.4.1
       Fresh const-random-macro v0.1.16
       Fresh patchkit v0.2.1
       Fresh string_cache v0.8.8
       Fresh charset v0.1.5
       Fresh itertools v0.13.0
       Fresh itertools v0.10.5
       Fresh num-integer v0.1.46
       Fresh is-terminal v0.4.16
       Fresh crossbeam-epoch v0.9.18
       Fresh url v2.5.4
       Fresh ena v0.14.3
       Fresh diff v0.1.13
       Fresh quoted_printable v0.5.1
       Fresh boxcar v0.2.10
       Fresh base64ct v1.7.1
       Fresh unic-char-range v0.9.0
       Fresh regex-syntax v0.6.29
       Fresh rustc-hash v2.1.1
       Fresh xml-rs v0.8.25
       Fresh unic-common v0.9.0
       Fresh minimal-lexical v0.2.1
       Fresh num-bigint v0.4.6
       Fresh const-random v0.1.18
       Fresh semver v1.0.26
       Fresh protobuf v2.28.0
       Fresh crossbeam-deque v0.8.6
       Fresh humansize v2.1.3
       Fresh unic-char-property v0.9.0
       Fresh lalrpop v0.19.12
       Fresh pep508_rs v0.9.2
       Fresh pem-rfc7468 v0.7.0
       Fresh nom v7.1.3
       Fresh unic-ucd-version v0.9.0
       Fresh distro-info v0.4.0
       Fresh stacker v0.1.19
       Fresh makefile-lossless v0.1.7
       Fresh env_filter v0.1.3
       Fresh atty v0.2.14
       Fresh zeroize v1.8.1
       Fresh configparser v3.1.0
       Fresh bit-vec v0.8.0
       Fresh bumpalo v3.17.0
       Fresh quick-error v1.2.3
       Fresh termcolor v1.4.1
       Fresh const-oid v0.9.6
       Fresh untrusted v0.9.0
       Fresh simd-adler32 v0.3.7
       Fresh data-encoding v2.8.0
       Fresh lockfree-object-pool v0.1.6
       Fresh chumsky v0.9.3
       Fresh rustversion v1.0.20
       Fresh askama_parser v0.2.1
       Fresh der v0.7.9
       Fresh unic-ucd-segment v0.9.0
       Fresh bit-set v0.8.0
       Fresh humantime v1.3.0
       Fresh dlv-list v0.5.2
       Fresh protobuf-codegen v2.28.0
       Fresh document_tree v0.4.1
       Fresh mime_guess v2.0.5
       Fresh simple_asn1 v0.6.3
       Fresh html5ever v0.26.0
       Fresh ring v0.17.13
       Fresh zopfli v0.8.1
       Fresh mailparse v0.15.0
       Fresh xml5ever v0.17.0
       Fresh merge3 v0.2.0
       Fresh protobuf-parse v3.7.2
       Fresh protoc v2.28.0
       Fresh rand_core v0.9.3
       Fresh basic-toml v0.1.10
       Fresh futures-executor v0.3.31
       Fresh xattr v1.5.0
       Fresh pem v3.0.5
       Fresh memmap2 v0.9.5
       Fresh cfg_aliases v0.2.1
       Fresh unicode-width v0.1.14
       Fresh unicode_categories v0.1.1
       Fresh typed-arena v2.0.2
       Fresh bit-vec v0.6.3
       Fresh entities v1.0.1
       Fresh chrono-tz v0.9.0
       Fresh env_logger v0.7.1
       Fresh jsonwebtoken v9.3.1
       Fresh askama_derive v0.12.5
       Fresh bit-set v0.5.3
       Fresh tar v0.4.44
       Fresh futures v0.3.31
       Fresh protobuf-codegen v3.7.2
       Fresh comrak v0.18.0
       Fresh getopts v0.2.21
       Fresh markup5ever_rcdom v0.2.0
   Compiling protoc-rust v2.28.0
       Fresh zip v2.4.1
       Fresh rand_chacha v0.9.0
       Fresh rfc2047-decoder v1.0.6
       Fresh axum-core v0.4.5
       Fresh fs-err v3.1.0
       Fresh markup5ever v0.14.1
       Fresh spki v0.7.3
       Fresh ordered-multimap v0.7.3
       Fresh unic-segment v0.9.0
       Fresh fancy-regex v0.14.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name protoc_rust --edition=2015 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/protoc-rust-2.28.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=e3245a377035f95b -C extra-filename=-606fffa627b5d769 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-98c17e40d8ea6aed.rmeta --extern protobuf_codegen=/home/jelmer/src/janitor/target/debug/deps/libprotobuf_codegen-13f80eb668e85f15.rmeta --extern protoc=/home/jelmer/src/janitor/target/debug/deps/libprotoc-7229da44e6aae22b.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-5cc214a3774c4b08.rmeta --cap-lints allow --cfg tokio_unstable`
       Fresh xmltree v0.11.0
       Fresh serde-xml-rs v0.5.1
       Fresh google-cloud-token v0.1.2
       Fresh match_token v0.1.0
       Fresh async-stream-impl v0.3.6
       Fresh m_lexer v0.0.4
   Compiling gix-sec v0.10.12
       Fresh static_assertions v1.1.0
       Fresh unicode-bom v2.0.3
       Fresh base64 v0.21.7
       Fresh lalrpop-util v0.19.12
       Fresh trim-in-place v0.1.7
       Fresh humantime v2.1.0
       Fresh inventory v0.3.20
       Fresh askama_escape v0.10.3
       Fresh pulldown-cmark-escape v0.11.0
       Fresh option-ext v0.2.0
       Fresh python-pkginfo v0.6.5
       Fresh rand v0.9.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_sec --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-sec-0.10.12/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("document-features", "serde"))' -C metadata=749b76bf612ee92b -C extra-filename=-2f1846542ec3dfa0 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-f96927166a8959d2.rmeta --cap-lints allow --cfg tokio_unstable`
       Fresh rust-ini v0.21.1
       Fresh askama v0.12.1
       Fresh env_logger v0.9.3
       Fresh async-stream v0.3.6
       Fresh twox-hash v1.6.3
       Fresh rst_renderer v0.4.1
       Fresh html5ever v0.29.1
       Fresh pulldown-cmark v0.13.0
       Fresh opam-file-rs v0.1.5
       Fresh dirs-sys v0.4.1
       Fresh pkcs8 v0.10.2
       Fresh select v0.6.1
       Fresh pretty_env_logger v0.4.0
       Fresh uo_rst_parser v0.4.3
       Fresh toml v0.5.11
       Fresh serde_path_to_error v0.1.17
       Fresh xdg v2.5.2
       Fresh matchit v0.7.3
       Fresh nix v0.29.0
       Fresh lz4_flex v0.11.3
       Fresh dirs v5.0.1
       Fresh stackdriver_logger v0.8.2
       Fresh lzma-rs v0.3.0
       Fresh instant v0.1.13
       Fresh fs_extra v1.3.0
       Fresh arc-swap v1.7.1
       Fresh askama_axum v0.4.0
       Fresh gethostname v1.0.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name pyo3_build_config --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pyo3-build-config-0.22.6/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no --cfg 'feature="default"' --cfg 'feature="resolve-config"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("abi3", "abi3-py310", "abi3-py311", "abi3-py312", "abi3-py37", "abi3-py38", "abi3-py39", "default", "extension-module", "python3-dll-a", "resolve-config"))' -C metadata=b5a24f29c71761d6 -C extra-filename=-c3380da3e85618ca --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern target_lexicon=/home/jelmer/src/janitor/target/debug/deps/libtarget_lexicon-3ec15ada12adecdb.rmeta --cap-lints allow --cfg tokio_unstable`
       Dirty pyo3-macros-backend v0.22.6: dependency info changed
   Compiling pyo3-macros-backend v0.22.6
       Dirty pyo3-ffi v0.22.6: dependency info changed
   Compiling pyo3-ffi v0.22.6
       Dirty pyo3 v0.22.6: dependency info changed
   Compiling pyo3 v0.22.6
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name build_script_build --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pyo3-macros-backend-0.22.6/build.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no --warn=rust_2018_idioms '--warn=clippy::useless_transmute' '--warn=clippy::used_underscore_binding' --warn=unused_lifetimes '--warn=clippy::unnecessary_wraps' '--warn=clippy::todo' --warn=rust_2021_prelude_collisions '--warn=clippy::manual_ok_or' '--warn=clippy::manual_assert' '--warn=clippy::let_unit_value' --warn=invalid_doc_attributes '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::explicit_iter_loop' '--warn=clippy::explicit_into_iter_loop' --warn=elided_lifetimes_in_paths '--warn=clippy::dbg_macro' '--warn=clippy::checked_conversions' '--warn=rustdoc::broken_intra_doc_links' '--warn=rustdoc::bare_urls' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("experimental-async", "gil-refs"))' -C metadata=0cfc5882855303fb -C extra-filename=-79e46e6e8723193a --out-dir /home/jelmer/src/janitor/target/debug/build/pyo3-macros-backend-79e46e6e8723193a -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern pyo3_build_config=/home/jelmer/src/janitor/target/debug/deps/libpyo3_build_config-c3380da3e85618ca.rlib --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name build_script_build --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pyo3-ffi-0.22.6/build.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no --warn=rust_2018_idioms '--warn=clippy::useless_transmute' '--warn=clippy::used_underscore_binding' --warn=unused_lifetimes '--warn=clippy::unnecessary_wraps' '--warn=clippy::todo' --warn=rust_2021_prelude_collisions '--warn=clippy::manual_ok_or' '--warn=clippy::manual_assert' '--warn=clippy::let_unit_value' --warn=invalid_doc_attributes '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::explicit_iter_loop' '--warn=clippy::explicit_into_iter_loop' --warn=elided_lifetimes_in_paths '--warn=clippy::dbg_macro' '--warn=clippy::checked_conversions' '--warn=rustdoc::broken_intra_doc_links' '--warn=rustdoc::bare_urls' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("abi3", "abi3-py310", "abi3-py311", "abi3-py312", "abi3-py37", "abi3-py38", "abi3-py39", "default", "extension-module", "generate-import-lib"))' -C metadata=69904315c4c50a8e -C extra-filename=-521faf4b33134f2d --out-dir /home/jelmer/src/janitor/target/debug/build/pyo3-ffi-521faf4b33134f2d -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern pyo3_build_config=/home/jelmer/src/janitor/target/debug/deps/libpyo3_build_config-c3380da3e85618ca.rlib --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name build_script_build --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pyo3-0.22.6/build.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no --warn=rust_2018_idioms '--warn=clippy::useless_transmute' '--warn=clippy::used_underscore_binding' --warn=unused_lifetimes '--warn=clippy::unnecessary_wraps' '--warn=clippy::todo' --warn=rust_2021_prelude_collisions '--warn=clippy::manual_ok_or' '--warn=clippy::manual_assert' '--warn=clippy::let_unit_value' --warn=invalid_doc_attributes '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::explicit_iter_loop' '--warn=clippy::explicit_into_iter_loop' --warn=elided_lifetimes_in_paths '--warn=clippy::dbg_macro' '--warn=clippy::checked_conversions' '--warn=rustdoc::broken_intra_doc_links' '--warn=rustdoc::bare_urls' --cfg 'feature="auto-initialize"' --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="indoc"' --cfg 'feature="macros"' --cfg 'feature="pyo3-macros"' --cfg 'feature="serde"' --cfg 'feature="unindent"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("abi3", "abi3-py310", "abi3-py311", "abi3-py312", "abi3-py37", "abi3-py38", "abi3-py39", "anyhow", "auto-initialize", "chrono", "chrono-tz", "default", "either", "experimental-async", "experimental-inspect", "extension-module", "eyre", "full", "generate-import-lib", "gil-refs", "hashbrown", "indexmap", "indoc", "inventory", "macros", "multiple-pymethods", "nightly", "num-bigint", "num-complex", "num-rational", "py-clone", "pyo3-macros", "rust_decimal", "serde", "smallvec", "unindent"))' -C metadata=6a00c75d20f231b9 -C extra-filename=-4f5904c7f4acb758 --out-dir /home/jelmer/src/janitor/target/debug/build/pyo3-4f5904c7f4acb758 -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern pyo3_build_config=/home/jelmer/src/janitor/target/debug/deps/libpyo3_build_config-c3380da3e85618ca.rlib --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/src/janitor/target/debug/build/pyo3-macros-backend-79e46e6e8723193a/build-script-build`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name pyo3_macros_backend --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pyo3-macros-backend-0.22.6/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no --warn=rust_2018_idioms '--warn=clippy::useless_transmute' '--warn=clippy::used_underscore_binding' --warn=unused_lifetimes '--warn=clippy::unnecessary_wraps' '--warn=clippy::todo' --warn=rust_2021_prelude_collisions '--warn=clippy::manual_ok_or' '--warn=clippy::manual_assert' '--warn=clippy::let_unit_value' --warn=invalid_doc_attributes '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::explicit_iter_loop' '--warn=clippy::explicit_into_iter_loop' --warn=elided_lifetimes_in_paths '--warn=clippy::dbg_macro' '--warn=clippy::checked_conversions' '--warn=rustdoc::broken_intra_doc_links' '--warn=rustdoc::bare_urls' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("experimental-async", "gil-refs"))' -C metadata=571502cdf13ab7d8 -C extra-filename=-145c1c11a7f3a990 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern heck=/home/jelmer/src/janitor/target/debug/deps/libheck-4d6a9c8516811f18.rmeta --extern proc_macro2=/home/jelmer/src/janitor/target/debug/deps/libproc_macro2-a7e2001652539cec.rmeta --extern pyo3_build_config=/home/jelmer/src/janitor/target/debug/deps/libpyo3_build_config-c3380da3e85618ca.rmeta --extern quote=/home/jelmer/src/janitor/target/debug/deps/libquote-8533776b6f1db290.rmeta --extern syn=/home/jelmer/src/janitor/target/debug/deps/libsyn-7fe0b75e1b133791.rmeta --cap-lints allow --cfg tokio_unstable --cfg invalid_from_utf8_lint --cfg c_str_lit --cfg diagnostic_namespace --check-cfg 'cfg(Py_LIMITED_API)' --check-cfg 'cfg(PyPy)' --check-cfg 'cfg(GraalPy)' --check-cfg 'cfg(py_sys_config, values("Py_DEBUG", "Py_REF_DEBUG", "Py_TRACE_REFS", "COUNT_ALLOCS"))' --check-cfg 'cfg(invalid_from_utf8_lint)' --check-cfg 'cfg(pyo3_disable_reference_pool)' --check-cfg 'cfg(pyo3_leak_on_drop_without_reference_pool)' --check-cfg 'cfg(diagnostic_namespace)' --check-cfg 'cfg(c_str_lit)' --check-cfg 'cfg(Py_3_7)' --check-cfg 'cfg(Py_3_8)' --check-cfg 'cfg(Py_3_9)' --check-cfg 'cfg(Py_3_10)' --check-cfg 'cfg(Py_3_11)' --check-cfg 'cfg(Py_3_12)' --check-cfg 'cfg(Py_3_13)'`
     Running `/home/jelmer/src/janitor/target/debug/build/pyo3-ffi-521faf4b33134f2d/build-script-build`
   Compiling sha1-checked v0.10.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sha1_checked --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sha1-checked-0.10.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "oid", "std", "zeroize"))' -C metadata=b9c9fa693cdfde38 -C extra-filename=-bc3dee7938bca6a3 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern digest=/home/jelmer/src/janitor/target/debug/deps/libdigest-beac815ccfdce274.rmeta --extern sha1=/home/jelmer/src/janitor/target/debug/deps/libsha1-b17c4ac71af9bf14.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/src/janitor/target/debug/build/pyo3-4f5904c7f4acb758/build-script-build`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name pyo3_ffi --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pyo3-ffi-0.22.6/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::useless_transmute' '--warn=clippy::used_underscore_binding' --warn=unused_lifetimes '--warn=clippy::unnecessary_wraps' '--warn=clippy::todo' --warn=rust_2021_prelude_collisions '--warn=clippy::manual_ok_or' '--warn=clippy::manual_assert' '--warn=clippy::let_unit_value' --warn=invalid_doc_attributes '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::explicit_iter_loop' '--warn=clippy::explicit_into_iter_loop' --warn=elided_lifetimes_in_paths '--warn=clippy::dbg_macro' '--warn=clippy::checked_conversions' '--warn=rustdoc::broken_intra_doc_links' '--warn=rustdoc::bare_urls' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("abi3", "abi3-py310", "abi3-py311", "abi3-py312", "abi3-py37", "abi3-py38", "abi3-py39", "default", "extension-module", "generate-import-lib"))' -C metadata=d32a8b72bc93de32 -C extra-filename=-b222fe21b5e32384 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-f96927166a8959d2.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -l python3.13 --cfg Py_3_6 --cfg Py_3_7 --cfg Py_3_8 --cfg Py_3_9 --cfg Py_3_10 --cfg Py_3_11 --cfg Py_3_12 --cfg Py_3_13 --cfg invalid_from_utf8_lint --cfg c_str_lit --cfg diagnostic_namespace --check-cfg 'cfg(Py_LIMITED_API)' --check-cfg 'cfg(PyPy)' --check-cfg 'cfg(GraalPy)' --check-cfg 'cfg(py_sys_config, values("Py_DEBUG", "Py_REF_DEBUG", "Py_TRACE_REFS", "COUNT_ALLOCS"))' --check-cfg 'cfg(invalid_from_utf8_lint)' --check-cfg 'cfg(pyo3_disable_reference_pool)' --check-cfg 'cfg(pyo3_leak_on_drop_without_reference_pool)' --check-cfg 'cfg(diagnostic_namespace)' --check-cfg 'cfg(c_str_lit)' --check-cfg 'cfg(Py_3_7)' --check-cfg 'cfg(Py_3_8)' --check-cfg 'cfg(Py_3_9)' --check-cfg 'cfg(Py_3_10)' --check-cfg 'cfg(Py_3_11)' --check-cfg 'cfg(Py_3_12)' --check-cfg 'cfg(Py_3_13)'`
   Compiling janitor v0.1.0 (/home/jelmer/src/janitor)
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name build_script_build --edition=2021 build.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="gcp"' --cfg 'feature="gcs"' --cfg 'feature="stackdriver_logger"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("debian", "default", "gcp", "gcs", "stackdriver_logger"))' -C metadata=42013b6ea93bac02 -C extra-filename=-3e4912ad33cb41bf --out-dir /home/jelmer/src/janitor/target/debug/build/janitor-3e4912ad33cb41bf -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern protobuf_codegen=/home/jelmer/src/janitor/target/debug/deps/libprotobuf_codegen-1954bdb467952b44.rlib --extern protoc_rust=/home/jelmer/src/janitor/target/debug/deps/libprotoc_rust-606fffa627b5d769.rlib --cfg tokio_unstable`
   Compiling gix-path v0.10.15
   Compiling gix-validate v0.9.4
   Compiling globset v0.4.16
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name globset --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/globset-0.4.16/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="log"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "log", "serde", "serde1", "simd-accel"))' -C metadata=047953162eb3c2e5 -C extra-filename=-0f95614837bfa1f9 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern aho_corasick=/home/jelmer/src/janitor/target/debug/deps/libaho_corasick-ffa8bcaabf3f32e4.rmeta --extern bstr=/home/jelmer/src/janitor/target/debug/deps/libbstr-6f503c0062ccbef8.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern regex_automata=/home/jelmer/src/janitor/target/debug/deps/libregex_automata-afc98b980bfba415.rmeta --extern regex_syntax=/home/jelmer/src/janitor/target/debug/deps/libregex_syntax-525eb983a84c169f.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_path --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-path-0.10.15/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=5a16034a0e11bdac -C extra-filename=-3bb66e4cb32aadec --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bstr=/home/jelmer/src/janitor/target/debug/deps/libbstr-6f503c0062ccbef8.rmeta --extern gix_trace=/home/jelmer/src/janitor/target/debug/deps/libgix_trace-b8139ff95a034a4d.rmeta --extern home=/home/jelmer/src/janitor/target/debug/deps/libhome-fc8b76e8d1b394eb.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_validate --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-validate-0.9.4/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=a4dfe859963c180f -C extra-filename=-7654a34ba17d943c --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bstr=/home/jelmer/src/janitor/target/debug/deps/libbstr-6f503c0062ccbef8.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling gix-features v0.41.1
   Compiling gix-config-value v0.14.12
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_features --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-features-0.41.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --cfg 'feature="default"' --cfg 'feature="fs-read-dir"' --cfg 'feature="prodash"' --cfg 'feature="progress"' --cfg 'feature="walkdir"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cache-efficiency-debug", "crc32", "default", "document-features", "fs-read-dir", "io-pipe", "once_cell", "parallel", "prodash", "progress", "progress-unit-bytes", "progress-unit-human-numbers", "tracing", "tracing-detail", "walkdir", "zlib", "zlib-ng", "zlib-ng-compat", "zlib-rs", "zlib-rust-backend", "zlib-stock"))' -C metadata=f9447e27a9dc21ce -C extra-filename=-fd3d337f1c519864 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern gix_path=/home/jelmer/src/janitor/target/debug/deps/libgix_path-3bb66e4cb32aadec.rmeta --extern gix_trace=/home/jelmer/src/janitor/target/debug/deps/libgix_trace-b8139ff95a034a4d.rmeta --extern gix_utils=/home/jelmer/src/janitor/target/debug/deps/libgix_utils-4fa084b49260f158.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-f96927166a8959d2.rmeta --extern prodash=/home/jelmer/src/janitor/target/debug/deps/libprodash-653078e7c61b7d03.rmeta --extern walkdir=/home/jelmer/src/janitor/target/debug/deps/libwalkdir-f95d3688eab8bd63.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_config_value --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-config-value-0.14.12/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("document-features", "serde"))' -C metadata=13ce59f8d840e74e -C extra-filename=-9118627372b1dcb7 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern bstr=/home/jelmer/src/janitor/target/debug/deps/libbstr-6f503c0062ccbef8.rmeta --extern gix_path=/home/jelmer/src/janitor/target/debug/deps/libgix_path-3bb66e4cb32aadec.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-f96927166a8959d2.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling ignore v0.4.23
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name ignore --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ignore-0.4.23/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("simd-accel"))' -C metadata=b1145666174b925f -C extra-filename=-2893183fe12503e9 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern crossbeam_deque=/home/jelmer/src/janitor/target/debug/deps/libcrossbeam_deque-214c0f5ee3a6e014.rmeta --extern globset=/home/jelmer/src/janitor/target/debug/deps/libglobset-0f95614837bfa1f9.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern regex_automata=/home/jelmer/src/janitor/target/debug/deps/libregex_automata-afc98b980bfba415.rmeta --extern same_file=/home/jelmer/src/janitor/target/debug/deps/libsame_file-86f44548f9281b53.rmeta --extern walkdir=/home/jelmer/src/janitor/target/debug/deps/libwalkdir-f95d3688eab8bd63.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling gix-hash v0.17.0
   Compiling gix-fs v0.14.0
   Compiling gix-glob v0.19.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_hash --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-hash-0.17.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("document-features", "serde"))' -C metadata=dd5afdbf080398cc -C extra-filename=-75921820cfc67c8b --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern faster_hex=/home/jelmer/src/janitor/target/debug/deps/libfaster_hex-d0341edac5596947.rmeta --extern gix_features=/home/jelmer/src/janitor/target/debug/deps/libgix_features-fd3d337f1c519864.rmeta --extern sha1_checked=/home/jelmer/src/janitor/target/debug/deps/libsha1_checked-bc3dee7938bca6a3.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_glob --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-glob-0.19.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("document-features", "serde"))' -C metadata=6e42db39e6fff057 -C extra-filename=-ca729eeb73408c31 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern bstr=/home/jelmer/src/janitor/target/debug/deps/libbstr-6f503c0062ccbef8.rmeta --extern gix_features=/home/jelmer/src/janitor/target/debug/deps/libgix_features-fd3d337f1c519864.rmeta --extern gix_path=/home/jelmer/src/janitor/target/debug/deps/libgix_path-3bb66e4cb32aadec.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_fs --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-fs-0.14.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("serde"))' -C metadata=af1551a7c76da560 -C extra-filename=-65489ace18893f19 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bstr=/home/jelmer/src/janitor/target/debug/deps/libbstr-6f503c0062ccbef8.rmeta --extern fastrand=/home/jelmer/src/janitor/target/debug/deps/libfastrand-85fe81c02209319d.rmeta --extern gix_features=/home/jelmer/src/janitor/target/debug/deps/libgix_features-fd3d337f1c519864.rmeta --extern gix_path=/home/jelmer/src/janitor/target/debug/deps/libgix_path-3bb66e4cb32aadec.rmeta --extern gix_utils=/home/jelmer/src/janitor/target/debug/deps/libgix_utils-4fa084b49260f158.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling gix-tempfile v17.0.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_tempfile --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-tempfile-17.0.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "document-features", "hp-hashmap", "signals"))' -C metadata=e6d11c6596e46fb7 -C extra-filename=-e2da7605123b169f --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern gix_fs=/home/jelmer/src/janitor/target/debug/deps/libgix_fs-65489ace18893f19.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-f96927166a8959d2.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern parking_lot=/home/jelmer/src/janitor/target/debug/deps/libparking_lot-014cc28dd9f2a440.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling gix-hashtable v0.8.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_hashtable --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-hashtable-0.8.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=b0fa54b109c06c0d -C extra-filename=-2c11bafbe7bee320 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern gix_hash=/home/jelmer/src/janitor/target/debug/deps/libgix_hash-75921820cfc67c8b.rmeta --extern hashbrown=/home/jelmer/src/janitor/target/debug/deps/libhashbrown-64d57f8b85b00ef7.rmeta --extern parking_lot=/home/jelmer/src/janitor/target/debug/deps/libparking_lot-014cc28dd9f2a440.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling toml_edit v0.22.24
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name toml_edit --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/toml_edit-0.22.24/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::zero_sized_map_values' '--warn=clippy::wildcard_imports' '--warn=clippy::verbose_file_reads' --warn=unused_qualifications --warn=unused_macro_rules --warn=unused_lifetimes --warn=unsafe_op_in_unsafe_fn --warn=unreachable_pub '--warn=clippy::uninlined_format_args' '--warn=clippy::trait_duplication_in_bounds' '--warn=clippy::todo' '--warn=clippy::string_to_string' '--warn=clippy::string_lit_as_bytes' '--warn=clippy::string_add_assign' '--warn=clippy::string_add' '--warn=clippy::str_to_string' '--warn=clippy::semicolon_if_nothing_returned' '--warn=clippy::self_named_module_files' '--warn=clippy::same_functions_in_if_condition' '--allow=clippy::result_large_err' '--warn=clippy::rest_pat_in_fully_bound_structs' '--warn=clippy::ref_option_ref' '--warn=clippy::redundant_feature_names' '--warn=clippy::rc_mutex' '--warn=clippy::ptr_as_ptr' '--warn=clippy::path_buf_push_overwrite' '--warn=clippy::negative_feature_names' '--warn=clippy::needless_for_each' '--warn=clippy::needless_continue' '--warn=clippy::mutex_integer' '--warn=clippy::mem_forget' '--warn=clippy::macro_use_imports' '--warn=clippy::lossy_float_literal' '--warn=clippy::linkedlist' '--allow=clippy::let_and_return' '--warn=clippy::large_types_passed_by_value' '--warn=clippy::large_stack_arrays' '--warn=clippy::large_digit_groups' '--warn=clippy::invalid_upcast_comparisons' '--warn=clippy::infinite_loop' '--warn=clippy::inefficient_to_string' '--warn=clippy::inconsistent_struct_constructor' '--warn=clippy::imprecise_flops' '--warn=clippy::implicit_clone' '--allow=clippy::if_same_then_else' '--warn=clippy::from_iter_instead_of_collect' '--warn=clippy::fn_params_excessive_bools' '--warn=clippy::float_cmp_const' '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::fallible_impl_from' '--warn=clippy::explicit_into_iter_loop' '--warn=clippy::explicit_deref_methods' '--warn=clippy::expl_impl_clone_on_copy' '--warn=clippy::enum_glob_use' '--warn=clippy::empty_enum' '--warn=clippy::doc_markdown' '--warn=clippy::debug_assert_with_mut_call' '--warn=clippy::dbg_macro' '--warn=clippy::create_dir' '--allow=clippy::collapsible_else_if' '--warn=clippy::checked_conversions' '--allow=clippy::branches_sharing_code' '--allow=clippy::bool_assert_comparison' --cfg 'feature="default"' --cfg 'feature="display"' --cfg 'feature="parse"' --cfg 'feature="serde"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "display", "parse", "perf", "serde", "unbounded"))' -C metadata=9d414490935c86bd -C extra-filename=-d480cef812b1eaf3 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-1b6bab000a8558ff.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_spanned=/home/jelmer/src/janitor/target/debug/deps/libserde_spanned-e9a6e047fbfb7294.rmeta --extern toml_datetime=/home/jelmer/src/janitor/target/debug/deps/libtoml_datetime-88c3477de84748e0.rmeta --extern winnow=/home/jelmer/src/janitor/target/debug/deps/libwinnow-41cc7c75049a70dd.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling gix-lock v17.0.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_lock --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-lock-17.0.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=58a4bdb540970199 -C extra-filename=-631746035f211637 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern gix_tempfile=/home/jelmer/src/janitor/target/debug/deps/libgix_tempfile-e2da7605123b169f.rmeta --extern gix_utils=/home/jelmer/src/janitor/target/debug/deps/libgix_utils-4fa084b49260f158.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling globwalk v0.9.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name globwalk --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/globwalk-0.9.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=e6aaa43601da34e9 -C extra-filename=-bbf6639926eaf7d7 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern ignore=/home/jelmer/src/janitor/target/debug/deps/libignore-2893183fe12503e9.rmeta --extern walkdir=/home/jelmer/src/janitor/target/debug/deps/libwalkdir-f95d3688eab8bd63.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling tera v1.20.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tera --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tera-1.20.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="builtins"' --cfg 'feature="chrono"' --cfg 'feature="chrono-tz"' --cfg 'feature="default"' --cfg 'feature="humansize"' --cfg 'feature="percent-encoding"' --cfg 'feature="rand"' --cfg 'feature="slug"' --cfg 'feature="urlencode"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("builtins", "chrono", "chrono-tz", "date-locale", "default", "humansize", "percent-encoding", "preserve_order", "rand", "slug", "urlencode"))' -C metadata=e7d317ff5b81329b -C extra-filename=-5c0ea67bf6a250a8 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern chrono_tz=/home/jelmer/src/janitor/target/debug/deps/libchrono_tz-88df00c297ed5dd1.rmeta --extern globwalk=/home/jelmer/src/janitor/target/debug/deps/libglobwalk-bbf6639926eaf7d7.rmeta --extern humansize=/home/jelmer/src/janitor/target/debug/deps/libhumansize-3cef4a36ece11ec6.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pest=/home/jelmer/src/janitor/target/debug/deps/libpest-1d9946d36cd83eec.rmeta --extern pest_derive=/home/jelmer/src/janitor/target/debug/deps/libpest_derive-fe71db729a47e30d.so --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-4ffe539611cdf71f.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern slug=/home/jelmer/src/janitor/target/debug/deps/libslug-cce4bcea53cc0a96.rlib --extern unic_segment=/home/jelmer/src/janitor/target/debug/deps/libunic_segment-c2ad6cbd019bf6b8.rmeta --cap-lints allow --cfg tokio_unstable`
       Dirty pyo3-macros v0.22.6: dependency info changed
   Compiling pyo3-macros v0.22.6
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name pyo3_macros --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pyo3-macros-0.22.6/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type proc-macro --emit=dep-info,link -C prefer-dynamic -C embed-bitcode=no --warn=rust_2018_idioms '--warn=clippy::useless_transmute' '--warn=clippy::used_underscore_binding' --warn=unused_lifetimes '--warn=clippy::unnecessary_wraps' '--warn=clippy::todo' --warn=rust_2021_prelude_collisions '--warn=clippy::manual_ok_or' '--warn=clippy::manual_assert' '--warn=clippy::let_unit_value' --warn=invalid_doc_attributes '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::explicit_iter_loop' '--warn=clippy::explicit_into_iter_loop' --warn=elided_lifetimes_in_paths '--warn=clippy::dbg_macro' '--warn=clippy::checked_conversions' '--warn=rustdoc::broken_intra_doc_links' '--warn=rustdoc::bare_urls' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("experimental-async", "gil-refs", "multiple-pymethods"))' -C metadata=a155d61fafa36185 -C extra-filename=-adbfecf6299123d1 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern proc_macro2=/home/jelmer/src/janitor/target/debug/deps/libproc_macro2-a7e2001652539cec.rlib --extern pyo3_macros_backend=/home/jelmer/src/janitor/target/debug/deps/libpyo3_macros_backend-145c1c11a7f3a990.rlib --extern quote=/home/jelmer/src/janitor/target/debug/deps/libquote-8533776b6f1db290.rlib --extern syn=/home/jelmer/src/janitor/target/debug/deps/libsyn-7fe0b75e1b133791.rlib --extern proc_macro --cap-lints allow --cfg tokio_unstable`
   Compiling tokio-stream v0.1.17
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio_stream --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-stream-0.1.17/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="fs"' --cfg 'feature="time"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "fs", "full", "io-util", "net", "signal", "sync", "time", "tokio-util"))' -C metadata=dba63c6ab46f8e25 -C extra-filename=-c7d72ffe570c6601 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-00820b57743a40c7.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name pyo3 --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pyo3-0.22.6/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::useless_transmute' '--warn=clippy::used_underscore_binding' --warn=unused_lifetimes '--warn=clippy::unnecessary_wraps' '--warn=clippy::todo' --warn=rust_2021_prelude_collisions '--warn=clippy::manual_ok_or' '--warn=clippy::manual_assert' '--warn=clippy::let_unit_value' --warn=invalid_doc_attributes '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::explicit_iter_loop' '--warn=clippy::explicit_into_iter_loop' --warn=elided_lifetimes_in_paths '--warn=clippy::dbg_macro' '--warn=clippy::checked_conversions' '--warn=rustdoc::broken_intra_doc_links' '--warn=rustdoc::bare_urls' --cfg 'feature="auto-initialize"' --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="indoc"' --cfg 'feature="macros"' --cfg 'feature="pyo3-macros"' --cfg 'feature="serde"' --cfg 'feature="unindent"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("abi3", "abi3-py310", "abi3-py311", "abi3-py312", "abi3-py37", "abi3-py38", "abi3-py39", "anyhow", "auto-initialize", "chrono", "chrono-tz", "default", "either", "experimental-async", "experimental-inspect", "extension-module", "eyre", "full", "generate-import-lib", "gil-refs", "hashbrown", "indexmap", "indoc", "inventory", "macros", "multiple-pymethods", "nightly", "num-bigint", "num-complex", "num-rational", "py-clone", "pyo3-macros", "rust_decimal", "serde", "smallvec", "unindent"))' -C metadata=68c28cc538a3eea4 -C extra-filename=-777300d23104bd4b --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern cfg_if=/home/jelmer/src/janitor/target/debug/deps/libcfg_if-c30e1cd0415bac84.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern indoc=/home/jelmer/src/janitor/target/debug/deps/libindoc-8b627c6bf8c7b5f4.so --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-f96927166a8959d2.rmeta --extern memoffset=/home/jelmer/src/janitor/target/debug/deps/libmemoffset-671b681cbd17a03b.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern pyo3_ffi=/home/jelmer/src/janitor/target/debug/deps/libpyo3_ffi-b222fe21b5e32384.rmeta --extern pyo3_macros=/home/jelmer/src/janitor/target/debug/deps/libpyo3_macros-adbfecf6299123d1.so --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern unindent=/home/jelmer/src/janitor/target/debug/deps/libunindent-2f0b7e45a02f755c.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu --cfg Py_3_6 --cfg Py_3_7 --cfg Py_3_8 --cfg Py_3_9 --cfg Py_3_10 --cfg Py_3_11 --cfg Py_3_12 --cfg Py_3_13 --cfg invalid_from_utf8_lint --cfg c_str_lit --cfg diagnostic_namespace --check-cfg 'cfg(Py_LIMITED_API)' --check-cfg 'cfg(PyPy)' --check-cfg 'cfg(GraalPy)' --check-cfg 'cfg(py_sys_config, values("Py_DEBUG", "Py_REF_DEBUG", "Py_TRACE_REFS", "COUNT_ALLOCS"))' --check-cfg 'cfg(invalid_from_utf8_lint)' --check-cfg 'cfg(pyo3_disable_reference_pool)' --check-cfg 'cfg(pyo3_leak_on_drop_without_reference_pool)' --check-cfg 'cfg(diagnostic_namespace)' --check-cfg 'cfg(c_str_lit)' --check-cfg 'cfg(Py_3_7)' --check-cfg 'cfg(Py_3_8)' --check-cfg 'cfg(Py_3_9)' --check-cfg 'cfg(Py_3_10)' --check-cfg 'cfg(Py_3_11)' --check-cfg 'cfg(Py_3_12)' --check-cfg 'cfg(Py_3_13)'`
   Compiling gix-date v0.9.4
   Compiling env_logger v0.11.7
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name env_logger --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/env_logger-0.11.7/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::zero_sized_map_values' '--warn=clippy::wildcard_imports' '--warn=clippy::verbose_file_reads' --warn=unused_qualifications --warn=unused_macro_rules --warn=unused_lifetimes --warn=unsafe_op_in_unsafe_fn --warn=unreachable_pub '--warn=clippy::uninlined_format_args' '--warn=clippy::trait_duplication_in_bounds' '--warn=clippy::todo' '--warn=clippy::string_to_string' '--warn=clippy::string_lit_as_bytes' '--warn=clippy::string_add_assign' '--warn=clippy::string_add' '--warn=clippy::str_to_string' '--warn=clippy::semicolon_if_nothing_returned' '--warn=clippy::self_named_module_files' '--warn=clippy::same_functions_in_if_condition' '--allow=clippy::result_large_err' '--warn=clippy::rest_pat_in_fully_bound_structs' '--warn=clippy::ref_option_ref' '--warn=clippy::redundant_feature_names' '--warn=clippy::rc_mutex' '--warn=clippy::ptr_as_ptr' '--warn=clippy::path_buf_push_overwrite' '--warn=clippy::negative_feature_names' '--warn=clippy::needless_for_each' '--warn=clippy::needless_continue' '--warn=clippy::mutex_integer' '--warn=clippy::mem_forget' '--warn=clippy::macro_use_imports' '--warn=clippy::lossy_float_literal' '--warn=clippy::linkedlist' '--allow=clippy::let_and_return' '--warn=clippy::large_types_passed_by_value' '--warn=clippy::large_stack_arrays' '--warn=clippy::large_digit_groups' '--warn=clippy::invalid_upcast_comparisons' '--warn=clippy::infinite_loop' '--warn=clippy::inefficient_to_string' '--warn=clippy::inconsistent_struct_constructor' '--warn=clippy::imprecise_flops' '--warn=clippy::implicit_clone' '--allow=clippy::if_same_then_else' '--warn=clippy::from_iter_instead_of_collect' '--warn=clippy::fn_params_excessive_bools' '--warn=clippy::float_cmp_const' '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::fallible_impl_from' '--warn=clippy::explicit_into_iter_loop' '--warn=clippy::explicit_deref_methods' '--warn=clippy::expl_impl_clone_on_copy' '--warn=clippy::enum_glob_use' '--warn=clippy::empty_enum' '--warn=clippy::doc_markdown' '--warn=clippy::debug_assert_with_mut_call' '--warn=clippy::dbg_macro' '--warn=clippy::create_dir' '--allow=clippy::collapsible_else_if' '--warn=clippy::checked_conversions' '--allow=clippy::branches_sharing_code' '--allow=clippy::bool_assert_comparison' --cfg 'feature="auto-color"' --cfg 'feature="color"' --cfg 'feature="default"' --cfg 'feature="humantime"' --cfg 'feature="regex"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("auto-color", "color", "default", "humantime", "regex", "unstable-kv"))' -C metadata=c8e08d335f3fd605 -C extra-filename=-6abdb84b0421fecc --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern anstream=/home/jelmer/src/janitor/target/debug/deps/libanstream-2daa15f4ec64371e.rmeta --extern anstyle=/home/jelmer/src/janitor/target/debug/deps/libanstyle-3491f347c6e7c6e0.rmeta --extern env_filter=/home/jelmer/src/janitor/target/debug/deps/libenv_filter-c6ad938435df54a0.rmeta --extern jiff=/home/jelmer/src/janitor/target/debug/deps/libjiff-199e5d92b021876a.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_date --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-date-0.9.4/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("document-features", "serde"))' -C metadata=f682840585bda43b -C extra-filename=-7a444f14168f162e --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bstr=/home/jelmer/src/janitor/target/debug/deps/libbstr-6f503c0062ccbef8.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern jiff=/home/jelmer/src/janitor/target/debug/deps/libjiff-199e5d92b021876a.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling tokio-util v0.7.14
   Compiling tokio-native-tls v0.3.1
   Compiling tower v0.5.2
   Compiling async-compression v0.4.23
   Compiling backoff v0.4.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio_stream --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-stream-0.1.17/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="fs"' --cfg 'feature="time"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "fs", "full", "io-util", "net", "signal", "sync", "time", "tokio-util"))' -C metadata=f0ba8e44f1026fdd -C extra-filename=-816776d522fca5f1 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio_native_tls --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-native-tls-0.3.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("vendored"))' -C metadata=7f5bb74a739e83fc -C extra-filename=-c1b6e4843025b914 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-9ca42638756f24e8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tower --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tower-0.5.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="__common"' --cfg 'feature="futures-core"' --cfg 'feature="futures-util"' --cfg 'feature="log"' --cfg 'feature="make"' --cfg 'feature="pin-project-lite"' --cfg 'feature="sync_wrapper"' --cfg 'feature="timeout"' --cfg 'feature="tokio"' --cfg 'feature="tracing"' --cfg 'feature="util"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__common", "balance", "buffer", "discover", "filter", "full", "futures-core", "futures-util", "hdrhistogram", "hedge", "indexmap", "limit", "load", "load-shed", "log", "make", "pin-project-lite", "ready-cache", "reconnect", "retry", "slab", "spawn-ready", "steer", "sync_wrapper", "timeout", "tokio", "tokio-stream", "tokio-util", "tracing", "util"))' -C metadata=e28fb43df2bd3820 -C extra-filename=-e3fc2d9edd7c5e3e --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern tower_layer=/home/jelmer/src/janitor/target/debug/deps/libtower_layer-fff117c1c28e545a.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name tokio_util --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-util-0.7.14/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(loom)' --check-cfg 'cfg(mio_unsupported_force_poll_poll)' --check-cfg 'cfg(tokio_allow_from_blocking_fd)' --check-cfg 'cfg(tokio_internal_mt_counters)' --check-cfg 'cfg(tokio_no_parking_lot)' --check-cfg 'cfg(tokio_no_tuning_tests)' --check-cfg 'cfg(tokio_taskdump)' --check-cfg 'cfg(tokio_unstable)' --cfg 'feature="codec"' --cfg 'feature="default"' --cfg 'feature="io"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__docs_rs", "codec", "compat", "default", "full", "futures-io", "futures-util", "hashbrown", "io", "io-util", "net", "rt", "slab", "time", "tracing"))' -C metadata=b61cdcb4892ce8f3 -C extra-filename=-0594be3a9e82e568 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_sink=/home/jelmer/src/janitor/target/debug/deps/libfutures_sink-0f1aae5d0426fde7.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name async_compression --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/async-compression-0.4.23/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="flate2"' --cfg 'feature="gzip"' --cfg 'feature="tokio"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("all", "all-algorithms", "all-implementations", "brotli", "bzip2", "deflate", "deflate64", "flate2", "futures-io", "gzip", "libzstd", "lz4", "lzma", "tokio", "xz", "xz2", "zlib", "zstd", "zstd-safe", "zstdmt"))' -C metadata=bedbe2085a6a1a13 -C extra-filename=-010e9930c22df287 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name backoff --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/backoff-0.4.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="futures"' --cfg 'feature="futures-core"' --cfg 'feature="pin-project-lite"' --cfg 'feature="tokio"' --cfg 'feature="tokio_1"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("async-std", "async_std_1", "default", "futures", "futures-core", "pin-project-lite", "tokio", "tokio_1", "wasm-bindgen"))' -C metadata=2e3559acc675a4ce -C extra-filename=-7aa1f850c4954588 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern getrandom=/home/jelmer/src/janitor/target/debug/deps/libgetrandom-b6ab44046e752405.rmeta --extern instant=/home/jelmer/src/janitor/target/debug/deps/libinstant-1c5a97e6a6b2208e.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-4ffe539611cdf71f.rmeta --extern tokio_1=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling gix-actor v0.34.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_actor --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-actor-0.34.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("document-features", "serde"))' -C metadata=63a68d410901d14e -C extra-filename=-8dd921d49bba8fd5 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bstr=/home/jelmer/src/janitor/target/debug/deps/libbstr-6f503c0062ccbef8.rmeta --extern gix_date=/home/jelmer/src/janitor/target/debug/deps/libgix_date-7a444f14168f162e.rmeta --extern gix_utils=/home/jelmer/src/janitor/target/debug/deps/libgix_utils-4fa084b49260f158.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern winnow=/home/jelmer/src/janitor/target/debug/deps/libwinnow-41cc7c75049a70dd.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling toml v0.8.20
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name toml --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/toml-0.8.20/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=rust_2018_idioms '--warn=clippy::zero_sized_map_values' '--warn=clippy::wildcard_imports' '--warn=clippy::verbose_file_reads' --warn=unused_qualifications --warn=unused_macro_rules --warn=unused_lifetimes --warn=unsafe_op_in_unsafe_fn --warn=unreachable_pub '--warn=clippy::uninlined_format_args' '--warn=clippy::trait_duplication_in_bounds' '--warn=clippy::todo' '--warn=clippy::string_to_string' '--warn=clippy::string_lit_as_bytes' '--warn=clippy::string_add_assign' '--warn=clippy::string_add' '--warn=clippy::str_to_string' '--warn=clippy::semicolon_if_nothing_returned' '--warn=clippy::self_named_module_files' '--warn=clippy::same_functions_in_if_condition' '--allow=clippy::result_large_err' '--warn=clippy::rest_pat_in_fully_bound_structs' '--warn=clippy::ref_option_ref' '--warn=clippy::redundant_feature_names' '--warn=clippy::rc_mutex' '--warn=clippy::ptr_as_ptr' '--warn=clippy::path_buf_push_overwrite' '--warn=clippy::negative_feature_names' '--warn=clippy::needless_for_each' '--warn=clippy::needless_continue' '--warn=clippy::mutex_integer' '--warn=clippy::mem_forget' '--warn=clippy::macro_use_imports' '--warn=clippy::lossy_float_literal' '--warn=clippy::linkedlist' '--allow=clippy::let_and_return' '--warn=clippy::large_types_passed_by_value' '--warn=clippy::large_stack_arrays' '--warn=clippy::large_digit_groups' '--warn=clippy::invalid_upcast_comparisons' '--warn=clippy::infinite_loop' '--warn=clippy::inefficient_to_string' '--warn=clippy::inconsistent_struct_constructor' '--warn=clippy::imprecise_flops' '--warn=clippy::implicit_clone' '--allow=clippy::if_same_then_else' '--warn=clippy::from_iter_instead_of_collect' '--warn=clippy::fn_params_excessive_bools' '--warn=clippy::float_cmp_const' '--warn=clippy::flat_map_option' '--warn=clippy::filter_map_next' '--warn=clippy::fallible_impl_from' '--warn=clippy::explicit_into_iter_loop' '--warn=clippy::explicit_deref_methods' '--warn=clippy::expl_impl_clone_on_copy' '--warn=clippy::enum_glob_use' '--warn=clippy::empty_enum' '--warn=clippy::doc_markdown' '--warn=clippy::debug_assert_with_mut_call' '--warn=clippy::dbg_macro' '--warn=clippy::create_dir' '--allow=clippy::collapsible_else_if' '--warn=clippy::checked_conversions' '--allow=clippy::branches_sharing_code' '--allow=clippy::bool_assert_comparison' --cfg 'feature="default"' --cfg 'feature="display"' --cfg 'feature="parse"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "display", "indexmap", "parse", "preserve_order"))' -C metadata=7cc0987dc2c3c257 -C extra-filename=-d5483cddfb4473b0 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_spanned=/home/jelmer/src/janitor/target/debug/deps/libserde_spanned-e9a6e047fbfb7294.rmeta --extern toml_datetime=/home/jelmer/src/janitor/target/debug/deps/libtoml_datetime-88c3477de84748e0.rmeta --extern toml_edit=/home/jelmer/src/janitor/target/debug/deps/libtoml_edit-d480cef812b1eaf3.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling sqlx-core v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_core --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-core-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="_rt-async-std"' --cfg 'feature="_rt-tokio"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="any"' --cfg 'feature="async-io"' --cfg 'feature="async-std"' --cfg 'feature="chrono"' --cfg 'feature="crc"' --cfg 'feature="default"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="native-tls"' --cfg 'feature="offline"' --cfg 'feature="serde"' --cfg 'feature="serde_json"' --cfg 'feature="sha2"' --cfg 'feature="tokio"' --cfg 'feature="tokio-stream"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_tls-native-tls", "_tls-none", "_tls-rustls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "any", "async-io", "async-std", "bigdecimal", "bit-vec", "bstr", "chrono", "crc", "default", "ipnetwork", "json", "mac_address", "migrate", "native-tls", "offline", "regex", "rust_decimal", "rustls", "rustls-native-certs", "rustls-pemfile", "serde", "serde_json", "sha2", "time", "tokio", "tokio-stream", "uuid", "webpki-roots"))' -C metadata=de73c2d86db68af9 -C extra-filename=-71565b0104e94c2e --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_io=/home/jelmer/src/janitor/target/debug/deps/libasync_io-2442fda842a01f7a.rmeta --extern async_std=/home/jelmer/src/janitor/target/debug/deps/libasync_std-d693f9395dd19d05.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-4eb151582e08ecdb.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern crossbeam_queue=/home/jelmer/src/janitor/target/debug/deps/libcrossbeam_queue-577e4d13a58a6351.rmeta --extern either=/home/jelmer/src/janitor/target/debug/deps/libeither-5d949479ced69761.rmeta --extern event_listener=/home/jelmer/src/janitor/target/debug/deps/libevent_listener-55331feab369961e.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_intrusive=/home/jelmer/src/janitor/target/debug/deps/libfutures_intrusive-c085726410f20eaa.rmeta --extern futures_io=/home/jelmer/src/janitor/target/debug/deps/libfutures_io-40db0a981b134123.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-e2fb2d440b459c82.rmeta --extern hashbrown=/home/jelmer/src/janitor/target/debug/deps/libhashbrown-06a88afb9eab536b.rmeta --extern hashlink=/home/jelmer/src/janitor/target/debug/deps/libhashlink-59dbdb8fc63c8797.rmeta --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-d66b118fa0fa0d11.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-400fd68aa602ed65.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-d6016ca7baa5f84b.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-9969bfe2b2f70651.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-00820b57743a40c7.rmeta --extern tokio_stream=/home/jelmer/src/janitor/target/debug/deps/libtokio_stream-c7d72ffe570c6601.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-f6f36781d1866faf.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-93d7573978769e30.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_core --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-core-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="_rt-async-std"' --cfg 'feature="_rt-tokio"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="any"' --cfg 'feature="async-io"' --cfg 'feature="async-std"' --cfg 'feature="chrono"' --cfg 'feature="crc"' --cfg 'feature="default"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="native-tls"' --cfg 'feature="offline"' --cfg 'feature="serde"' --cfg 'feature="serde_json"' --cfg 'feature="sha2"' --cfg 'feature="time"' --cfg 'feature="tokio"' --cfg 'feature="tokio-stream"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_tls-native-tls", "_tls-none", "_tls-rustls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "any", "async-io", "async-std", "bigdecimal", "bit-vec", "bstr", "chrono", "crc", "default", "ipnetwork", "json", "mac_address", "migrate", "native-tls", "offline", "regex", "rust_decimal", "rustls", "rustls-native-certs", "rustls-pemfile", "serde", "serde_json", "sha2", "time", "tokio", "tokio-stream", "uuid", "webpki-roots"))' -C metadata=3f75d4ef3baae753 -C extra-filename=-889a5213dce729ff --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_io=/home/jelmer/src/janitor/target/debug/deps/libasync_io-68c6881e06af5fb5.rmeta --extern async_std=/home/jelmer/src/janitor/target/debug/deps/libasync_std-ceb095294ba49aaa.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern crossbeam_queue=/home/jelmer/src/janitor/target/debug/deps/libcrossbeam_queue-577e4d13a58a6351.rmeta --extern either=/home/jelmer/src/janitor/target/debug/deps/libeither-5d949479ced69761.rmeta --extern event_listener=/home/jelmer/src/janitor/target/debug/deps/libevent_listener-55331feab369961e.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_intrusive=/home/jelmer/src/janitor/target/debug/deps/libfutures_intrusive-8de560a3fb7b1d3e.rmeta --extern futures_io=/home/jelmer/src/janitor/target/debug/deps/libfutures_io-40db0a981b134123.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern hashbrown=/home/jelmer/src/janitor/target/debug/deps/libhashbrown-06a88afb9eab536b.rmeta --extern hashlink=/home/jelmer/src/janitor/target/debug/deps/libhashlink-59dbdb8fc63c8797.rmeta --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-1b6bab000a8558ff.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-9ca42638756f24e8.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-0518d41859d801b9.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-93a16e2f2de5f6ed.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern tokio_stream=/home/jelmer/src/janitor/target/debug/deps/libtokio_stream-816776d522fca5f1.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling h2 v0.4.8
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name h2 --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/h2-0.4.8/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(fuzzing)' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("stream", "unstable"))' -C metadata=592b7e8fe0122307 -C extra-filename=-0f1e603f327c59e0 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern atomic_waker=/home/jelmer/src/janitor/target/debug/deps/libatomic_waker-21f0b624b8878034.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern fnv=/home/jelmer/src/janitor/target/debug/deps/libfnv-135eca59eff18b18.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_sink=/home/jelmer/src/janitor/target/debug/deps/libfutures_sink-0f1aae5d0426fde7.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-1b6bab000a8558ff.rmeta --extern slab=/home/jelmer/src/janitor/target/debug/deps/libslab-58feeb60e58ddd09.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-0594be3a9e82e568.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling gix-object v0.48.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_object --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-object-0.48.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("document-features", "serde", "verbose-object-parsing-errors"))' -C metadata=56b11b9462c9d2fe -C extra-filename=-aa40a8e451958b74 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bstr=/home/jelmer/src/janitor/target/debug/deps/libbstr-6f503c0062ccbef8.rmeta --extern gix_actor=/home/jelmer/src/janitor/target/debug/deps/libgix_actor-8dd921d49bba8fd5.rmeta --extern gix_date=/home/jelmer/src/janitor/target/debug/deps/libgix_date-7a444f14168f162e.rmeta --extern gix_features=/home/jelmer/src/janitor/target/debug/deps/libgix_features-fd3d337f1c519864.rmeta --extern gix_hash=/home/jelmer/src/janitor/target/debug/deps/libgix_hash-75921820cfc67c8b.rmeta --extern gix_hashtable=/home/jelmer/src/janitor/target/debug/deps/libgix_hashtable-2c11bafbe7bee320.rmeta --extern gix_path=/home/jelmer/src/janitor/target/debug/deps/libgix_path-3bb66e4cb32aadec.rmeta --extern gix_utils=/home/jelmer/src/janitor/target/debug/deps/libgix_utils-4fa084b49260f158.rmeta --extern gix_validate=/home/jelmer/src/janitor/target/debug/deps/libgix_validate-7654a34ba17d943c.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-0518d41859d801b9.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern winnow=/home/jelmer/src/janitor/target/debug/deps/libwinnow-41cc7c75049a70dd.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling pyproject-toml v0.13.4
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name pyproject_toml --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pyproject-toml-0.13.4/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("glob", "pep639-glob", "tracing"))' -C metadata=7ef24750695b9969 -C extra-filename=-a0ba8d2ed0eefc4f --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern indexmap=/home/jelmer/src/janitor/target/debug/deps/libindexmap-1b6bab000a8558ff.rmeta --extern pep440_rs=/home/jelmer/src/janitor/target/debug/deps/libpep440_rs-396545d9aec3c6f1.rlib --extern pep508_rs=/home/jelmer/src/janitor/target/debug/deps/libpep508_rs-dbb9ac0af9cda1f3.rlib --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern toml=/home/jelmer/src/janitor/target/debug/deps/libtoml-d5483cddfb4473b0.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/src/janitor/target/debug/build/janitor-3e4912ad33cb41bf/build-script-build`
   Compiling gix-ref v0.51.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_ref --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-ref-0.51.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("document-features", "serde"))' -C metadata=accc85e746000779 -C extra-filename=-f950d4768b0c5a02 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern gix_actor=/home/jelmer/src/janitor/target/debug/deps/libgix_actor-8dd921d49bba8fd5.rmeta --extern gix_features=/home/jelmer/src/janitor/target/debug/deps/libgix_features-fd3d337f1c519864.rmeta --extern gix_fs=/home/jelmer/src/janitor/target/debug/deps/libgix_fs-65489ace18893f19.rmeta --extern gix_hash=/home/jelmer/src/janitor/target/debug/deps/libgix_hash-75921820cfc67c8b.rmeta --extern gix_lock=/home/jelmer/src/janitor/target/debug/deps/libgix_lock-631746035f211637.rmeta --extern gix_object=/home/jelmer/src/janitor/target/debug/deps/libgix_object-aa40a8e451958b74.rmeta --extern gix_path=/home/jelmer/src/janitor/target/debug/deps/libgix_path-3bb66e4cb32aadec.rmeta --extern gix_tempfile=/home/jelmer/src/janitor/target/debug/deps/libgix_tempfile-e2da7605123b169f.rmeta --extern gix_utils=/home/jelmer/src/janitor/target/debug/deps/libgix_utils-4fa084b49260f158.rmeta --extern gix_validate=/home/jelmer/src/janitor/target/debug/deps/libgix_validate-7654a34ba17d943c.rmeta --extern memmap2=/home/jelmer/src/janitor/target/debug/deps/libmemmap2-eacc05f9d4f4eab8.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern winnow=/home/jelmer/src/janitor/target/debug/deps/libwinnow-41cc7c75049a70dd.rmeta --cap-lints allow --cfg tokio_unstable`
       Dirty deb822-lossless v0.2.4: dependency info changed
   Compiling deb822-lossless v0.2.4
       Dirty pyo3-filelike v0.4.1: dependency info changed
   Compiling pyo3-filelike v0.4.1
       Dirty pyo3-log v0.11.0: dependency info changed
   Compiling pyo3-log v0.11.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name deb822_lossless --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/deb822-lossless-0.2.4/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="python-debian"' --cfg 'feature="serde"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "derive", "python-debian", "serde"))' -C metadata=5de361dcb2fa4a6d -C extra-filename=-a2f8bbe16554433c --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern deb822_derive=/home/jelmer/src/janitor/target/debug/deps/libdeb822_derive-f74a42b98018062e.so --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern rowan=/home/jelmer/src/janitor/target/debug/deps/librowan-526d818a99f2d05d.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name pyo3_filelike --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pyo3-filelike-0.4.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=850d85eb04cef2d6 -C extra-filename=-4260b2d0090d278a --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name pyo3_log --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pyo3-log-0.11.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=d572313f3e488444 -C extra-filename=-632241dddb9114dd --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern arc_swap=/home/jelmer/src/janitor/target/debug/deps/libarc_swap-bd5aa4a1e22f9e5d.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
       Dirty dep3 v0.1.28: dependency info changed
   Compiling dep3 v0.1.28
       Dirty r-description v0.3.1: dependency info changed
   Compiling r-description v0.3.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name dep3 --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/dep3-0.1.28/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=cc09da04b60b8ee3 -C extra-filename=-394c3b7964f867b9 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-a2f8bbe16554433c.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name r_description --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/r-description-0.3.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="serde"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("serde"))' -C metadata=5f5df1fc4e94fdba -C extra-filename=-ace8e3ae5bb06b10 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-a2f8bbe16554433c.rmeta --extern rowan=/home/jelmer/src/janitor/target/debug/deps/librowan-526d818a99f2d05d.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
   Compiling gix-config v0.44.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name gix_config --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gix-config-0.44.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--warn=clippy::pedantic' '--allow=clippy::wildcard_imports' '--allow=clippy::used_underscore_binding' '--allow=clippy::unused_self' '--allow=clippy::unreadable_literal' '--allow=clippy::unnecessary_wraps' '--allow=clippy::unnecessary_join' '--allow=clippy::trivially_copy_pass_by_ref' '--allow=clippy::transmute_ptr_to_ptr' '--allow=clippy::too_many_lines' '--allow=clippy::too_long_first_doc_paragraph' '--allow=clippy::struct_field_names' '--allow=clippy::struct_excessive_bools' '--allow=clippy::stable_sort_primitive' '--allow=clippy::single_match_else' '--allow=clippy::similar_names' '--allow=clippy::should_panic_without_expect' '--allow=clippy::return_self_not_must_use' '--allow=clippy::redundant_else' '--allow=clippy::range_plus_one' '--allow=clippy::option_option' '--allow=clippy::no_effect_underscore_binding' '--allow=clippy::needless_raw_string_hashes' '--allow=clippy::needless_pass_by_value' '--allow=clippy::needless_for_each' '--allow=clippy::needless_continue' '--allow=clippy::naive_bytecount' '--allow=clippy::mut_mut' '--allow=clippy::must_use_candidate' '--allow=clippy::module_name_repetitions' '--allow=clippy::missing_panics_doc' '--allow=clippy::missing_errors_doc' '--allow=clippy::match_wildcard_for_single_variants' '--allow=clippy::match_wild_err_arm' '--allow=clippy::match_same_arms' '--allow=clippy::match_bool' '--allow=clippy::many_single_char_names' '--allow=clippy::manual_string_new' '--allow=clippy::manual_let_else' '--allow=clippy::manual_is_variant_and' '--allow=clippy::manual_assert' '--allow=clippy::large_stack_arrays' '--allow=clippy::iter_without_into_iter' '--allow=clippy::iter_not_returning_iterator' '--allow=clippy::items_after_statements' '--allow=clippy::inline_always' '--allow=clippy::inefficient_to_string' '--allow=clippy::inconsistent_struct_constructor' '--allow=clippy::implicit_clone' '--allow=clippy::ignored_unit_patterns' '--allow=clippy::if_not_else' '--allow=clippy::from_iter_instead_of_collect' '--allow=clippy::fn_params_excessive_bools' '--allow=clippy::filter_map_next' '--allow=clippy::explicit_iter_loop' '--allow=clippy::explicit_into_iter_loop' '--allow=clippy::explicit_deref_methods' '--allow=clippy::enum_glob_use' '--allow=clippy::empty_docs' '--allow=clippy::doc_markdown' '--allow=clippy::default_trait_access' '--allow=clippy::copy_iterator' '--allow=clippy::checked_conversions' '--allow=clippy::cast_sign_loss' '--allow=clippy::cast_precision_loss' '--allow=clippy::cast_possible_wrap' '--allow=clippy::cast_possible_truncation' '--allow=clippy::cast_lossless' '--allow=clippy::borrow_as_ptr' '--allow=clippy::bool_to_int_with_if' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("document-features", "serde"))' -C metadata=1b8c28aec3c13692 -C extra-filename=-ef9b5e43933bc657 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bstr=/home/jelmer/src/janitor/target/debug/deps/libbstr-6f503c0062ccbef8.rmeta --extern gix_config_value=/home/jelmer/src/janitor/target/debug/deps/libgix_config_value-9118627372b1dcb7.rmeta --extern gix_features=/home/jelmer/src/janitor/target/debug/deps/libgix_features-fd3d337f1c519864.rmeta --extern gix_glob=/home/jelmer/src/janitor/target/debug/deps/libgix_glob-ca729eeb73408c31.rmeta --extern gix_path=/home/jelmer/src/janitor/target/debug/deps/libgix_path-3bb66e4cb32aadec.rmeta --extern gix_ref=/home/jelmer/src/janitor/target/debug/deps/libgix_ref-f950d4768b0c5a02.rmeta --extern gix_sec=/home/jelmer/src/janitor/target/debug/deps/libgix_sec-2f1846542ec3dfa0.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-0518d41859d801b9.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern unicode_bom=/home/jelmer/src/janitor/target/debug/deps/libunicode_bom-bb0c78d8245504f4.rmeta --extern winnow=/home/jelmer/src/janitor/target/debug/deps/libwinnow-41cc7c75049a70dd.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper v1.6.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-1.6.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(hyper_unstable_tracing)' --check-cfg 'cfg(hyper_unstable_ffi)' --cfg 'feature="client"' --cfg 'feature="default"' --cfg 'feature="http1"' --cfg 'feature="http2"' --cfg 'feature="server"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("capi", "client", "default", "ffi", "full", "http1", "http2", "nightly", "server", "tracing"))' -C metadata=ccf003c14ab42e18 -C extra-filename=-63fe26b8683779ff --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern h2=/home/jelmer/src/janitor/target/debug/deps/libh2-0f1e603f327c59e0.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern httparse=/home/jelmer/src/janitor/target/debug/deps/libhttparse-de9e4dfe0f78db23.rmeta --extern httpdate=/home/jelmer/src/janitor/target/debug/deps/libhttpdate-66eb51e4c8d24adc.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-0518d41859d801b9.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern want=/home/jelmer/src/janitor/target/debug/deps/libwant-676b1650d2642fde.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling sqlx-postgres v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_postgres --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-postgres-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="chrono"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="offline"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("any", "bigdecimal", "bit-vec", "chrono", "ipnetwork", "json", "mac_address", "migrate", "offline", "rust_decimal", "time", "uuid"))' -C metadata=7ca63ecde6faea05 -C extra-filename=-fe872018237c7f96 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern atoi=/home/jelmer/src/janitor/target/debug/deps/libatoi-f8455101c6ea3fc4.rmeta --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-bf6eccdff131582a.rmeta --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern byteorder=/home/jelmer/src/janitor/target/debug/deps/libbyteorder-0d143049d2b5b6fb.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-4eb151582e08ecdb.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern dotenvy=/home/jelmer/src/janitor/target/debug/deps/libdotenvy-c5aead2ec43889b7.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-1f4beae7161f5951.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-e2fb2d440b459c82.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern hkdf=/home/jelmer/src/janitor/target/debug/deps/libhkdf-46ecc70a36f2ad04.rmeta --extern hmac=/home/jelmer/src/janitor/target/debug/deps/libhmac-d886aec669ee88c9.rmeta --extern home=/home/jelmer/src/janitor/target/debug/deps/libhome-fc8b76e8d1b394eb.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-400fd68aa602ed65.rmeta --extern md5=/home/jelmer/src/janitor/target/debug/deps/libmd5-e96a7bc866d34328.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-e190b0b4b7812a25.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-9969bfe2b2f70651.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-71565b0104e94c2e.rmeta --extern stringprep=/home/jelmer/src/janitor/target/debug/deps/libstringprep-3e84ecdea7cbe138.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-f6f36781d1866faf.rmeta --extern whoami=/home/jelmer/src/janitor/target/debug/deps/libwhoami-aa57d5d7654aa522.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper-util v0.1.10
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper_util --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-util-0.1.10/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="client"' --cfg 'feature="client-legacy"' --cfg 'feature="default"' --cfg 'feature="http1"' --cfg 'feature="http2"' --cfg 'feature="server"' --cfg 'feature="service"' --cfg 'feature="tokio"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__internal_happy_eyeballs_tests", "client", "client-legacy", "default", "full", "http1", "http2", "server", "server-auto", "server-graceful", "service", "tokio"))' -C metadata=88b7cf835e205756 -C extra-filename=-6cd21a6dd6095d6f --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-63fe26b8683779ff.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern socket2=/home/jelmer/src/janitor/target/debug/deps/libsocket2-9be7658adf7d58e8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling hyper-tls v0.6.0
   Compiling axum v0.7.9
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name hyper_tls --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/hyper-tls-0.6.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("alpn", "vendored"))' -C metadata=403301321d16cda1 -C extra-filename=-be195f696c56483b --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-63fe26b8683779ff.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-6cd21a6dd6095d6f.rmeta --extern native_tls=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-9ca42638756f24e8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern tokio_native_tls=/home/jelmer/src/janitor/target/debug/deps/libtokio_native_tls-c1b6e4843025b914.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name axum --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/axum-0.7.9/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="form"' --cfg 'feature="http1"' --cfg 'feature="http2"' --cfg 'feature="json"' --cfg 'feature="matched-path"' --cfg 'feature="original-uri"' --cfg 'feature="query"' --cfg 'feature="tokio"' --cfg 'feature="tower-log"' --cfg 'feature="tracing"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__private_docs", "default", "form", "http1", "http2", "json", "macros", "matched-path", "multipart", "original-uri", "query", "tokio", "tower-log", "tracing", "ws"))' -C metadata=6c39c7cfb1b27ad1 -C extra-filename=-8fb5e3339fd3daf6 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern axum_core=/home/jelmer/src/janitor/target/debug/deps/libaxum_core-7f0c6da1d585f831.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-63fe26b8683779ff.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-6cd21a6dd6095d6f.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern matchit=/home/jelmer/src/janitor/target/debug/deps/libmatchit-da252bff9596cbbd.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern mime=/home/jelmer/src/janitor/target/debug/deps/libmime-c74000db2aaf511f.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern rustversion=/home/jelmer/src/janitor/target/debug/deps/librustversion-494b2fd16358ba50.so --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_path_to_error=/home/jelmer/src/janitor/target/debug/deps/libserde_path_to_error-72e72ae8986ce543.rmeta --extern serde_urlencoded=/home/jelmer/src/janitor/target/debug/deps/libserde_urlencoded-e0562f68e1545f98.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern tower=/home/jelmer/src/janitor/target/debug/deps/libtower-e3fc2d9edd7c5e3e.rmeta --extern tower_layer=/home/jelmer/src/janitor/target/debug/deps/libtower_layer-fff117c1c28e545a.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_postgres --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-postgres-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="any"' --cfg 'feature="chrono"' --cfg 'feature="json"' --cfg 'feature="migrate"' --cfg 'feature="time"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("any", "bigdecimal", "bit-vec", "chrono", "ipnetwork", "json", "mac_address", "migrate", "offline", "rust_decimal", "time", "uuid"))' -C metadata=364a9b5528e7a8f7 -C extra-filename=-67cde0941f4b68ff --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern atoi=/home/jelmer/src/janitor/target/debug/deps/libatoi-08701d6ef2ff6341.rmeta --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-b1a4de7ed4da6927.rmeta --extern bitflags=/home/jelmer/src/janitor/target/debug/deps/libbitflags-262765b49292667c.rmeta --extern byteorder=/home/jelmer/src/janitor/target/debug/deps/libbyteorder-9b173727c781ecdd.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern crc=/home/jelmer/src/janitor/target/debug/deps/libcrc-b279ac953a9b582e.rmeta --extern dotenvy=/home/jelmer/src/janitor/target/debug/deps/libdotenvy-c5aead2ec43889b7.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern hkdf=/home/jelmer/src/janitor/target/debug/deps/libhkdf-46ecc70a36f2ad04.rmeta --extern hmac=/home/jelmer/src/janitor/target/debug/deps/libhmac-d886aec669ee88c9.rmeta --extern home=/home/jelmer/src/janitor/target/debug/deps/libhome-fc8b76e8d1b394eb.rmeta --extern itoa=/home/jelmer/src/janitor/target/debug/deps/libitoa-bfd0ae774c0f1080.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern md5=/home/jelmer/src/janitor/target/debug/deps/libmd5-e96a7bc866d34328.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-4ffe539611cdf71f.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern smallvec=/home/jelmer/src/janitor/target/debug/deps/libsmallvec-0518d41859d801b9.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-889a5213dce729ff.rmeta --extern stringprep=/home/jelmer/src/janitor/target/debug/deps/libstringprep-3e84ecdea7cbe138.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-93a16e2f2de5f6ed.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern whoami=/home/jelmer/src/janitor/target/debug/deps/libwhoami-aa57d5d7654aa522.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling reqwest v0.12.15
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name reqwest --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/reqwest-0.12.15/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --warn=unexpected_cfgs --check-cfg 'cfg(reqwest_unstable)' --cfg 'feature="__tls"' --cfg 'feature="blocking"' --cfg 'feature="charset"' --cfg 'feature="default"' --cfg 'feature="default-tls"' --cfg 'feature="h2"' --cfg 'feature="http2"' --cfg 'feature="json"' --cfg 'feature="macos-system-configuration"' --cfg 'feature="multipart"' --cfg 'feature="stream"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("__rustls", "__rustls-ring", "__tls", "blocking", "brotli", "charset", "cookies", "default", "default-tls", "deflate", "gzip", "h2", "hickory-dns", "http2", "http3", "json", "macos-system-configuration", "multipart", "native-tls", "native-tls-alpn", "native-tls-vendored", "rustls-tls", "rustls-tls-manual-roots", "rustls-tls-manual-roots-no-provider", "rustls-tls-native-roots", "rustls-tls-native-roots-no-provider", "rustls-tls-no-provider", "rustls-tls-webpki-roots", "rustls-tls-webpki-roots-no-provider", "socks", "stream", "trust-dns", "zstd"))' -C metadata=da32a6fff858240b -C extra-filename=-4bc10541b85b7d73 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-b1a4de7ed4da6927.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern encoding_rs=/home/jelmer/src/janitor/target/debug/deps/libencoding_rs-58c41e4932181cd7.rmeta --extern futures_channel=/home/jelmer/src/janitor/target/debug/deps/libfutures_channel-80924661373affb3.rmeta --extern futures_core=/home/jelmer/src/janitor/target/debug/deps/libfutures_core-f8f202ea4a1b3511.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern h2=/home/jelmer/src/janitor/target/debug/deps/libh2-0f1e603f327c59e0.rmeta --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern http_body=/home/jelmer/src/janitor/target/debug/deps/libhttp_body-1d4f0892cf54b8e4.rmeta --extern http_body_util=/home/jelmer/src/janitor/target/debug/deps/libhttp_body_util-6e6bedb883e10297.rmeta --extern hyper=/home/jelmer/src/janitor/target/debug/deps/libhyper-63fe26b8683779ff.rmeta --extern hyper_tls=/home/jelmer/src/janitor/target/debug/deps/libhyper_tls-be195f696c56483b.rmeta --extern hyper_util=/home/jelmer/src/janitor/target/debug/deps/libhyper_util-6cd21a6dd6095d6f.rmeta --extern ipnet=/home/jelmer/src/janitor/target/debug/deps/libipnet-5873e4e1530bf49f.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern mime=/home/jelmer/src/janitor/target/debug/deps/libmime-c74000db2aaf511f.rmeta --extern mime_guess=/home/jelmer/src/janitor/target/debug/deps/libmime_guess-7ee1813410f2722d.rmeta --extern native_tls_crate=/home/jelmer/src/janitor/target/debug/deps/libnative_tls-9ca42638756f24e8.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pin_project_lite=/home/jelmer/src/janitor/target/debug/deps/libpin_project_lite-fd4c1f8573716747.rmeta --extern rustls_pemfile=/home/jelmer/src/janitor/target/debug/deps/librustls_pemfile-68bb2d10b5046659.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_urlencoded=/home/jelmer/src/janitor/target/debug/deps/libserde_urlencoded-e0562f68e1545f98.rmeta --extern sync_wrapper=/home/jelmer/src/janitor/target/debug/deps/libsync_wrapper-e40416c1ff7982d8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern tokio_native_tls=/home/jelmer/src/janitor/target/debug/deps/libtokio_native_tls-c1b6e4843025b914.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-0594be3a9e82e568.rmeta --extern tower=/home/jelmer/src/janitor/target/debug/deps/libtower-e3fc2d9edd7c5e3e.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling google-cloud-metadata v0.5.1
   Compiling reqwest-middleware v0.3.3
   Compiling prometheus v0.14.0
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name google_cloud_metadata --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/google-cloud-metadata-0.5.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=b20247e610dd6d9b -C extra-filename=-e41d41280625b251 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name reqwest_middleware --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/reqwest-middleware-0.3.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="json"' --cfg 'feature="multipart"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("charset", "http2", "json", "multipart", "rustls-tls"))' -C metadata=3f60145798e682cc -C extra-filename=-f308b11c072e9b2c --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern anyhow=/home/jelmer/src/janitor/target/debug/deps/libanyhow-904a89ff6dd1202e.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern http=/home/jelmer/src/janitor/target/debug/deps/libhttp-5c70c8489530c80e.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern tower_service=/home/jelmer/src/janitor/target/debug/deps/libtower_service-6c34c4705a7ff141.rmeta --cap-lints allow --cfg tokio_unstable`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name prometheus --edition=2018 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/prometheus-0.14.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="protobuf"' --cfg 'feature="reqwest"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "gen", "libc", "nightly", "process", "procfs", "protobuf", "protobuf-codegen", "push", "reqwest"))' -C metadata=c991881147b03f10 -C extra-filename=-675a227af86df9e2 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern cfg_if=/home/jelmer/src/janitor/target/debug/deps/libcfg_if-c30e1cd0415bac84.rmeta --extern fnv=/home/jelmer/src/janitor/target/debug/deps/libfnv-135eca59eff18b18.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern memchr=/home/jelmer/src/janitor/target/debug/deps/libmemchr-7e4c5f25ce5d29b2.rmeta --extern parking_lot=/home/jelmer/src/janitor/target/debug/deps/libparking_lot-014cc28dd9f2a440.rmeta --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-b1e3dc89f529da22.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-8c88c3b03e7f8814.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling google-cloud-auth v0.17.2
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name google_cloud_auth --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/google-cloud-auth-0.17.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="default-tls"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "default-tls", "external-account", "hex", "hickory-dns", "hmac", "path-clean", "percent-encoding", "rustls-tls", "sha2", "url"))' -C metadata=a5603720e275c4f9 -C extra-filename=-8fbb47d89a739226 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-41324bb9dba3dad5.rmeta --extern google_cloud_metadata=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_metadata-e41d41280625b251.rmeta --extern google_cloud_token=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_token-6145b7093dd432ee.rmeta --extern home=/home/jelmer/src/janitor/target/debug/deps/libhome-fc8b76e8d1b394eb.rmeta --extern jsonwebtoken=/home/jelmer/src/janitor/target/debug/deps/libjsonwebtoken-7f6ac9e80053d66b.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-93a16e2f2de5f6ed.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern urlencoding=/home/jelmer/src/janitor/target/debug/deps/liburlencoding-0ba1b8b89d728edb.rmeta --cap-lints allow --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
   Compiling sqlx-macros-core v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_macros_core --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-macros-core-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no --warn=unexpected_cfgs --cfg 'feature="_rt-async-std"' --cfg 'feature="_rt-tokio"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="async-std"' --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="json"' --cfg 'feature="macros"' --cfg 'feature="migrate"' --cfg 'feature="postgres"' --cfg 'feature="sqlx-postgres"' --cfg 'feature="tokio"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_sqlite", "_tls-native-tls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "async-std", "bigdecimal", "bit-vec", "chrono", "default", "derive", "ipnetwork", "json", "mac_address", "macros", "migrate", "mysql", "postgres", "rust_decimal", "sqlite", "sqlite-unbundled", "sqlx-mysql", "sqlx-postgres", "sqlx-sqlite", "time", "tokio", "uuid"))' -C metadata=8cbc03a17dacfde6 -C extra-filename=-f51a136b13281372 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_std=/home/jelmer/src/janitor/target/debug/deps/libasync_std-d693f9395dd19d05.rmeta --extern dotenvy=/home/jelmer/src/janitor/target/debug/deps/libdotenvy-c5aead2ec43889b7.rmeta --extern either=/home/jelmer/src/janitor/target/debug/deps/libeither-5d949479ced69761.rmeta --extern heck=/home/jelmer/src/janitor/target/debug/deps/libheck-4d6a9c8516811f18.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern proc_macro2=/home/jelmer/src/janitor/target/debug/deps/libproc_macro2-a7e2001652539cec.rmeta --extern quote=/home/jelmer/src/janitor/target/debug/deps/libquote-8533776b6f1db290.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-71565b0104e94c2e.rmeta --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-fe872018237c7f96.rmeta --extern syn=/home/jelmer/src/janitor/target/debug/deps/libsyn-7fe0b75e1b133791.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-5cc214a3774c4b08.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-00820b57743a40c7.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-93d7573978769e30.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling sqlx-macros v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx_macros --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-macros-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type proc-macro --emit=dep-info,link -C prefer-dynamic -C embed-bitcode=no '--deny=clippy::disallowed_methods' '--deny=clippy::cast_sign_loss' '--deny=clippy::cast_possible_wrap' '--deny=clippy::cast_possible_truncation' --cfg 'feature="_rt-async-std"' --cfg 'feature="_rt-tokio"' --cfg 'feature="_tls-native-tls"' --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="json"' --cfg 'feature="macros"' --cfg 'feature="migrate"' --cfg 'feature="postgres"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_tls-native-tls", "_tls-rustls-aws-lc-rs", "_tls-rustls-ring-native-roots", "_tls-rustls-ring-webpki", "bigdecimal", "bit-vec", "chrono", "default", "derive", "ipnetwork", "json", "mac_address", "macros", "migrate", "mysql", "postgres", "rust_decimal", "sqlite", "sqlite-unbundled", "time", "uuid"))' -C metadata=3cbdef2241f0844b -C extra-filename=-7b85c6da819a0a53 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern proc_macro2=/home/jelmer/src/janitor/target/debug/deps/libproc_macro2-a7e2001652539cec.rlib --extern quote=/home/jelmer/src/janitor/target/debug/deps/libquote-8533776b6f1db290.rlib --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-71565b0104e94c2e.rlib --extern sqlx_macros_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_macros_core-f51a136b13281372.rlib --extern syn=/home/jelmer/src/janitor/target/debug/deps/libsyn-7fe0b75e1b133791.rlib --extern proc_macro --cap-lints allow --cfg tokio_unstable`
   Compiling google-cloud-storage v0.22.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name google_cloud_storage --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/google-cloud-storage-0.22.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="auth"' --cfg 'feature="default"' --cfg 'feature="default-tls"' --cfg 'feature="google-cloud-auth"' --cfg 'feature="google-cloud-metadata"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("auth", "default", "default-tls", "external-account", "google-cloud-auth", "google-cloud-metadata", "hickory-dns", "rustls-tls", "trace"))' -C metadata=f2284dbc3699409b -C extra-filename=-0c6e211063707140 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern anyhow=/home/jelmer/src/janitor/target/debug/deps/libanyhow-904a89ff6dd1202e.rmeta --extern async_stream=/home/jelmer/src/janitor/target/debug/deps/libasync_stream-f0f1e6ef812a7b6c.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern base64=/home/jelmer/src/janitor/target/debug/deps/libbase64-41324bb9dba3dad5.rmeta --extern bytes=/home/jelmer/src/janitor/target/debug/deps/libbytes-099f42b20cf0143e.rmeta --extern futures_util=/home/jelmer/src/janitor/target/debug/deps/libfutures_util-1f0a9bd6fcb5f15f.rmeta --extern google_cloud_auth=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_auth-8fbb47d89a739226.rmeta --extern google_cloud_metadata=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_metadata-e41d41280625b251.rmeta --extern google_cloud_token=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_token-6145b7093dd432ee.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern once_cell=/home/jelmer/src/janitor/target/debug/deps/libonce_cell-ed5628b205e2b376.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pkcs8=/home/jelmer/src/janitor/target/debug/deps/libpkcs8-ef54810b56a401a1.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rmeta --extern reqwest_middleware=/home/jelmer/src/janitor/target/debug/deps/libreqwest_middleware-f308b11c072e9b2c.rmeta --extern ring=/home/jelmer/src/janitor/target/debug/deps/libring-5cd153576e85d8b3.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha2=/home/jelmer/src/janitor/target/debug/deps/libsha2-8a411318faefeef5.rmeta --extern thiserror=/home/jelmer/src/janitor/target/debug/deps/libthiserror-03606536217ee5c4.rmeta --extern time=/home/jelmer/src/janitor/target/debug/deps/libtime-93a16e2f2de5f6ed.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --cap-lints allow --cfg tokio_unstable -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
   Compiling sqlx v0.8.3
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name sqlx --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sqlx-0.8.3/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="_rt-async-std"' --cfg 'feature="_rt-tokio"' --cfg 'feature="any"' --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="derive"' --cfg 'feature="json"' --cfg 'feature="macros"' --cfg 'feature="migrate"' --cfg 'feature="postgres"' --cfg 'feature="runtime-async-std"' --cfg 'feature="runtime-async-std-native-tls"' --cfg 'feature="runtime-tokio"' --cfg 'feature="runtime-tokio-native-tls"' --cfg 'feature="sqlx-macros"' --cfg 'feature="sqlx-postgres"' --cfg 'feature="tls-native-tls"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("_rt-async-std", "_rt-tokio", "_sqlite", "_unstable-all-types", "all-databases", "any", "bigdecimal", "bit-vec", "chrono", "default", "derive", "ipnetwork", "json", "mac_address", "macros", "migrate", "mysql", "postgres", "regexp", "runtime-async-std", "runtime-async-std-native-tls", "runtime-async-std-rustls", "runtime-tokio", "runtime-tokio-native-tls", "runtime-tokio-rustls", "rust_decimal", "sqlite", "sqlite-unbundled", "sqlx-macros", "sqlx-mysql", "sqlx-postgres", "sqlx-sqlite", "time", "tls-native-tls", "tls-none", "tls-rustls", "tls-rustls-aws-lc-rs", "tls-rustls-ring", "tls-rustls-ring-native-roots", "tls-rustls-ring-webpki", "uuid"))' -C metadata=3af2ecff8b456432 -C extra-filename=-4aede0cee3bfd326 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-889a5213dce729ff.rmeta --extern sqlx_macros=/home/jelmer/src/janitor/target/debug/deps/libsqlx_macros-7b85c6da819a0a53.so --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-67cde0941f4b68ff.rmeta --cap-lints allow --cfg tokio_unstable`
   Compiling debversion v0.4.4
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debversion --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debversion-0.4.4/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="python-debian"' --cfg 'feature="serde"' --cfg 'feature="sqlx"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("default", "python-debian", "serde", "sqlx"))' -C metadata=6a68efa9792f82b4 -C extra-filename=-2bb3e4a33b8b8241 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-4aede0cee3bfd326.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
   Compiling debian-control v0.1.41
   Compiling debian-changelog v0.2.0
   Compiling debian-copyright v0.1.27
   Compiling debbugs v0.1.5
   Compiling debian-watch v0.2.8
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_control --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-control-0.1.41/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="chrono"' --cfg 'feature="default"' --cfg 'feature="lossless"' --cfg 'feature="python-debian"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("chrono", "default", "lossless", "python-debian", "serde"))' -C metadata=454377333781b52d -C extra-filename=-ff757dbc033f80ab --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-a2f8bbe16554433c.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern rowan=/home/jelmer/src/janitor/target/debug/deps/librowan-526d818a99f2d05d.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_changelog --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-changelog-0.2.0/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=a1b073f97dbd669f -C extra-filename=-fbf10812cd9c3fd9 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern rowan=/home/jelmer/src/janitor/target/debug/deps/librowan-eb9a38bc4675dcd4.rmeta --extern textwrap=/home/jelmer/src/janitor/target/debug/deps/libtextwrap-5e0992fd5b607969.rmeta --extern whoami=/home/jelmer/src/janitor/target/debug/deps/libwhoami-aa57d5d7654aa522.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debbugs --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debbugs-0.1.5/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="blocking"' --cfg 'feature="default"' --cfg 'feature="mailparse"' --cfg 'feature="tokio"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("blocking", "default", "env_logger", "mailparse", "tokio"))' -C metadata=102d6890c67e8b6e -C extra-filename=-9ca1a4ed446a11bf --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern mailparse=/home/jelmer/src/janitor/target/debug/deps/libmailparse-59d4fe371c391495.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern xmltree=/home/jelmer/src/janitor/target/debug/deps/libxmltree-a857e8900318793a.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_watch --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-watch-0.2.8/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=f5c0f142988d24a9 -C extra-filename=-d3a9e751c0226f75 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern m_lexer=/home/jelmer/src/janitor/target/debug/deps/libm_lexer-8cd1ae29b9d2b419.rmeta --extern rowan=/home/jelmer/src/janitor/target/debug/deps/librowan-526d818a99f2d05d.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_copyright --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-copyright-0.1.27/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values())' -C metadata=19b692057d38829d -C extra-filename=-0d7f650fbe5927ea --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-a2f8bbe16554433c.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
   Compiling breezyshim v0.1.227
   Compiling buildlog-consultant v0.1.1
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name breezyshim --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/breezyshim-0.1.227/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="auto-initialize"' --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="dirty-tracker"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("auto-initialize", "debian", "default", "dirty-tracker", "sqlx"))' -C metadata=5b361c7a31cbadce -C extra-filename=-76cd14e9b060156e --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern ctor=/home/jelmer/src/janitor/target/debug/deps/libctor-72258acac2d0b9ee.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-ff757dbc033f80ab.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern difflib=/home/jelmer/src/janitor/target/debug/deps/libdifflib-20593172ebc6932e.rmeta --extern dirty_tracker=/home/jelmer/src/janitor/target/debug/deps/libdirty_tracker-6abba3579c29934f.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern patchkit=/home/jelmer/src/janitor/target/debug/deps/libpatchkit-045f9ba36150b582.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rmeta --extern pyo3_filelike=/home/jelmer/src/janitor/target/debug/deps/libpyo3_filelike-4260b2d0090d278a.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name buildlog_consultant --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/buildlog-consultant-0.1.1/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("chatgpt", "cli", "default", "tokio"))' -C metadata=e6b164a1c9beeece -C extra-filename=-5d9feac314f608bd --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-ff757dbc033f80ab.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-6abdb84b0421fecc.rmeta --extern fancy_regex=/home/jelmer/src/janitor/target/debug/deps/libfancy_regex-0688edb11485e39a.rmeta --extern inventory=/home/jelmer/src/janitor/target/debug/deps/libinventory-97a54ddffe78909c.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern pep508_rs=/home/jelmer/src/janitor/target/debug/deps/libpep508_rs-dbb9ac0af9cda1f3.rlib --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_yaml=/home/jelmer/src/janitor/target/debug/deps/libserde_yaml-bdc6b84cefd54c5e.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern text_size=/home/jelmer/src/janitor/target/debug/deps/libtext_size-68834c6d82d5a146.rmeta --extern textwrap=/home/jelmer/src/janitor/target/debug/deps/libtextwrap-5e0992fd5b607969.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
   Compiling debian-analyzer v0.158.25
   Compiling upstream-ontologist v0.2.2
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_analyzer --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/debian-analyzer-0.158.25/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="default"' --cfg 'feature="merge3"' --cfg 'feature="python"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "default", "merge3", "python", "svp", "udd"))' -C metadata=e777745dc91426fc -C extra-filename=-5b67a9919d78661e --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern configparser=/home/jelmer/src/janitor/target/debug/deps/libconfigparser-aaa60c0f437f3031.rmeta --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-a2f8bbe16554433c.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-ff757dbc033f80ab.rmeta --extern debian_copyright=/home/jelmer/src/janitor/target/debug/deps/libdebian_copyright-0d7f650fbe5927ea.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern dep3=/home/jelmer/src/janitor/target/debug/deps/libdep3-394c3b7964f867b9.rmeta --extern difflib=/home/jelmer/src/janitor/target/debug/deps/libdifflib-20593172ebc6932e.rmeta --extern distro_info=/home/jelmer/src/janitor/target/debug/deps/libdistro_info-58e7847c3f078622.rmeta --extern filetime=/home/jelmer/src/janitor/target/debug/deps/libfiletime-8f90a18bbe2253cd.rmeta --extern hex=/home/jelmer/src/janitor/target/debug/deps/libhex-e6d76a83ec319582.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern makefile_lossless=/home/jelmer/src/janitor/target/debug/deps/libmakefile_lossless-72f5494b420fe70d.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern merge3=/home/jelmer/src/janitor/target/debug/deps/libmerge3-1c24ac3badc9ba5b.rmeta --extern patchkit=/home/jelmer/src/janitor/target/debug/deps/libpatchkit-045f9ba36150b582.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rmeta --extern semver=/home/jelmer/src/janitor/target/debug/deps/libsemver-b40c4b6a404a126d.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern sha1=/home/jelmer/src/janitor/target/debug/deps/libsha1-b17c4ac71af9bf14.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --extern toml_edit=/home/jelmer/src/janitor/target/debug/deps/libtoml_edit-d480cef812b1eaf3.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name upstream_ontologist --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/upstream-ontologist-0.2.2/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cargo"' --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="dist-ini"' --cfg 'feature="git-config"' --cfg 'feature="launchpad"' --cfg 'feature="opam"' --cfg 'feature="pyo3"' --cfg 'feature="pyproject-toml"' --cfg 'feature="python-pkginfo"' --cfg 'feature="r-description"' --cfg 'feature="setup-cfg"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cargo", "cli", "debcargo", "debian", "debversion", "default", "dist-ini", "git-config", "launchpad", "opam", "pyo3", "pyproject-toml", "python-pkginfo", "r-description", "setup-cfg"))' -C metadata=f6f879474766ee97 -C extra-filename=-7460468d82cf9a53 --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern configparser=/home/jelmer/src/janitor/target/debug/deps/libconfigparser-aaa60c0f437f3031.rmeta --extern debbugs=/home/jelmer/src/janitor/target/debug/deps/libdebbugs-9ca1a4ed446a11bf.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-ff757dbc033f80ab.rmeta --extern debian_copyright=/home/jelmer/src/janitor/target/debug/deps/libdebian_copyright-0d7f650fbe5927ea.rmeta --extern debian_watch=/home/jelmer/src/janitor/target/debug/deps/libdebian_watch-d3a9e751c0226f75.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern distro_info=/home/jelmer/src/janitor/target/debug/deps/libdistro_info-58e7847c3f078622.rmeta --extern futures=/home/jelmer/src/janitor/target/debug/deps/libfutures-463765cbcde647a0.rmeta --extern gix_config=/home/jelmer/src/janitor/target/debug/deps/libgix_config-ef9b5e43933bc657.rmeta --extern html5ever=/home/jelmer/src/janitor/target/debug/deps/libhtml5ever-a5c180b1f8f9cb62.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern makefile_lossless=/home/jelmer/src/janitor/target/debug/deps/libmakefile_lossless-72f5494b420fe70d.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern opam_file_rs=/home/jelmer/src/janitor/target/debug/deps/libopam_file_rs-9139c3ee6717366b.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pulldown_cmark=/home/jelmer/src/janitor/target/debug/deps/libpulldown_cmark-f8ae6f14c6522330.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rmeta --extern pyproject_toml=/home/jelmer/src/janitor/target/debug/deps/libpyproject_toml-a0ba8d2ed0eefc4f.rmeta --extern python_pkginfo=/home/jelmer/src/janitor/target/debug/deps/libpython_pkginfo-0bbea4cc7c4c24ae.rmeta --extern r_description=/home/jelmer/src/janitor/target/debug/deps/libr_description-ace8e3ae5bb06b10.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rmeta --extern rst_renderer=/home/jelmer/src/janitor/target/debug/deps/librst_renderer-5b887f6aa2fd10e0.rmeta --extern ini=/home/jelmer/src/janitor/target/debug/deps/libini-1686d1492ce3fc13.rmeta --extern select=/home/jelmer/src/janitor/target/debug/deps/libselect-7768af7a1933fbaa.rmeta --extern semver=/home/jelmer/src/janitor/target/debug/deps/libsemver-b40c4b6a404a126d.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_yaml=/home/jelmer/src/janitor/target/debug/deps/libserde_yaml-bdc6b84cefd54c5e.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern textwrap=/home/jelmer/src/janitor/target/debug/deps/libtextwrap-5e0992fd5b607969.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern toml=/home/jelmer/src/janitor/target/debug/deps/libtoml-d5483cddfb4473b0.rmeta --extern uo_rst_parser=/home/jelmer/src/janitor/target/debug/deps/libuo_rst_parser-a31e66f15bcaff87.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --extern xmltree=/home/jelmer/src/janitor/target/debug/deps/libxmltree-a857e8900318793a.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -L native=/home/jelmer/src/janitor/target/debug/build/psm-d70ac1db1779e382/out`
   Compiling silver-platter v0.5.48
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name silver_platter --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/silver-platter-0.5.48/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="detect-update-changelog"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default", "detect-update-changelog", "gpg", "last-attempt-db", "pyo3"))' -C metadata=92e3ae2ad8fce162 -C extra-filename=-99004d4ad89f4c8e --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-5b67a9919d78661e.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-ff757dbc033f80ab.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-6abdb84b0421fecc.rmeta --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-f96927166a8959d2.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-d51b72eab852ecda.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_yaml=/home/jelmer/src/janitor/target/debug/deps/libserde_yaml-bdc6b84cefd54c5e.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --extern tera=/home/jelmer/src/janitor/target/debug/deps/libtera-5c0ea67bf6a250a8.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --extern xdg=/home/jelmer/src/janitor/target/debug/deps/libxdg-23f110d46d019c5b.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor --edition=2021 src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="gcp"' --cfg 'feature="gcs"' --cfg 'feature="stackdriver_logger"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("debian", "default", "gcp", "gcs", "stackdriver_logger"))' -C metadata=85bae1026071c99d -C extra-filename=-2b9d2a508e000f49 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e3513f3c2c846e6a.rmeta --extern async_compression=/home/jelmer/src/janitor/target/debug/deps/libasync_compression-010e9930c22df287.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rmeta --extern buildlog_consultant=/home/jelmer/src/janitor/target/debug/deps/libbuildlog_consultant-5d9feac314f608bd.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-ff757dbc033f80ab.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-6abdb84b0421fecc.rmeta --extern fancy_regex=/home/jelmer/src/janitor/target/debug/deps/libfancy_regex-0688edb11485e39a.rmeta --extern filetime=/home/jelmer/src/janitor/target/debug/deps/libfiletime-8f90a18bbe2253cd.rmeta --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern futures=/home/jelmer/src/janitor/target/debug/deps/libfutures-463765cbcde647a0.rmeta --extern google_cloud_auth=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_auth-8fbb47d89a739226.rmeta --extern google_cloud_storage=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_storage-0c6e211063707140.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-675a227af86df9e2.rmeta --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-b1e3dc89f529da22.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-99004d4ad89f4c8e.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-4aede0cee3bfd326.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-889a5213dce729ff.rmeta --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-67cde0941f4b68ff.rmeta --extern stackdriver_logger=/home/jelmer/src/janitor/target/debug/deps/libstackdriver_logger-b45c27ef10c05d41.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
   Compiling ognibuild v0.0.33
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name ognibuild --edition=2021 /home/jelmer/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ognibuild-0.0.33/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="breezy"' --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="dep-server"' --cfg 'feature="udd"' --cfg 'feature="upstream"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("breezy", "cli", "debian", "default", "dep-server", "stackdriver_logger", "udd", "upstream"))' -C metadata=427285601679c38d -C extra-filename=-87189c762fb2be0d --out-dir /home/jelmer/src/janitor/target/debug/deps -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-8fb5e3339fd3daf6.rmeta --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rmeta --extern buildlog_consultant=/home/jelmer/src/janitor/target/debug/deps/libbuildlog_consultant-5d9feac314f608bd.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern deb822_lossless=/home/jelmer/src/janitor/target/debug/deps/libdeb822_lossless-a2f8bbe16554433c.rmeta --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-5b67a9919d78661e.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-ff757dbc033f80ab.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern dirs=/home/jelmer/src/janitor/target/debug/deps/libdirs-e9a130c349e2bc4d.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-6abdb84b0421fecc.rmeta --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern fs_extra=/home/jelmer/src/janitor/target/debug/deps/libfs_extra-d88324ba9dee345b.rmeta --extern inventory=/home/jelmer/src/janitor/target/debug/deps/libinventory-97a54ddffe78909c.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern lazy_static=/home/jelmer/src/janitor/target/debug/deps/liblazy_static-ac8c428bdce69098.rmeta --extern libc=/home/jelmer/src/janitor/target/debug/deps/liblibc-f96927166a8959d2.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern lz4_flex=/home/jelmer/src/janitor/target/debug/deps/liblz4_flex-f2f943111041020d.rmeta --extern lzma_rs=/home/jelmer/src/janitor/target/debug/deps/liblzma_rs-2f42bd85de193117.rmeta --extern makefile_lossless=/home/jelmer/src/janitor/target/debug/deps/libmakefile_lossless-72f5494b420fe70d.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern nix=/home/jelmer/src/janitor/target/debug/deps/libnix-6b4ef5f907bfc73a.rmeta --extern pep508_rs=/home/jelmer/src/janitor/target/debug/deps/libpep508_rs-dbb9ac0af9cda1f3.rlib --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rmeta --extern pyproject_toml=/home/jelmer/src/janitor/target/debug/deps/libpyproject_toml-a0ba8d2ed0eefc4f.rmeta --extern r_description=/home/jelmer/src/janitor/target/debug/deps/libr_description-ace8e3ae5bb06b10.rmeta --extern rand=/home/jelmer/src/janitor/target/debug/deps/librand-4ffe539611cdf71f.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rmeta --extern semver=/home/jelmer/src/janitor/target/debug/deps/libsemver-b40c4b6a404a126d.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern serde_yaml=/home/jelmer/src/janitor/target/debug/deps/libserde_yaml-bdc6b84cefd54c5e.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-4aede0cee3bfd326.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern toml=/home/jelmer/src/janitor/target/debug/deps/libtoml-d5483cddfb4473b0.rmeta --extern toml_edit=/home/jelmer/src/janitor/target/debug/deps/libtoml_edit-d480cef812b1eaf3.rmeta --extern upstream_ontologist=/home/jelmer/src/janitor/target/debug/deps/libupstream_ontologist-7460468d82cf9a53.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --extern whoami=/home/jelmer/src/janitor/target/debug/deps/libwhoami-aa57d5d7654aa522.rmeta --extern xmltree=/home/jelmer/src/janitor/target/debug/deps/libxmltree-a857e8900318793a.rmeta --cap-lints allow --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -L native=/home/jelmer/src/janitor/target/debug/build/psm-d70ac1db1779e382/out`
warning: function `reprocess_run_logs` is never used
 --> src/reprocess_logs.rs:8:10
  |
8 | async fn reprocess_run_logs(
  |          ^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(dead_code)]` on by default

warning: field `branch_url` is never read
  --> src/schedule.rs:32:5
   |
30 | pub struct ScheduleRequest {
   |            --------------- field in this struct
31 |     codebase: String,
32 |     branch_url: String,
   |     ^^^^^^^^^^

warning: function `has_cotenants` is never used
  --> src/state.rs:80:10
   |
80 | async fn has_cotenants(
   |          ^^^^^^^^^^^^^

warning: field `name` is never read
  --> src/state.rs:87:13
   |
86 |     struct Codebase {
   |            -------- field in this struct
87 |         pub name: String,
   |             ^^^^
   |
   = note: `Codebase` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: function `iter_publishable_suites` is never used
   --> src/state.rs:113:10
    |
113 | async fn iter_publishable_suites(
    |          ^^^^^^^^^^^^^^^^^^^^^^^

warning: `janitor` (lib) generated 5 warnings
   Compiling janitor-worker v0.0.0 (/home/jelmer/src/janitor/worker)
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor_worker --edition=2021 worker/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default"))' -C metadata=6bffe1f7d32befef -C extra-filename=-dda3a9755d05977a --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e3513f3c2c846e6a.rmeta --extern askama_axum=/home/jelmer/src/janitor/target/debug/deps/libaskama_axum-e8895333fc0a961c.rmeta --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-8fb5e3339fd3daf6.rmeta --extern backoff=/home/jelmer/src/janitor/target/debug/deps/libbackoff-7aa1f850c4954588.rmeta --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-5b67a9919d78661e.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern gethostname=/home/jelmer/src/janitor/target/debug/deps/libgethostname-ae3ed00e9ff91cfd.rmeta --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-2b9d2a508e000f49.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern nix=/home/jelmer/src/janitor/target/debug/deps/libnix-6b4ef5f907bfc73a.rmeta --extern ognibuild=/home/jelmer/src/janitor/target/debug/deps/libognibuild-87189c762fb2be0d.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-675a227af86df9e2.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rmeta --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-632241dddb9114dd.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-99004d4ad89f4c8e.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-0594be3a9e82e568.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out -L native=/home/jelmer/src/janitor/target/debug/build/psm-d70ac1db1779e382/out`
warning: unused variable: `sys_path`
  --> worker/src/debian/mod.rs:74:9
   |
74 |     let sys_path = pyo3::Python::with_gil(|py| {
   |         ^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_sys_path`
   |
   = note: `#[warn(unused_variables)]` on by default

warning: unused variable: `sys_executable`
  --> worker/src/debian/mod.rs:86:9
   |
86 |     let sys_executable = pyo3::Python::with_gil(|py| {
   |         ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_sys_executable`

warning: unused variable: `subpath`
   --> worker/src/generic/mod.rs:137:5
    |
137 |     subpath: &Path,
    |     ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_subpath`

warning: unused variable: `default_empty`
   --> worker/src/lib.rs:253:5
    |
253 |     default_empty: Option<bool>,
    |     ^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_default_empty`

warning: function `derive_branch_name` is never used
   --> worker/src/lib.rs:944:4
    |
944 | fn derive_branch_name(url: &url::Url) -> String {
    |    ^^^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` on by default

warning: `janitor-worker` (lib) generated 5 warnings
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor_dist --edition=2021 worker/src/bin/dist.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default"))' -C metadata=1a2e9bb3851eb08f -C extra-filename=-3e450b078b72919b --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e3513f3c2c846e6a.rlib --extern askama_axum=/home/jelmer/src/janitor/target/debug/deps/libaskama_axum-e8895333fc0a961c.rlib --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-8fb5e3339fd3daf6.rlib --extern backoff=/home/jelmer/src/janitor/target/debug/deps/libbackoff-7aa1f850c4954588.rlib --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rlib --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rlib --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rlib --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-5b67a9919d78661e.rlib --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rlib --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rlib --extern gethostname=/home/jelmer/src/janitor/target/debug/deps/libgethostname-ae3ed00e9ff91cfd.rlib --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-2b9d2a508e000f49.rlib --extern janitor_worker=/home/jelmer/src/janitor/target/debug/deps/libjanitor_worker-dda3a9755d05977a.rlib --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rlib --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rlib --extern nix=/home/jelmer/src/janitor/target/debug/deps/libnix-6b4ef5f907bfc73a.rlib --extern ognibuild=/home/jelmer/src/janitor/target/debug/deps/libognibuild-87189c762fb2be0d.rlib --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rlib --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-675a227af86df9e2.rlib --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rlib --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-632241dddb9114dd.rlib --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rlib --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rlib --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rlib --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rlib --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-99004d4ad89f4c8e.rlib --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rlib --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rlib --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-0594be3a9e82e568.rlib --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rlib --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out -L native=/home/jelmer/src/janitor/target/debug/build/psm-d70ac1db1779e382/out`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_build --edition=2021 worker/src/bin/debian-build.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default"))' -C metadata=6a95fbfa58c6ea57 -C extra-filename=-43cdaca24b785e9a --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e3513f3c2c846e6a.rlib --extern askama_axum=/home/jelmer/src/janitor/target/debug/deps/libaskama_axum-e8895333fc0a961c.rlib --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-8fb5e3339fd3daf6.rlib --extern backoff=/home/jelmer/src/janitor/target/debug/deps/libbackoff-7aa1f850c4954588.rlib --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rlib --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rlib --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rlib --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-5b67a9919d78661e.rlib --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rlib --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rlib --extern gethostname=/home/jelmer/src/janitor/target/debug/deps/libgethostname-ae3ed00e9ff91cfd.rlib --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-2b9d2a508e000f49.rlib --extern janitor_worker=/home/jelmer/src/janitor/target/debug/deps/libjanitor_worker-dda3a9755d05977a.rlib --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rlib --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rlib --extern nix=/home/jelmer/src/janitor/target/debug/deps/libnix-6b4ef5f907bfc73a.rlib --extern ognibuild=/home/jelmer/src/janitor/target/debug/deps/libognibuild-87189c762fb2be0d.rlib --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rlib --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-675a227af86df9e2.rlib --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rlib --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-632241dddb9114dd.rlib --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rlib --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rlib --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rlib --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rlib --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-99004d4ad89f4c8e.rlib --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rlib --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rlib --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-0594be3a9e82e568.rlib --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rlib --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out -L native=/home/jelmer/src/janitor/target/debug/build/psm-d70ac1db1779e382/out`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor_worker --edition=2021 worker/src/bin/worker.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default"))' -C metadata=f562e6cffbec47e0 -C extra-filename=-f44d9bfe5e314590 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e3513f3c2c846e6a.rlib --extern askama_axum=/home/jelmer/src/janitor/target/debug/deps/libaskama_axum-e8895333fc0a961c.rlib --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-8fb5e3339fd3daf6.rlib --extern backoff=/home/jelmer/src/janitor/target/debug/deps/libbackoff-7aa1f850c4954588.rlib --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rlib --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rlib --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rlib --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-5b67a9919d78661e.rlib --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rlib --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rlib --extern gethostname=/home/jelmer/src/janitor/target/debug/deps/libgethostname-ae3ed00e9ff91cfd.rlib --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-2b9d2a508e000f49.rlib --extern janitor_worker=/home/jelmer/src/janitor/target/debug/deps/libjanitor_worker-dda3a9755d05977a.rlib --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rlib --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rlib --extern nix=/home/jelmer/src/janitor/target/debug/deps/libnix-6b4ef5f907bfc73a.rlib --extern ognibuild=/home/jelmer/src/janitor/target/debug/deps/libognibuild-87189c762fb2be0d.rlib --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rlib --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-675a227af86df9e2.rlib --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rlib --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-632241dddb9114dd.rlib --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rlib --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rlib --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rlib --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rlib --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-99004d4ad89f4c8e.rlib --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rlib --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rlib --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-0594be3a9e82e568.rlib --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rlib --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out -L native=/home/jelmer/src/janitor/target/debug/build/psm-d70ac1db1779e382/out`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name generic_build --edition=2021 worker/src/bin/generic-build.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default"))' -C metadata=7536fb31c407fef0 -C extra-filename=-9569e2a67062c1b2 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e3513f3c2c846e6a.rlib --extern askama_axum=/home/jelmer/src/janitor/target/debug/deps/libaskama_axum-e8895333fc0a961c.rlib --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-8fb5e3339fd3daf6.rlib --extern backoff=/home/jelmer/src/janitor/target/debug/deps/libbackoff-7aa1f850c4954588.rlib --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rlib --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rlib --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rlib --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-5b67a9919d78661e.rlib --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rlib --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rlib --extern gethostname=/home/jelmer/src/janitor/target/debug/deps/libgethostname-ae3ed00e9ff91cfd.rlib --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-2b9d2a508e000f49.rlib --extern janitor_worker=/home/jelmer/src/janitor/target/debug/deps/libjanitor_worker-dda3a9755d05977a.rlib --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rlib --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rlib --extern nix=/home/jelmer/src/janitor/target/debug/deps/libnix-6b4ef5f907bfc73a.rlib --extern ognibuild=/home/jelmer/src/janitor/target/debug/deps/libognibuild-87189c762fb2be0d.rlib --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rlib --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-675a227af86df9e2.rlib --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rlib --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-632241dddb9114dd.rlib --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rlib --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rlib --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rlib --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rlib --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-99004d4ad89f4c8e.rlib --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rlib --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rlib --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-0594be3a9e82e568.rlib --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rlib --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out -L native=/home/jelmer/src/janitor/target/debug/build/psm-d70ac1db1779e382/out`
warning: unused variable: `packaging_tree`
   --> worker/src/bin/dist.rs:101:10
    |
101 |     let (packaging_tree, packaging_debian_path) = if let Some(packaging) = args.packaging {
    |          ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_packaging_tree`
    |
    = note: `#[warn(unused_variables)]` on by default

warning: unused variable: `packaging_debian_path`
   --> worker/src/bin/dist.rs:101:26
    |
101 |     let (packaging_tree, packaging_debian_path) = if let Some(packaging) = args.packaging {
    |                          ^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_packaging_debian_path`

warning: unused variable: `retcode`
   --> worker/src/bin/dist.rs:209:13
    |
209 |             retcode,
    |             ^^^^^^^ help: try ignoring the field: `retcode: _`

warning: unused variable: `e`
   --> worker/src/bin/dist.rs:233:43
    |
233 |         Err(Error::DependencyInstallError(e)) => {
    |                                           ^ help: if this is intentional, prefix it with an underscore: `_e`

warning: `janitor-worker` (bin "janitor-dist") generated 4 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 35.29s
Copying rust artifact from target/debug/janitor-worker to build/scripts-3.13/janitor-worker
cargo build --manifest-path worker/Cargo.toml --message-format=json-render-diagnostics -v --features cli debian
       Fresh unicode-ident v1.0.18
       Fresh cfg-if v1.0.0
       Fresh memchr v2.7.4
       Fresh autocfg v1.4.0
       Fresh once_cell v1.21.0
       Fresh value-bag v1.10.0
       Fresh regex-syntax v0.8.5
       Fresh pin-project-lite v0.2.16
       Fresh allocator-api2 v0.2.21
       Fresh bitflags v2.9.0
       Fresh scopeguard v1.2.0
       Fresh futures-core v0.3.31
       Fresh itoa v1.0.15
       Fresh foldhash v0.1.4
       Fresh equivalent v1.0.2
       Fresh version_check v0.9.5
       Fresh shlex v1.3.0
       Fresh futures-io v0.3.31
       Fresh fastrand v2.3.0
       Fresh stable_deref_trait v1.2.0
       Fresh bytes v1.10.1
       Fresh litemap v0.7.5
       Fresh writeable v0.5.5
       Fresh icu_locid_transform_data v1.5.0
       Fresh proc-macro2 v1.0.94
       Fresh hashbrown v0.15.2
       Fresh cc v1.2.16
       Fresh tracing-core v0.1.33
       Fresh percent-encoding v2.3.1
       Fresh icu_properties_data v1.5.0
       Fresh utf8_iter v1.0.4
       Fresh utf16_iter v1.0.5
       Fresh icu_normalizer_data v1.5.0
       Fresh write16 v1.0.0
       Fresh ryu v1.0.20
       Fresh pin-utils v0.1.0
       Fresh home v0.5.11
       Fresh log v0.4.27
       Fresh pkg-config v0.3.32
       Fresh vcpkg v0.2.15
       Fresh futures-task v0.3.31
       Fresh quote v1.0.39
       Fresh libc v0.2.170
       Fresh zerocopy v0.8.23
       Fresh crossbeam-utils v0.8.21
       Fresh atomic-waker v1.1.2
       Fresh parking v2.2.1
       Fresh tinyvec_macros v0.1.1
       Fresh foreign-types-shared v0.1.1
       Fresh iana-time-zone v0.1.61
       Fresh subtle v2.6.1
       Fresh aho-corasick v1.1.3
       Fresh linux-raw-sys v0.9.2
       Fresh siphasher v1.0.1
       Fresh openssl-probe v0.1.6
       Fresh syn v2.0.100
       Fresh lock_api v0.4.12
       Fresh slab v0.4.9
       Fresh ppv-lite86 v0.2.21
       Fresh typenum v1.18.0
       Fresh concurrent-queue v2.5.0
       Fresh signal-hook-registry v1.4.2
       Fresh tinyvec v1.9.0
       Fresh foreign-types v0.3.2
       Fresh regex-automata v0.4.9
       Fresh phf_shared v0.11.3
       Fresh cpufeatures v0.2.17
       Fresh futures-lite v2.6.0
       Fresh zerocopy v0.7.35
       Fresh heck v0.5.0
       Fresh bitflags v1.3.2
       Fresh async-task v4.7.1
       Fresh event-listener v2.5.3
       Fresh serde_derive v1.0.219
       Fresh synstructure v0.13.1
       Fresh thiserror-impl v2.0.12
       Fresh displaydoc v0.2.5
       Fresh zerovec-derive v0.10.3
       Fresh icu_provider_macros v1.5.0
       Fresh tracing-attributes v0.1.28
       Fresh generic-array v0.14.7
       Fresh tokio-macros v2.5.0
       Fresh futures-macro v0.3.31
       Fresh openssl-macros v0.1.1
       Fresh target-lexicon v0.12.16
       Fresh rustix v1.0.2
       Fresh unicode-normalization v0.1.24
       Fresh regex v1.11.1
       Fresh event-listener v5.4.0
       Fresh ahash v0.8.11
       Fresh piper v0.2.4
       Fresh async-executor v1.13.1
       Fresh getrandom v0.2.15
       Fresh linux-raw-sys v0.3.8
       Fresh bstr v1.11.3
       Fresh serde v1.0.219
       Fresh zerofrom-derive v0.1.6
       Fresh yoke-derive v0.7.5
       Fresh thiserror v2.0.12
       Fresh crypto-common v0.1.6
       Fresh block-buffer v0.10.4
       Fresh event-listener-strategy v0.5.3
       Fresh hashbrown v0.14.5
       Fresh waker-fn v1.2.0
       Fresh fastrand v1.9.0
       Fresh tracing v0.1.41
       Fresh async-channel v1.9.0
       Fresh async-lock v2.8.0
       Fresh unicase v2.8.1
       Fresh fnv v1.0.7
       Fresh powerfmt v0.2.0
       Fresh crc-catalog v2.4.0
       Fresh num-conv v0.1.0
       Fresh time-core v0.1.3
       Fresh same-file v1.0.6
       Fresh zerofrom v0.1.6
       Fresh indexmap v2.8.0
       Fresh serde_json v1.0.140
       Fresh either v1.15.0
       Fresh digest v0.10.7
       Fresh smallvec v1.14.0
       Fresh async-lock v3.4.0
       Fresh async-channel v2.3.1
       Fresh futures-lite v1.13.0
       Fresh linux-raw-sys v0.4.15
       Fresh crc v3.2.1
       Fresh walkdir v2.5.0
       Fresh time-macros v0.2.20
       Fresh deranged v0.3.11
       Fresh http v1.2.0
       Fresh thiserror-impl v1.0.69
       Fresh openssl-sys v0.9.107
       Fresh socket2 v0.5.8
       Fresh mio v1.0.3
       Fresh crossbeam-queue v0.3.12
       Fresh yoke v0.7.5
       Fresh pyo3-build-config v0.22.6
       Fresh parking_lot_core v0.9.10
       Fresh blocking v1.6.1
       Fresh phf_generator v0.11.3
       Fresh sha2 v0.10.8
       Fresh rustix v0.38.44
       Fresh num-traits v0.2.19
       Fresh time v0.3.39
       Fresh thiserror v1.0.69
       Fresh hashlink v0.10.0
       Fresh futures-sink v0.3.31
       Fresh hmac v0.12.1
       Fresh openssl v0.10.72
       Fresh http-body v1.0.1
       Fresh rand_core v0.6.4
       Fresh form_urlencoded v1.2.1
       Fresh unindent v0.2.4
       Fresh indoc v2.0.6
       Fresh hex v0.4.3
       Fresh unicode-bidi v0.3.18
       Fresh mime v0.3.17
       Fresh unicode-properties v0.1.3
       Fresh zerovec v0.10.4
       Fresh parking_lot v0.12.3
       Fresh memoffset v0.9.1
       Fresh polling v3.7.4
       Fresh native-tls v0.2.14
       Fresh hkdf v0.12.4
       Fresh stringprep v0.1.5
       Fresh chrono v0.4.40
       Fresh rand_chacha v0.3.1
       Fresh futures-util v0.3.31
       Fresh md-5 v0.10.6
       Fresh io-lifetimes v1.0.11
       Fresh whoami v1.5.2
       Fresh tower-service v0.3.3
       Fresh dotenvy v0.15.7
       Fresh futures-channel v0.3.31
       Fresh polling v2.8.0
       Fresh socket2 v0.4.10
       Fresh kv-log-macro v1.0.7
       Fresh base64 v0.22.1
       Fresh rustc-hash v1.1.0
       Fresh tinystr v0.7.6
       Fresh icu_collections v1.5.0
       Fresh tokio v1.44.2
       Fresh async-io v2.4.0
       Fresh rand v0.8.5
       Fresh rustix v0.37.28
       Fresh text-size v1.1.1
       Fresh countme v3.0.1
       Fresh new_debug_unreachable v1.0.6
       Fresh try-lock v0.2.5
       Fresh futures-intrusive v0.5.0
       Fresh http-body-util v0.1.3
       Fresh encoding_rs v0.8.35
       Fresh sync_wrapper v1.0.2
       Fresh httpdate v1.0.3
       Fresh tower-layer v0.3.3
       Fresh byteorder v1.5.0
       Fresh lazy-regex-proc_macros v3.4.1
       Fresh atoi v2.0.0
       Fresh precomputed-hash v0.1.1
       Fresh gix-trace v0.1.12
       Fresh siphasher v0.3.11
       Fresh icu_locid v1.5.0
       Fresh pyo3-macros-backend v0.22.6
       Fresh pyo3-ffi v0.22.6
       Fresh async-global-executor v2.4.1
       Fresh tokio-util v0.7.14
       Fresh httparse v1.10.1
       Fresh tokio-stream v0.1.17
       Fresh async-io v1.13.0
       Fresh want v0.3.1
       Fresh getrandom v0.3.1
       Fresh rowan v0.16.1
       Fresh lazy-regex v3.4.1
       Fresh tower v0.5.2
       Fresh gix-path v0.10.15
       Fresh tokio-native-tls v0.3.1
       Fresh string_cache_codegen v0.5.4
       Fresh phf_codegen v0.11.3
       Fresh serde_urlencoded v0.7.1
       Fresh gix-utils v0.2.0
       Fresh unicode-xid v0.2.6
       Fresh rustls-pki-types v1.11.0
       Fresh adler2 v2.0.0
       Fresh prodash v29.0.1
       Fresh deb822-derive v0.2.0
       Fresh winnow v0.7.3
       Fresh unicode-width v0.2.0
       Fresh icu_provider v1.5.0
       Fresh pyo3-macros v0.22.6
       Fresh h2 v0.4.8
       Fresh async-std v1.13.1
       Fresh syn v1.0.109
       Fresh rustls-pemfile v2.2.0
       Fresh miniz_oxide v0.8.5
       Fresh anyhow v1.0.97
       Fresh crunchy v0.2.3
       Fresh ipnet v2.11.0
       Fresh gix-features v0.41.1
       Fresh tempfile v3.19.0
       Fresh sha1 v0.10.6
       Fresh mac v0.1.1
       Fresh utf8parse v0.2.2
       Fresh jiff v0.2.4
       Fresh async-trait v0.1.88
       Fresh filetime v0.2.25
       Fresh is_terminal_polyfill v1.70.1
       Fresh colorchoice v1.0.3
       Fresh lazy_static v1.5.0
       Fresh utf-8 v0.7.6
       Fresh ucd-trie v0.1.7
       Fresh anstyle-query v1.1.2
       Fresh anstyle v1.0.10
       Fresh inotify-sys v0.1.5
       Fresh dirs-sys-next v0.1.2
       Fresh icu_locid_transform v1.5.0
       Fresh pyo3 v0.22.6
       Fresh hyper v1.6.0
       Fresh tiny-keccak v2.0.2
       Fresh phf_generator v0.10.0
       Fresh futf v0.1.5
       Fresh anstyle-parse v0.2.6
       Fresh pest v2.7.15
       Fresh crc32fast v1.4.2
       Fresh gimli v0.31.1
       Fresh unicode-linebreak v0.1.5
       Fresh smawk v0.3.2
       Fresh inotify v0.9.6
       Fresh synstructure v0.12.6
       Fresh dirs-next v2.0.0
       Fresh phf_shared v0.10.0
       Fresh serde_spanned v0.6.8
       Fresh toml_datetime v0.6.8
       Fresh mio v0.8.11
       Fresh crossbeam-channel v0.5.15
       Fresh unsafe-libyaml v0.2.11
       Fresh clap_lex v0.7.4
       Fresh strsim v0.11.1
       Fresh rustc-demangle v0.1.24
       Fresh urlencoding v2.1.3
       Fresh dtor-proc-macro v0.0.5
       Fresh icu_properties v1.5.1
       Fresh hyper-util v0.1.10
       Fresh deb822-lossless v0.2.4
       Fresh anstream v0.6.18
       Fresh tendril v0.4.3
       Fresh phf_codegen v0.10.0
       Fresh object v0.36.7
       Fresh textwrap v0.16.2
       Fresh addr2line v0.24.2
       Fresh pest_meta v2.7.15
       Fresh term v0.7.0
       Fresh toml_edit v0.22.24
       Fresh dtor v0.0.5
       Fresh serde_yaml v0.9.34+deprecated
       Fresh flate2 v1.1.0
       Fresh phf v0.10.1
       Fresh notify v6.1.1
       Fresh sha1-checked v0.10.0
       Fresh rowan v0.15.16
       Fresh protobuf-support v3.7.2
       Fresh version-ranges v0.1.1
       Fresh which v4.4.2
       Fresh faster-hex v0.9.0
       Fresh parse-zoneinfo v0.3.1
       Fresh clap_derive v4.5.32
       Fresh phf v0.11.3
       Fresh csv-core v0.1.12
       Fresh icu_normalizer v1.5.0
       Fresh hyper-tls v0.6.0
       Fresh failure_derive v0.1.8
       Fresh clap_builder v4.5.36
       Fresh backtrace v0.3.74
       Fresh pest_generator v2.7.15
       Fresh deunicode v1.6.0
       Fresh unscanny v0.1.0
       Fresh difflib v0.4.0
       Fresh maplit v1.0.2
       Fresh fixedbitset v0.4.2
       Fresh ctor-proc-macro v0.0.5
       Fresh libm v0.2.11
       Fresh gix-hash v0.17.0
       Fresh chrono-tz-build v0.3.0
       Fresh csv v1.3.1
       Fresh ascii-canvas v3.0.0
       Fresh dirty-tracker v0.3.0
       Fresh protobuf v3.7.2
       Fresh psm v0.1.25
       Fresh pyo3-filelike v0.4.1
       Fresh const-random-macro v0.1.16
       Fresh patchkit v0.2.1
       Fresh gix-date v0.9.4
       Fresh gix-fs v0.14.0
       Fresh idna_adapter v1.2.0
       Fresh failure v0.1.8
       Fresh pest_derive v2.7.15
       Fresh clap v4.5.36
       Fresh slug v0.1.6
       Fresh ctor v0.4.1
       Fresh pep440_rs v0.7.3
       Fresh petgraph v0.6.5
       Fresh charset v0.1.5
       Fresh string_cache v0.8.8
       Fresh itertools v0.10.5
       Fresh itertools v0.13.0
       Fresh num-integer v0.1.46
       Fresh crossbeam-epoch v0.9.18
       Fresh is-terminal v0.4.16
       Fresh ena v0.14.3
       Fresh diff v0.1.13
       Fresh minimal-lexical v0.2.1
       Fresh xml-rs v0.8.25
       Fresh regex-syntax v0.6.29
       Fresh rustc-hash v2.1.1
       Fresh unic-char-range v0.9.0
       Fresh unic-common v0.9.0
       Fresh idna v1.0.3
       Fresh markup5ever v0.11.0
       Fresh quoted_printable v0.5.1
       Fresh base64ct v1.7.1
       Fresh boxcar v0.2.10
       Fresh num-bigint v0.4.6
       Fresh unic-ucd-version v0.9.0
       Fresh crossbeam-deque v0.8.6
       Fresh lalrpop v0.19.12
       Fresh unic-char-property v0.9.0
       Fresh distro-info v0.4.0
       Fresh semver v1.0.26
       Fresh nom v7.1.3
       Fresh protobuf v2.28.0
       Fresh stacker v0.1.19
       Fresh gix-actor v0.34.0
       Fresh const-random v0.1.18
       Fresh humansize v2.1.3
       Fresh gix-hashtable v0.8.0
       Fresh gix-tempfile v17.0.0
       Fresh makefile-lossless v0.1.7
       Fresh gix-validate v0.9.4
       Fresh globset v0.4.16
       Fresh env_filter v0.1.3
       Fresh url v2.5.4
       Fresh pem-rfc7468 v0.7.0
       Fresh atty v0.2.14
       Fresh untrusted v0.9.0
       Fresh quick-error v1.2.3
       Fresh data-encoding v2.8.0
       Fresh lockfree-object-pool v0.1.6
       Fresh configparser v3.1.0
       Fresh bit-vec v0.8.0
       Fresh simd-adler32 v0.3.7
       Fresh termcolor v1.4.1
       Fresh bumpalo v3.17.0
       Fresh zeroize v1.8.1
       Fresh const-oid v0.9.6
       Fresh protobuf-codegen v2.28.0
       Fresh html5ever v0.26.0
       Fresh chumsky v0.9.3
       Fresh mime_guess v2.0.5
       Fresh ignore v0.4.23
       Fresh rustversion v1.0.20
       Fresh unic-ucd-segment v0.9.0
       Fresh simple_asn1 v0.6.3
       Fresh xml5ever v0.17.0
       Fresh gix-lock v17.0.0
       Fresh env_logger v0.11.7
       Fresh sqlx-core v0.8.3
       Fresh reqwest v0.12.15
       Fresh pep508_rs v0.9.2
       Fresh humantime v1.3.0
       Fresh der v0.7.9
       Fresh mailparse v0.15.0
       Fresh ring v0.17.13
       Fresh dep3 v0.1.28
       Fresh zopfli v0.8.1
       Fresh bit-set v0.8.0
       Fresh document_tree v0.4.1
       Fresh askama_parser v0.2.1
       Fresh gix-object v0.48.0
       Fresh dlv-list v0.5.2
       Fresh merge3 v0.2.0
       Fresh protobuf-parse v3.7.2
       Fresh protoc v2.28.0
       Fresh rand_core v0.9.3
       Fresh futures-executor v0.3.31
       Fresh basic-toml v0.1.10
       Fresh xattr v1.5.0
       Fresh pem v3.0.5
       Fresh memmap2 v0.9.5
       Fresh unicode-width v0.1.14
       Fresh bit-vec v0.6.3
       Fresh unicode_categories v0.1.1
       Fresh entities v1.0.1
       Fresh typed-arena v2.0.2
       Fresh sqlx-postgres v0.8.3
       Fresh cfg_aliases v0.2.1
       Fresh env_logger v0.7.1
       Fresh ordered-multimap v0.7.3
       Fresh fs-err v3.1.0
       Fresh zip v2.4.1
       Fresh spki v0.7.3
       Fresh protoc-rust v2.28.0
       Fresh comrak v0.18.0
       Fresh rand_chacha v0.9.0
       Fresh askama_derive v0.12.5
       Fresh google-cloud-metadata v0.5.1
       Fresh getopts v0.2.21
       Fresh tar v0.4.44
       Fresh fancy-regex v0.14.0
       Fresh protobuf-codegen v3.7.2
       Fresh futures v0.3.31
       Fresh bit-set v0.5.3
       Fresh jsonwebtoken v9.3.1
       Fresh gix-ref v0.51.0
       Fresh chrono-tz v0.9.0
       Fresh rfc2047-decoder v1.0.6
       Fresh axum-core v0.4.5
       Fresh markup5ever_rcdom v0.2.0
       Fresh globwalk v0.9.1
       Fresh unic-segment v0.9.0
       Fresh markup5ever v0.14.1
       Fresh serde-xml-rs v0.5.1
       Fresh xmltree v0.11.0
       Fresh sqlx-macros-core v0.8.3
       Fresh toml v0.8.20
       Fresh google-cloud-token v0.1.2
       Fresh gix-glob v0.19.0
       Fresh gix-config-value v0.14.12
       Fresh async-stream-impl v0.3.6
       Fresh m_lexer v0.0.4
       Fresh match_token v0.1.0
       Fresh gix-sec v0.10.12
       Fresh static_assertions v1.1.0
       Fresh askama_escape v0.10.3
       Fresh pulldown-cmark-escape v0.11.0
       Fresh humantime v2.1.0
       Fresh lalrpop-util v0.19.12
       Fresh base64 v0.21.7
       Fresh inventory v0.3.20
       Fresh option-ext v0.2.0
       Fresh trim-in-place v0.1.7
       Fresh unicode-bom v2.0.3
       Fresh rst_renderer v0.4.1
       Fresh rand v0.9.0
       Fresh pkcs8 v0.10.2
       Fresh select v0.6.1
       Fresh python-pkginfo v0.6.5
       Fresh pretty_env_logger v0.4.0
       Fresh tera v1.20.0
       Fresh reqwest-middleware v0.3.3
       Fresh uo_rst_parser v0.4.3
       Fresh sqlx-macros v0.8.3
       Fresh google-cloud-auth v0.17.2
       Fresh async-stream v0.3.6
       Fresh rust-ini v0.21.1
       Fresh pulldown-cmark v0.13.0
       Fresh askama v0.12.1
       Fresh gix-config v0.44.0
       Fresh pyproject-toml v0.13.4
       Fresh dirs-sys v0.4.1
       Fresh html5ever v0.29.1
       Fresh opam-file-rs v0.1.5
       Fresh env_logger v0.9.3
       Fresh twox-hash v1.6.3
       Fresh r-description v0.3.1
       Fresh toml v0.5.11
       Fresh serde_path_to_error v0.1.17
       Fresh xdg v2.5.2
       Fresh matchit v0.7.3
       Dirty janitor v0.1.0 (/home/jelmer/src/janitor): the precalculated components changed
   Compiling janitor v0.1.0 (/home/jelmer/src/janitor)
       Fresh async-compression v0.4.23
       Fresh lzma-rs v0.3.0
       Fresh instant v0.1.13
       Fresh arc-swap v1.7.1
       Fresh fs_extra v1.3.0
       Fresh gethostname v1.0.0
     Running `/home/jelmer/src/janitor/target/debug/build/janitor-3e4912ad33cb41bf/build-script-build`
       Fresh sqlx v0.8.3
       Fresh axum v0.7.9
       Fresh google-cloud-storage v0.22.1
       Fresh nix v0.29.0
       Fresh prometheus v0.14.0
       Fresh dirs v5.0.1
       Fresh stackdriver_logger v0.8.2
       Fresh lz4_flex v0.11.3
       Fresh pyo3-log v0.11.0
       Fresh askama_axum v0.4.0
       Fresh backoff v0.4.0
       Fresh debversion v0.4.4
       Fresh debian-control v0.1.41
       Fresh debian-changelog v0.2.0
       Fresh debian-copyright v0.1.27
       Fresh debian-watch v0.2.8
       Fresh debbugs v0.1.5
       Fresh breezyshim v0.1.227
       Fresh buildlog-consultant v0.1.1
       Fresh debian-analyzer v0.158.25
       Fresh upstream-ontologist v0.2.2
       Fresh silver-platter v0.5.48
       Fresh ognibuild v0.0.33
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor --edition=2021 src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="debian"' --cfg 'feature="default"' --cfg 'feature="gcp"' --cfg 'feature="gcs"' --cfg 'feature="stackdriver_logger"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("debian", "default", "gcp", "gcs", "stackdriver_logger"))' -C metadata=85bae1026071c99d -C extra-filename=-2b9d2a508e000f49 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e3513f3c2c846e6a.rmeta --extern async_compression=/home/jelmer/src/janitor/target/debug/deps/libasync_compression-010e9930c22df287.rmeta --extern async_trait=/home/jelmer/src/janitor/target/debug/deps/libasync_trait-a0e4f95a72127984.so --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rmeta --extern buildlog_consultant=/home/jelmer/src/janitor/target/debug/deps/libbuildlog_consultant-5d9feac314f608bd.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_control=/home/jelmer/src/janitor/target/debug/deps/libdebian_control-ff757dbc033f80ab.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern env_logger=/home/jelmer/src/janitor/target/debug/deps/libenv_logger-6abdb84b0421fecc.rmeta --extern fancy_regex=/home/jelmer/src/janitor/target/debug/deps/libfancy_regex-0688edb11485e39a.rmeta --extern filetime=/home/jelmer/src/janitor/target/debug/deps/libfiletime-8f90a18bbe2253cd.rmeta --extern flate2=/home/jelmer/src/janitor/target/debug/deps/libflate2-8f750e2fad1f4e3a.rmeta --extern futures=/home/jelmer/src/janitor/target/debug/deps/libfutures-463765cbcde647a0.rmeta --extern google_cloud_auth=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_auth-8fbb47d89a739226.rmeta --extern google_cloud_storage=/home/jelmer/src/janitor/target/debug/deps/libgoogle_cloud_storage-0c6e211063707140.rmeta --extern lazy_regex=/home/jelmer/src/janitor/target/debug/deps/liblazy_regex-b6dba2d53203475e.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-675a227af86df9e2.rmeta --extern protobuf=/home/jelmer/src/janitor/target/debug/deps/libprotobuf-b1e3dc89f529da22.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rmeta --extern regex=/home/jelmer/src/janitor/target/debug/deps/libregex-4d79fad2269d97a5.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-99004d4ad89f4c8e.rmeta --extern sqlx=/home/jelmer/src/janitor/target/debug/deps/libsqlx-4aede0cee3bfd326.rmeta --extern sqlx_core=/home/jelmer/src/janitor/target/debug/deps/libsqlx_core-889a5213dce729ff.rmeta --extern sqlx_postgres=/home/jelmer/src/janitor/target/debug/deps/libsqlx_postgres-67cde0941f4b68ff.rmeta --extern stackdriver_logger=/home/jelmer/src/janitor/target/debug/deps/libstackdriver_logger-b45c27ef10c05d41.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern tracing=/home/jelmer/src/janitor/target/debug/deps/libtracing-0986249cd85cd0a5.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out`
warning: function `reprocess_run_logs` is never used
 --> src/reprocess_logs.rs:8:10
  |
8 | async fn reprocess_run_logs(
  |          ^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(dead_code)]` on by default

warning: field `branch_url` is never read
  --> src/schedule.rs:32:5
   |
30 | pub struct ScheduleRequest {
   |            --------------- field in this struct
31 |     codebase: String,
32 |     branch_url: String,
   |     ^^^^^^^^^^

warning: function `has_cotenants` is never used
  --> src/state.rs:80:10
   |
80 | async fn has_cotenants(
   |          ^^^^^^^^^^^^^

warning: field `name` is never read
  --> src/state.rs:87:13
   |
86 |     struct Codebase {
   |            -------- field in this struct
87 |         pub name: String,
   |             ^^^^
   |
   = note: `Codebase` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: function `iter_publishable_suites` is never used
   --> src/state.rs:113:10
    |
113 | async fn iter_publishable_suites(
    |          ^^^^^^^^^^^^^^^^^^^^^^^

       Dirty janitor-worker v0.0.0 (/home/jelmer/src/janitor/worker): dependency info changed
   Compiling janitor-worker v0.0.0 (/home/jelmer/src/janitor/worker)
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor_worker --edition=2021 worker/src/lib.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type lib --emit=dep-info,metadata,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default"))' -C metadata=6bffe1f7d32befef -C extra-filename=-dda3a9755d05977a --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e3513f3c2c846e6a.rmeta --extern askama_axum=/home/jelmer/src/janitor/target/debug/deps/libaskama_axum-e8895333fc0a961c.rmeta --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-8fb5e3339fd3daf6.rmeta --extern backoff=/home/jelmer/src/janitor/target/debug/deps/libbackoff-7aa1f850c4954588.rmeta --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rmeta --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rmeta --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rmeta --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-5b67a9919d78661e.rmeta --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rmeta --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rmeta --extern gethostname=/home/jelmer/src/janitor/target/debug/deps/libgethostname-ae3ed00e9ff91cfd.rmeta --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-2b9d2a508e000f49.rmeta --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rmeta --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rmeta --extern nix=/home/jelmer/src/janitor/target/debug/deps/libnix-6b4ef5f907bfc73a.rmeta --extern ognibuild=/home/jelmer/src/janitor/target/debug/deps/libognibuild-87189c762fb2be0d.rmeta --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rmeta --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-675a227af86df9e2.rmeta --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rmeta --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-632241dddb9114dd.rmeta --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rmeta --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rmeta --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rmeta --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rmeta --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-99004d4ad89f4c8e.rmeta --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rmeta --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rmeta --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-0594be3a9e82e568.rmeta --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rmeta --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out -L native=/home/jelmer/src/janitor/target/debug/build/psm-d70ac1db1779e382/out`
warning: `janitor` (lib) generated 5 warnings
warning: unused variable: `sys_path`
  --> worker/src/debian/mod.rs:74:9
   |
74 |     let sys_path = pyo3::Python::with_gil(|py| {
   |         ^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_sys_path`
   |
   = note: `#[warn(unused_variables)]` on by default

warning: unused variable: `sys_executable`
  --> worker/src/debian/mod.rs:86:9
   |
86 |     let sys_executable = pyo3::Python::with_gil(|py| {
   |         ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_sys_executable`

warning: unused variable: `subpath`
   --> worker/src/generic/mod.rs:137:5
    |
137 |     subpath: &Path,
    |     ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_subpath`

warning: unused variable: `default_empty`
   --> worker/src/lib.rs:253:5
    |
253 |     default_empty: Option<bool>,
    |     ^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_default_empty`

warning: function `derive_branch_name` is never used
   --> worker/src/lib.rs:944:4
    |
944 | fn derive_branch_name(url: &url::Url) -> String {
    |    ^^^^^^^^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` on by default

warning: `janitor-worker` (lib) generated 5 warnings
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor_dist --edition=2021 worker/src/bin/dist.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default"))' -C metadata=1a2e9bb3851eb08f -C extra-filename=-3e450b078b72919b --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e3513f3c2c846e6a.rlib --extern askama_axum=/home/jelmer/src/janitor/target/debug/deps/libaskama_axum-e8895333fc0a961c.rlib --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-8fb5e3339fd3daf6.rlib --extern backoff=/home/jelmer/src/janitor/target/debug/deps/libbackoff-7aa1f850c4954588.rlib --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rlib --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rlib --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rlib --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-5b67a9919d78661e.rlib --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rlib --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rlib --extern gethostname=/home/jelmer/src/janitor/target/debug/deps/libgethostname-ae3ed00e9ff91cfd.rlib --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-2b9d2a508e000f49.rlib --extern janitor_worker=/home/jelmer/src/janitor/target/debug/deps/libjanitor_worker-dda3a9755d05977a.rlib --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rlib --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rlib --extern nix=/home/jelmer/src/janitor/target/debug/deps/libnix-6b4ef5f907bfc73a.rlib --extern ognibuild=/home/jelmer/src/janitor/target/debug/deps/libognibuild-87189c762fb2be0d.rlib --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rlib --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-675a227af86df9e2.rlib --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rlib --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-632241dddb9114dd.rlib --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rlib --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rlib --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rlib --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rlib --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-99004d4ad89f4c8e.rlib --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rlib --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rlib --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-0594be3a9e82e568.rlib --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rlib --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out -L native=/home/jelmer/src/janitor/target/debug/build/psm-d70ac1db1779e382/out`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name debian_build --edition=2021 worker/src/bin/debian-build.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default"))' -C metadata=6a95fbfa58c6ea57 -C extra-filename=-43cdaca24b785e9a --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e3513f3c2c846e6a.rlib --extern askama_axum=/home/jelmer/src/janitor/target/debug/deps/libaskama_axum-e8895333fc0a961c.rlib --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-8fb5e3339fd3daf6.rlib --extern backoff=/home/jelmer/src/janitor/target/debug/deps/libbackoff-7aa1f850c4954588.rlib --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rlib --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rlib --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rlib --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-5b67a9919d78661e.rlib --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rlib --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rlib --extern gethostname=/home/jelmer/src/janitor/target/debug/deps/libgethostname-ae3ed00e9ff91cfd.rlib --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-2b9d2a508e000f49.rlib --extern janitor_worker=/home/jelmer/src/janitor/target/debug/deps/libjanitor_worker-dda3a9755d05977a.rlib --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rlib --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rlib --extern nix=/home/jelmer/src/janitor/target/debug/deps/libnix-6b4ef5f907bfc73a.rlib --extern ognibuild=/home/jelmer/src/janitor/target/debug/deps/libognibuild-87189c762fb2be0d.rlib --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rlib --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-675a227af86df9e2.rlib --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rlib --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-632241dddb9114dd.rlib --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rlib --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rlib --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rlib --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rlib --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-99004d4ad89f4c8e.rlib --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rlib --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rlib --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-0594be3a9e82e568.rlib --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rlib --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out -L native=/home/jelmer/src/janitor/target/debug/build/psm-d70ac1db1779e382/out`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name janitor_worker --edition=2021 worker/src/bin/worker.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default"))' -C metadata=f562e6cffbec47e0 -C extra-filename=-f44d9bfe5e314590 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e3513f3c2c846e6a.rlib --extern askama_axum=/home/jelmer/src/janitor/target/debug/deps/libaskama_axum-e8895333fc0a961c.rlib --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-8fb5e3339fd3daf6.rlib --extern backoff=/home/jelmer/src/janitor/target/debug/deps/libbackoff-7aa1f850c4954588.rlib --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rlib --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rlib --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rlib --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-5b67a9919d78661e.rlib --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rlib --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rlib --extern gethostname=/home/jelmer/src/janitor/target/debug/deps/libgethostname-ae3ed00e9ff91cfd.rlib --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-2b9d2a508e000f49.rlib --extern janitor_worker=/home/jelmer/src/janitor/target/debug/deps/libjanitor_worker-dda3a9755d05977a.rlib --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rlib --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rlib --extern nix=/home/jelmer/src/janitor/target/debug/deps/libnix-6b4ef5f907bfc73a.rlib --extern ognibuild=/home/jelmer/src/janitor/target/debug/deps/libognibuild-87189c762fb2be0d.rlib --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rlib --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-675a227af86df9e2.rlib --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rlib --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-632241dddb9114dd.rlib --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rlib --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rlib --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rlib --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rlib --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-99004d4ad89f4c8e.rlib --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rlib --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rlib --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-0594be3a9e82e568.rlib --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rlib --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out -L native=/home/jelmer/src/janitor/target/debug/build/psm-d70ac1db1779e382/out`
     Running `/home/jelmer/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc --crate-name generic_build --edition=2021 worker/src/bin/generic-build.rs --error-format=json --json=diagnostic-rendered-ansi,artifacts,future-incompat --crate-type bin --emit=dep-info,link -C embed-bitcode=no -C debuginfo=2 --cfg 'feature="cli"' --cfg 'feature="debian"' --cfg 'feature="default"' --check-cfg 'cfg(docsrs,test)' --check-cfg 'cfg(feature, values("cli", "debian", "default"))' -C metadata=7536fb31c407fef0 -C extra-filename=-9569e2a67062c1b2 --out-dir /home/jelmer/src/janitor/target/debug/deps -C incremental=/home/jelmer/src/janitor/target/debug/incremental -L dependency=/home/jelmer/src/janitor/target/debug/deps --extern askama=/home/jelmer/src/janitor/target/debug/deps/libaskama-e3513f3c2c846e6a.rlib --extern askama_axum=/home/jelmer/src/janitor/target/debug/deps/libaskama_axum-e8895333fc0a961c.rlib --extern axum=/home/jelmer/src/janitor/target/debug/deps/libaxum-8fb5e3339fd3daf6.rlib --extern backoff=/home/jelmer/src/janitor/target/debug/deps/libbackoff-7aa1f850c4954588.rlib --extern breezyshim=/home/jelmer/src/janitor/target/debug/deps/libbreezyshim-76cd14e9b060156e.rlib --extern chrono=/home/jelmer/src/janitor/target/debug/deps/libchrono-14ed257ce817e461.rlib --extern clap=/home/jelmer/src/janitor/target/debug/deps/libclap-59b401794f276823.rlib --extern debian_analyzer=/home/jelmer/src/janitor/target/debug/deps/libdebian_analyzer-5b67a9919d78661e.rlib --extern debian_changelog=/home/jelmer/src/janitor/target/debug/deps/libdebian_changelog-fbf10812cd9c3fd9.rlib --extern debversion=/home/jelmer/src/janitor/target/debug/deps/libdebversion-2bb3e4a33b8b8241.rlib --extern gethostname=/home/jelmer/src/janitor/target/debug/deps/libgethostname-ae3ed00e9ff91cfd.rlib --extern janitor=/home/jelmer/src/janitor/target/debug/deps/libjanitor-2b9d2a508e000f49.rlib --extern janitor_worker=/home/jelmer/src/janitor/target/debug/deps/libjanitor_worker-dda3a9755d05977a.rlib --extern log=/home/jelmer/src/janitor/target/debug/deps/liblog-da043839f6c60ee2.rlib --extern maplit=/home/jelmer/src/janitor/target/debug/deps/libmaplit-a7afc18e3018525b.rlib --extern nix=/home/jelmer/src/janitor/target/debug/deps/libnix-6b4ef5f907bfc73a.rlib --extern ognibuild=/home/jelmer/src/janitor/target/debug/deps/libognibuild-87189c762fb2be0d.rlib --extern percent_encoding=/home/jelmer/src/janitor/target/debug/deps/libpercent_encoding-9e7a562fefdd4c61.rlib --extern prometheus=/home/jelmer/src/janitor/target/debug/deps/libprometheus-675a227af86df9e2.rlib --extern pyo3=/home/jelmer/src/janitor/target/debug/deps/libpyo3-777300d23104bd4b.rlib --extern pyo3_log=/home/jelmer/src/janitor/target/debug/deps/libpyo3_log-632241dddb9114dd.rlib --extern reqwest=/home/jelmer/src/janitor/target/debug/deps/libreqwest-4bc10541b85b7d73.rlib --extern serde=/home/jelmer/src/janitor/target/debug/deps/libserde-db9c03d680295e8d.rlib --extern serde_json=/home/jelmer/src/janitor/target/debug/deps/libserde_json-6b0fb4ec6d47e5f9.rlib --extern shlex=/home/jelmer/src/janitor/target/debug/deps/libshlex-897de7434cfa06c5.rlib --extern silver_platter=/home/jelmer/src/janitor/target/debug/deps/libsilver_platter-99004d4ad89f4c8e.rlib --extern tempfile=/home/jelmer/src/janitor/target/debug/deps/libtempfile-7158f71b4efb8be8.rlib --extern tokio=/home/jelmer/src/janitor/target/debug/deps/libtokio-70d97630dfe17319.rlib --extern tokio_util=/home/jelmer/src/janitor/target/debug/deps/libtokio_util-0594be3a9e82e568.rlib --extern url=/home/jelmer/src/janitor/target/debug/deps/liburl-b00546bc2a4639c9.rlib --cfg tokio_unstable -L native=/usr/lib/x86_64-linux-gnu -L native=/home/jelmer/src/janitor/target/debug/build/ring-a33f7333aabc483e/out -L native=/home/jelmer/src/janitor/target/debug/build/psm-d70ac1db1779e382/out`
warning: unused variable: `packaging_tree`
   --> worker/src/bin/dist.rs:101:10
    |
101 |     let (packaging_tree, packaging_debian_path) = if let Some(packaging) = args.packaging {
    |          ^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_packaging_tree`
    |
    = note: `#[warn(unused_variables)]` on by default

warning: unused variable: `packaging_debian_path`
   --> worker/src/bin/dist.rs:101:26
    |
101 |     let (packaging_tree, packaging_debian_path) = if let Some(packaging) = args.packaging {
    |                          ^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_packaging_debian_path`

warning: unused variable: `retcode`
   --> worker/src/bin/dist.rs:209:13
    |
209 |             retcode,
    |             ^^^^^^^ help: try ignoring the field: `retcode: _`

warning: unused variable: `e`
   --> worker/src/bin/dist.rs:233:43
    |
233 |         Err(Error::DependencyInstallError(e)) => {
    |                                           ^ help: if this is intentional, prefix it with an underscore: `_e`

warning: `janitor-worker` (bin "janitor-dist") generated 4 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.38s
Copying rust artifact from target/debug/janitor-dist to build/scripts-3.13/janitor-dist
PYTHONPATH=/home/jelmer/src/janitor/py: PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python python3 -m pytest -vv tests
============================= test session starts ==============================
platform linux -- Python 3.13.3, pytest-8.3.5, pluggy-1.5.0 -- /usr/bin/python3
cachedir: .pytest_cache
hypothesis profile 'default' -> database=DirectoryBasedExampleDatabase(PosixPath('/home/jelmer/src/janitor/.hypothesis/examples'))
rootdir: /home/jelmer/src/janitor
configfile: pyproject.toml
plugins: asyncio-0.25.1, typeguard-4.4.2, hypothesis-6.130.5, cov-5.0.0, repeat-0.9.3, aiohttp-1.1.0, anyio-4.8.0
asyncio: mode=Mode.AUTO, asyncio_default_fixture_loop_scope=function
collecting ... collected 63 items / 1 error

==================================== ERRORS ====================================
____________________ ERROR collecting tests/test_runner.py _____________________
ImportError while importing test module '/home/jelmer/src/janitor/tests/test_runner.py'.
Hint: make sure your test modules/packages have valid Python names.
Traceback:
/usr/lib/python3/dist-packages/_pytest/python.py:493: in importtestmodule
    mod = import_path(
/usr/lib/python3/dist-packages/_pytest/pathlib.py:587: in import_path
    importlib.import_module(module_name)
/usr/lib/python3.13/importlib/__init__.py:88: in import_module
    return _bootstrap._gcd_import(name[level:], package, level)
<frozen importlib._bootstrap>:1387: in _gcd_import
    ???
<frozen importlib._bootstrap>:1360: in _find_and_load
    ???
<frozen importlib._bootstrap>:1331: in _find_and_load_unlocked
    ???
<frozen importlib._bootstrap>:935: in _load_unlocked
    ???
/usr/lib/python3/dist-packages/_pytest/assertion/rewrite.py:185: in exec_module
    exec(co, module.__dict__)
tests/test_runner.py:29: in <module>
    from janitor.runner import (
py/janitor/runner.py:74: in <module>
    from silver_platter import (
E   ImportError: cannot import name 'BranchRateLimited' from 'silver_platter' (/usr/lib/python3/dist-packages/silver_platter.cpython-313-x86_64-linux-gnu.so)
=============================== warnings summary ===============================
../../../../usr/lib/python3/dist-packages/subunit/v2.py:51
  /usr/lib/python3/dist-packages/subunit/v2.py:51: DeprecationWarning: datetime.datetime.utcfromtimestamp() is deprecated and scheduled for removal in a future version. Use timezone-aware objects to represent datetimes in UTC: datetime.datetime.fromtimestamp(timestamp, datetime.UTC).
    EPOCH = datetime.datetime.utcfromtimestamp(0).replace(tzinfo=iso8601.Utc())

-- Docs: https://docs.pytest.org/en/stable/how-to/capture-warnings.html

---------- coverage: platform linux, python 3.13.3-final-0 -----------
Coverage HTML written to dir htmlcov

=========================== short test summary info ============================
ERROR tests/test_runner.py
!!!!!!!!!!!!!!!!!!!! Interrupted: 1 error during collection !!!!!!!!!!!!!!!!!!!!
========================= 1 warning, 1 error in 1.85s ==========================
make: *** [Makefile:65: test] Error 2
