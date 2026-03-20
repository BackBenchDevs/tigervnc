# RuVNC Viewer

A modern, cross-platform VNC viewer built in Rust with an [egui](https://github.com/emilk/egui) interface, powered by the [TigerVNC](https://github.com/TigerVNC/tigervnc) protocol engine.

## Features

- Native VNC protocol support via TigerVNC's battle-tested C++ core
- Address book with persistent server management (groups, tags, search)
- Secure credential storage via OS keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- Team server synchronization from a remote JSON endpoint
- Bidirectional clipboard sharing (client <-> server)
- Multi-monitor awareness (ExtendedDesktopSize screen layout tracking)
- GPU-accelerated and software rendering backends
- Internationalization via Project Fluent (automatic locale detection)
- Cross-platform: Linux, macOS (Intel + Apple Silicon), Windows

## Building

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- C/C++ compiler (gcc, clang, or MSVC)
- pkg-config

### macOS

```tcsh
brew install jpeg-turbo gnutls nettle pixman zlib pkg-config
cd ruvnc-viewer
cargo build --release
```

### Linux (Debian/Ubuntu)

```bash
sudo apt-get install -y libjpeg-turbo8-dev libgnutls28-dev \
  libnettle-dev libpixman-1-dev zlib1g-dev libpam0g-dev \
  libgtk-3-dev libxcb-shape0-dev libxcb-xfixes0-dev
cd ruvnc-viewer
cargo build --release
```

### Windows (MSYS2 MinGW64)

```bash
pacman -S mingw-w64-x86_64-toolchain mingw-w64-x86_64-rust \
  mingw-w64-x86_64-libjpeg-turbo mingw-w64-x86_64-gnutls \
  mingw-w64-x86_64-nettle mingw-w64-x86_64-pixman \
  mingw-w64-x86_64-zlib mingw-w64-x86_64-pkg-config
cd ruvnc-viewer
cargo build --release
```

### Running

The binary is at `target/release/ruvnc-viewer` (or `ruvnc-viewer.exe` on Windows).

```tcsh
./target/release/ruvnc-viewer
```

### Running Tests

```tcsh
cargo test
```

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full project specification, including
data flow diagrams, platform specifics, performance targets, and i18n strategy.

```
ruvnc-viewer/
  src/
    main.rs              # Entry point, window setup
    bridge.rs            # cxx FFI bridge to TigerVNC C++ core
    connection.rs        # VNC connection state machine
    credentials.rs       # OS keyring integration
    address_book.rs      # Server list persistence
    sync.rs              # Team server sync
    i18n/                # Fluent-based localization (sys-locale detection)
    ui/                  # egui UI components
    renderer/            # GPU and software framebuffer rendering
    tunnel/              # (planned) SSH tunnel support
  i18n/
    en-US.ftl            # English translations (default)
  bridge/
    src/headless_conn.cc # C++ CConnection subclass for headless operation
    include/             # C++ headers
  build.rs              # Compiles TigerVNC common/ and the cxx bridge
```

## Attribution

This project is a derivative work based on [TigerVNC](https://github.com/TigerVNC/tigervnc).
It includes a new user interface built in Rust, which interfaces with the original
TigerVNC C++ core protocol engine. See [NOTICE.md](NOTICE.md) for full details.

## License

Copyright (C) 2026 BackBenchDevs

This program is free software; you can redistribute it and/or modify it under the
terms of the GNU General Public License as published by the Free Software Foundation;
either version 2 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but **WITHOUT ANY
WARRANTY**; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
PARTICULAR PURPOSE. See the [GNU General Public License](LICENSE) for more details.

The original TigerVNC code is Copyright (C) 1999-2025 by the TigerVNC Team and
its contributors. See individual source files for specific copyright holders.
