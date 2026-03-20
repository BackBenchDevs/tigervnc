# RuVNC Viewer - English (US) translations
# This is the default/fallback locale.

# --- Application ---
app-title = RuVNC Viewer
app-version = Version { $version }

# --- Menu Bar ---
menu-file = File
menu-file-new-connection = New Connection...
menu-file-import = Import Servers...
menu-file-export = Export Servers...
menu-file-quit = Quit

menu-team = Team
menu-team-sync-now = Sync Now
menu-team-configure = Configure Sync URL...

menu-help = Help
menu-help-about = About RuVNC Viewer

# --- About Dialog ---
about-title = About RuVNC Viewer
about-heading = RuVNC Viewer
about-description = Modern Rust/egui VNC viewer.
about-based-on = Based on TigerVNC v1.16.80 protocol engine.
about-copyright = Copyright (C) 2026 BackBenchDevs
about-license = Licensed under GPL-2.0-or-later
about-no-warranty = This program comes with ABSOLUTELY NO WARRANTY.
about-source-code = Source Code
about-close = Close

# --- Status Bar ---
status-servers =
    { $count ->
        [one] { $count } server
       *[other] { $count } servers
    }
status-active-sessions =
    { $count ->
        [one] { $count } active session
       *[other] { $count } active sessions
    }
status-ready = Ready

# --- Connection ---
connection-connecting = Connecting...
connection-cancel = Cancel
connection-disconnect = Disconnect
connection-enter-password = Enter password to connect...
connection-error = Connection failed to { $host }:{ $port }

# --- Address Book ---
addressbook-add = Add Server
addressbook-connect = Connect
addressbook-edit = Edit
addressbook-delete = Delete
addressbook-search-placeholder = Search servers...
addressbook-quick-connect = Quick Connect
addressbook-no-servers = No servers configured. Click "Add Server" to get started.

# --- Password Dialog ---
password-title = Authentication Required
password-host = Host: { $host }:{ $port }
password-username = Username
password-password = Password
password-save = Save password in keyring
password-ok = OK
password-cancel = Cancel

# --- Sync Dialog ---
sync-title = Team Sync Configuration
sync-url-label = Enter the URL of your team's server list (JSON):
sync-url-hint = Supports: HTTPS URL, GitHub Gist raw URL, S3 presigned URL
sync-syncing = Syncing...
sync-error = Last error: { $error }
sync-save = Save
sync-save-and-sync = Save & Sync Now
sync-cancel = Cancel

# --- Session Window ---
session-title = { $name } - RuVNC Viewer
