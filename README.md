# jgb

An attempt at writing a Game Boy emulator. jgb is not cycle-accurate (mainly due to non-cycle-accurate memory timings) but it is a low-level emulator.

## Requirements

This project requires at least version 1.65.0 of the [Rust toolchain](https://doc.rust-lang.org/book/ch01-01-installation.html) to build.

Additionally, this project requires core [SDL2](https://www.libsdl.org/) headers.

Linux (Debian-based):
```shell
sudo apt install libsdl2-dev
```

macOS:
```shell
brew install sdl2
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
