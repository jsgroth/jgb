# jgb

An attempt at writing a Game Boy emulator. jgb is not cycle-accurate (mainly due to non-cycle-accurate memory timings) but it is a functional low-level emulator with no game-specific logic or game-specific hacks. It should be able to run the majority of licensed Game Boy and Game Boy Color games.

## Requirements

### Rust

This project requires version 1.65.0 or later of the [Rust toolchain](https://doc.rust-lang.org/book/ch01-01-installation.html) to build.
See link for installation instructions.

### SDL2

This project requires [SDL2](https://www.libsdl.org/) core headers and TTF headers to build.

Linux (Debian-based):
```shell
sudo apt install libsdl2-dev libsdl2-ttf-dev
```

macOS:
```shell
brew install sdl2 sdl2-ttf
```

Windows:
* https://github.com/libsdl-org/SDL/releases/
* https://github.com/libsdl-org/SDL_ttf/releases/

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

By default the GUI app will read and save its config using the file `jgb-config.toml` in the current working directory. To override this and use a custom path, use the `--config` command-line arg:
```shell
target/release/jgb-gui --config /path/to/my/config.toml
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
