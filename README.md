[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/nust64?style=flat-square)](https://crates.io/crates/nust64)
[![Documentation](https://img.shields.io/docsrs/nust64?style=flat-square)](https://docs.rs/nust64)

### Description
`nust64` is a tool for building rust projects into n64 roms. It's intended as a [Cargo runner](https://doc.rust-lang.org/cargo/reference/config.html#targettriplerunner), but may also be used as a library.

### Usage
For using nust64 as a crate, refer to the [docs](https://docs.rs/nust64).

Otherwise, you can install nust64 as a runnable program using `cargo install nust64`. If you wish to install from source, download the repo and run `cargo install --path .` Once installed, run `nust64 --help` for additional details.

#### Cargo Runner
First you should install nust64 as described above. Next, if your project doesn't already have it,
create the file `.cargo/config.toml`, and include this section:
```Toml
[target.mips-nintendo64-none]
runner = [
    "nust64",
    "--elf"
]
```
When you `cargo run` or `cargo run --release`, Cargo will append the runner command with the path to the compiled ELF file for your project, and execute the command.

If you are using a target with a different name, then replace `mips-nintendo64-none` with the desired target [triple](https://doc.rust-lang.org/cargo/reference/config.html#targettriplerunner) or [cfg expression](https://doc.rust-lang.org/cargo/reference/config.html#targetcfgrunner).

If you want to use any arguments that have spaces in it, you must format it like below.. Say you wanted to run the Ares emulator after building the rom:
```Toml
[target.mips-nintendo64-none]
runner = [
    "nust64",
    "--post-exec", "/path/to/ares >>ROM<<",
    "--elf"
]
```

#### IPL3 Selection
The IPL3 is a small portion of code that the N64 executes automatically during the boot process. Every ROM must contain a valid IPL3.

**To provide your own IPL3,** use the `--ipl3` argument like so:
```Toml
[target.mips-nintendo64-none]
runner = [
    "nust64",
    "--ipl3", "path/to/ipl3.bin",
    "--elf"
]
```

**To use one of libdragon's open-source IPL3s,** you must omit `--ipl3` and optionally specify which IPL3 version to use with `--libdragon <option>`. Available options: `compat`, `debug`, or `release`.

_If both `--ipl3` and `--libdragon` are omitted, `--libdragon release` will be used by default._

Here's an example of using the debug (aka the "dev") version of libdragon's IPL3:
```Toml
[target.mips-nintendo64-none]
runner = [
    "nust64",
    "--libdragon", "debug",
    "--elf"
]
```

### Acknowledgements
Thanks to the first build tool, `cargo-n64`, written by [parasyte](https://github.com/rust-console/cargo-n64). I initially relied on that project to learn the basics of what was needed to compile for the n64's architecture.