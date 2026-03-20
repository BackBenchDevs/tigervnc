# RuVNC Viewer - Feature Guide

## Address Book

Persistent server management with groups, tags, and full-text search.

### Adding Servers

- **Menu:** File > New Connection
- **Quick Connect:** Type `host` or `host:port` in the quick-connect bar and press Enter
- VNC display numbers (`:1`, `:2`, etc.) are automatically resolved to TCP ports (5901, 5902, etc.)

### Server Entry Fields

| Field    | Description                                      |
|----------|--------------------------------------------------|
| Name     | Display label (e.g., "Production DB")            |
| Host     | Hostname or IP address                           |
| Port     | TCP port (default 5900) or display number (0-99) |
| Group    | Organizational group (e.g., "Infrastructure")    |
| Tags     | Comma-separated labels for filtering             |
| Username | Default username for authentication              |
| Notes    | Free-text notes                                  |

### Search

The search bar filters servers across all fields: name, host, group, tags, and notes.
The search is case-insensitive.

### Groups

Servers are organized into groups. The sidebar shows all groups; clicking one filters
the server list. The "Default" group is created automatically.

### Storage

The address book is stored as JSON at:
- **macOS:** `~/Library/Application Support/ruvnc-viewer/servers.json`
- **Linux:** `~/.config/ruvnc-viewer/servers.json`
- **Windows:** `%APPDATA%\ruvnc-viewer\servers.json`

---

## Credential Storage

Passwords can be saved to the OS keyring for automatic login on subsequent connections.

### How It Works

1. When connecting to a server that requires authentication, a password dialog appears
2. Check "Save password in keyring" before clicking OK
3. On the next connection to the same server, the saved password is used automatically

### Keyring Backends

| Platform | Backend                                  |
|----------|------------------------------------------|
| macOS    | Keychain (via Security framework)        |
| Windows  | Credential Manager                       |
| Linux    | Secret Service D-Bus API (GNOME Keyring / KDE Wallet) |

Credentials are stored per host:port:username combination under the service name `ruvnc-viewer`.

---

## Team Server Synchronization

Synchronize a shared server list from a remote JSON endpoint, enabling teams to maintain
a central list of VNC servers.

### Setup

1. Go to **Team > Configure Sync URL**
2. Enter the URL of a JSON file containing server definitions
3. Click **Save & Sync Now**

### JSON Format

The sync endpoint should return either a bare JSON array or a wrapper object:

```json
[
  {
    "name": "Production Server",
    "host": "10.0.0.1",
    "port": 5900,
    "group": "Production",
    "tags": ["linux", "web"]
  }
]
```

Or with a wrapper:

```json
{
  "servers": [
    { "name": "Staging", "host": "10.0.0.2", "port": 5900 }
  ]
}
```

### Behavior

- Team servers are marked with a "Team" badge in the address book
- Team servers are read-only (cannot be edited locally)
- Local servers are never overwritten by team sync
- Each sync replaces the previous set of team servers
- Sync interval defaults to 5 minutes

### Supported URL Types

- HTTPS URLs
- GitHub Gist raw URLs
- S3 presigned URLs
- Any URL returning valid JSON

---

## Bidirectional Clipboard Sharing

Text clipboard is shared between the local machine and the remote VNC server.

### Client to Server

When you paste text in the viewer (Ctrl+V / Cmd+V), the text is sent to the remote
server's clipboard via the VNC protocol.

### Server to Client

When the remote server's clipboard changes, the text is automatically copied to your
local system clipboard. This happens transparently in the background.

---

## Multi-Monitor Awareness

RuVNC Viewer tracks the server's screen layout via the VNC ExtendedDesktopSize
pseudo-encoding.

### Current Capabilities

- `ScreenLayout` tracks all screens reported by the server
- Each `Screen` has: id, position (x, y), and dimensions (width, height)
- `bounding_box()` computes the total desktop area across all monitors
- `is_multi_monitor()` detects multi-screen setups

### Planned Enhancements

- Per-screen viewport selection (zoom into a single monitor)
- Monitor selector toolbar
- Desktop minimap showing monitor arrangement
- Client-side resize requests (`SetDesktopSize`)

---

## Rendering

### GPU Renderer (Default)

Uses egui's built-in GPU pipeline (OpenGL via glow) for hardware-accelerated
framebuffer display. Textures are uploaded per damage region for efficiency.

### Software Renderer (Fallback)

CPU-based rendering using tiny-skia. Used when GPU acceleration is unavailable.
The renderer composites the VNC framebuffer and cursor overlay into an egui-compatible
image.

### Zoom Modes

| Mode   | Description                                    |
|--------|------------------------------------------------|
| Fit    | Scale to fit the window, preserving aspect ratio |
| 1:1    | Native pixel resolution (1 VNC pixel = 1 screen pixel) |
| 50-200% | Fixed zoom percentages                        |

The zoom toolbar appears at the bottom of each session window.

---

## Multi-Session Support

Multiple VNC connections can be open simultaneously, each in its own window.

- Each session gets a dedicated viewport with its own menu bar
- Duplicate connection attempts focus the existing session window instead of opening a new one
- Sessions can be disconnected individually via Connection > Disconnect
- Closing a session window disconnects automatically

---

## Internationalization (i18n)

The UI supports localization via Project Fluent.

### How It Works

- The system locale is detected automatically at startup via `sys-locale`
- Translation strings are loaded from `.ftl` files in the `i18n/` directory
- English (en-US) is the default fallback

### Adding a New Language

1. Create a new file `i18n/<locale>.ftl` (e.g., `i18n/ja-JP.ftl`)
2. Copy all message keys from `i18n/en-US.ftl`
3. Translate the values
4. The locale will be picked up automatically if it matches the system locale

### API

```rust
use crate::i18n::{t, t_args, FluentArgValue};

// Simple lookup
let title = t("app-title");

// With arguments
let count: &dyn FluentArgValue = &5_i64;
let status = t_args("status-servers", &[("count", count)]);
```

---

## CI/CD Pipeline

Automated builds run on every push and pull request via GitHub Actions.

### Build Matrix

| Platform          | Runner       | Architecture |
|-------------------|--------------|--------------|
| Linux             | ubuntu-latest | x86_64      |
| macOS (Intel)     | macos-13     | x86_64       |
| macOS (Apple Silicon) | macos-14 | ARM64        |
| Windows           | windows-latest | x64 (MSYS2 MinGW) |

### Workflow Steps

1. **Lint:** `cargo fmt --check` + `cargo clippy -D warnings`
2. **Build:** `cargo build --release` on all 4 platforms
3. **Test:** `cargo test --release` on all 4 platforms
4. **Artifacts:** Binary uploaded per platform
5. **Release:** On git tag (`v*`), binaries are packaged with `LICENSE` and `NOTICE.md`
   and published as a GitHub Release

### Classic Viewer Guard

The CI also builds the original TigerVNC FLTK viewer on Linux as a regression check,
ensuring changes to `common/` don't break the upstream codebase.
