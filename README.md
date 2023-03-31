# jgb

An attempt at writing a Game Boy emulator. jgb is not cycle-accurate but does use low-level emulation.

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
target/release/jgb-cli -a -f <gb_file>
```

To view all command-line options:
```shell
target/release/jgb-cli -h
```
