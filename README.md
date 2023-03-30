# jgb

An attempt at writing a Game Boy emulator.

## Requirements

This project requires the [Rust toolchain](https://doc.rust-lang.org/book/ch01-01-installation.html) to build.

Additionally, this project requires [SDL2](https://www.libsdl.org/) headers.

Linux (Debian-based):
```shell
sudo apt-get install libsdl2-dev
```

## Build & Run

This emulator currently only has a command-line interface.

To build:
```shell
cargo build --release
```

To run a ROM file with audio enabled:
```shell
RUST_LOG=info target/release/jgb-cli --gb-file-path <gb_file> -a
```

To view all command-line options:
```shell
target/release/jgb-cli -h
```
