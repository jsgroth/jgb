# jgb

An attempt at writing a Game Boy emulator. jgb is not cycle-accurate (mainly due to non-cycle-accurate memory timings) but it is a low-level emulator.

## Requirements

### Rust

This project requires version 1.65.0 or later of the [Rust toolchain](https://doc.rust-lang.org/book/ch01-01-installation.html) to build.
See link for installation instructions.

### SDL2

This project requires core [SDL2](https://www.libsdl.org/) headers to build.

Linux (Debian-based):
```shell
sudo apt install libsdl2-dev
```

macOS:
```shell
brew install sdl2
```

### GTK3 (Linux GUI only)

On Linux only, the GUI requires [GTK3](https://www.gtk.org/) headers to build.

Linux (Debian-based):
```shell
sudo apt install libgtk-3-dev
```

## Build & Run GUI

To build the GUI:
```shell
cargo build --release --bin jgb-gui
```

To run the GUI:
```shell
target/release/jgb-gui
```

## Build & Run CLI

To build the CLI:
```shell
cargo build --release --bin jgb-cli
```

To run a ROM file with audio enabled:
```shell
target/release/jgb-cli -a -f <gb_file>
```

To view all command-line options:
```shell
target/release/jgb-cli -h
```
