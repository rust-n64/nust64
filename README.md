[![License: BSD 2-Clause](https://img.shields.io/badge/License-BSD%202--Clause-blue?style=flat-square)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/nust64?style=flat-square)](https://crates.io/crates/nust64)
[![Documentation](https://img.shields.io/docsrs/nust64?style=flat-square)](https://docs.rs/nust64)

### Description
`nust64` is a tool for building rust projects into n64 roms. It's usable as both a runnable binary and as a crate for developers who want additional build functionality.

While still a young crate, nust64 effectively replaces the original cargo-n64.

### Usage
For using nust64 as a crate, refer to the [docs](https://docs.rs/nust64).

Otherwise, you can install nust64 as a runnable program using `cargo install nust64`. If you wish to install from source, download the repo and run `cargo install --path .` Once installed, run `nust64 --help` for additional details.

### Nightly Rust
Unfortunately, the `-Z=build-std=core,alloc` argument needed to build the target project, still requires a nightly rust toolchain. If/when that becomes a stable feature, this crate should no longer require nightly rust.

### Acknowledgements
Thanks to the first build tool, `cargo-n64`, written by [parasyte](https://github.com/rust-console/cargo-n64). This project reuses their linker target files, and reimplements the necessary build arguments and ELF section parsing that makes this process work. Without their work, Rust on the n64 likely wouldn't be a thing yet.
