# RuVNC Viewer - Architecture & Project Specification

## 1. Project Overview

**Goal:** Provide a modern, high-performance VNC viewer using a Rust-based UI layer
on top of the established TigerVNC C++ core protocol engine.

**Target Platforms:** Windows (x86_64), macOS (Intel/Apple Silicon), Linux (x86_64/ARM64).

**License:** GNU GPL v2.0 or later (inherited from TigerVNC).

## 2. Architecture

### 2.1 Technology Stack

| Layer            | Technology                                              |
|------------------|---------------------------------------------------------|
| Core Engine      | TigerVNC C++ (`common/` libraries: RFB, encodings, TLS)|
| UI Layer         | Rust + egui/eframe                                      |
| Interoperability | `cxx` crate for safe C++/Rust FFI                       |
| Build System     | Cargo (primary) + `cc`/`cxx-build` (C++ compilation)    |
| Rendering        | GPU (wgpu via egui) + software fallback (tiny-skia)     |

### 2.2 Module Map

```
ruvnc-viewer/
  src/
    main.rs                  Entry point, eframe window setup
    bridge.rs                cxx FFI bridge definitions (Rust <-> C++)
    connection.rs            VNC connection state machine, input coalescing
    credentials.rs           OS keyring integration (macOS/Windows/Linux)
    address_book.rs          Persistent server list (JSON, groups, tags, search)
    sync.rs                  Team server sync from remote JSON endpoint
    ui/
      mod.rs                 App struct, menu bar, session management
      address_book_panel.rs  Server list UI with groups, search, quick-connect
      connection_dialog.rs   Add/edit server dialog
      password_dialog.rs     Credential prompt with save-to-keyring option
      session_view.rs        VNC framebuffer display, keyboard/pointer capture
      sync_dialog.rs         Team sync configuration
    renderer/
      mod.rs                 Renderer trait, backend selection
      types.rs               FrameBuffer, CursorData, ZoomMode types
      gpu_renderer.rs        GPU-accelerated texture upload via egui
      software_renderer.rs   CPU-based rendering fallback
    tunnel/
      mod.rs                 (Planned) SSH tunnel via ssh2-rs
    i18n/
      mod.rs                 Fluent-based localization loader
  i18n/
    en-US.ftl                English (default) translations
  bridge/
    src/headless_conn.cc     C++ CConnection subclass (FLTK-free)
    include/headless_conn.h  C++ header
  build.rs                   Compiles TigerVNC common/ and cxx bridge
```

### 2.3 Data Flow

```
User Input (keyboard/mouse)
    |
    v
egui event loop (session_view.rs)
    |
    v
ConnectionManager input queue (connection.rs)
    |  (coalesced pointer events)
    v
cxx FFI bridge (bridge.rs -> headless_conn.cc)
    |
    v
TigerVNC C++ CConnection (CMsgWriter -> socket)
    |
    v
VNC Server
    |
    v
TigerVNC C++ CConnection (CMsgReader -> decoders)
    |
    v
Framebuffer update callbacks (bridge.rs on_frame_updated)
    |
    v
FrameBuffer copy to Rust (connection.rs)
    |
    v
Renderer (gpu_renderer.rs or software_renderer.rs)
    |
    v
egui texture display (session_view.rs)
```

## 3. Functional Requirements

### 3.1 Protocol Support
- Full compliance with RFC 6143 (RFB 3.8) via TigerVNC core
- Encodings: Tight, ZRLE, Hextile, Raw, CopyRect, JPEG
- Security types: VncAuth, TLSVnc, X509Vnc, Plain, DH, MSLogonII, RSA-AES
  (via TigerVNC's GnuTLS/Nettle implementation)

### 3.2 Input Handling
- egui keyboard events mapped to X11 keysyms via bridge
- Pointer events coalesced per-frame to avoid socket flooding
- Clipboard: bidirectional text sharing (client <-> server)
  - Client-to-server: egui `Paste` events forwarded via `vnc_send_clipboard`
  - Server-to-client: bridge `poll_clipboard` writes to system clipboard via `ctx.copy_text()`

### 3.3 Multi-Monitor Awareness
- `ScreenLayout` tracks the server's ExtendedDesktopSize screen geometry
- `Screen` struct models individual monitors (id, position, dimensions)
- `bounding_box()` computes the total desktop area across all screens
- Future: per-screen viewport selection, monitor-specific zoom

### 3.4 Credential Management
- Interactive password dialog with optional "save" checkbox
- Persistent storage via OS keyring (`keyring` crate):
  - macOS: Keychain
  - Windows: Credential Manager
  - Linux: Secret Service (GNOME Keyring / KDE Wallet)

## 4. Platform Specifics

| Requirement    | Windows              | macOS                | Linux                     |
|----------------|----------------------|----------------------|---------------------------|
| Graphics API   | OpenGL (via glow)    | OpenGL (via glow)    | OpenGL / Wayland / X11    |
| C++ Deps       | MSYS2 MinGW packages | Homebrew             | apt/dnf                   |
| Packaging      | .exe + DLL bundle    | Binary               | Binary                    |
| Keyring        | Credential Manager   | Keychain             | Secret Service (D-Bus)    |

### Future Packaging Targets
- Windows: MSI or Inno Setup installer
- macOS: Notarized .dmg bundle
- Linux: .deb, .rpm, Flatpak, AppImage

## 5. Internationalization (i18n)

- **Framework:** Project Fluent (`fluent` + `fluent-bundle` crates)
- **Storage:** Translation files in `i18n/*.ftl`, loaded at runtime
- **Detection:** Automatic locale detection via `sys-locale` crate at startup
- **Fallback:** English (en-US) is the default when no matching locale is found

## 6. Compliance & Legal

- The project includes the original `LICENCE.TXT` from TigerVNC
- The Rust UI source code is public under GPL-2.0-or-later
- All Rust source files carry GPL copyright headers
- The About dialog includes copyright, license, no-warranty notice, and source link
- Binary releases include `LICENSE` and `NOTICE.md`
- See [NOTICE.md](NOTICE.md) for full attribution details

## 7. Build & CI/CD

- **CI Platform:** GitHub Actions (`.github/workflows/ruvnc-viewer.yml`)
- **Matrix:** Linux x86_64, macOS x86_64, macOS ARM64, Windows x64
- **Lint:** `rustfmt --check` + `clippy -D warnings` on every push/PR
- **Caching:** `Swatinem/rust-cache@v2` for Rust + C++ compilation artifacts
- **Release:** Automatic binary packaging on Git tag (`v*`) via `softprops/action-gh-release`
- **Regression Guard:** Classic TigerVNC FLTK viewer builds on Linux as a sanity check

## 8. Performance Targets

| Metric           | Target                                                |
|------------------|-------------------------------------------------------|
| Memory Overhead  | UI layer adds < 50 MB RAM over base C++ engine        |
| Frame Latency    | Rendering introduces < 2 ms delay vs raw TigerVNC     |
| Pointer Coalesce | Consecutive same-button pointer events merged per tick |
| Startup Time     | < 500 ms to first window on modern hardware            |
