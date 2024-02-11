# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)

## [0.3.0] - 2024-02-10
- Added: Support for libdragon's new open-source IPL3 (compat, dev, and release builds)
- Changed: All paths must now be UTF8.
- Changed: Custom IPL3s are no longer restricted to 4032 bytes.
- Changed: Fixed `Header::new()` parsing everything after the rom name incorrectly.

## [0.2.0] - 2023-01-05
- Added: New optional runner arguments (pre-/post-build commands, ELF section specifier, and append files to ROM)
- Removed: `Elf::build()` Builds can and should be handled by using a combination of `rust-toolchain.toml` and `.cargo/config.toml` files instead.
- Removed: Unused dependencies.
- Changed: `Elf::with_file()` to `Elf::new()`
- Changed: Checking if an ELF is executable is now a method instead of a field.
- Changed: The nightly toolchain is no longer required.
- Changed: Project is now licensed under MIT (recommended for rust crates) instead of BSD

## [0.1.7] - 2022-12-23
- Fixed: Filenames larger than 20 bytes panic

## [0.1.6] - 2022-10-29
- Fixed: ROM Header parser advancing incorrectly