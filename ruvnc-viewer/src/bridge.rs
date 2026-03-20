// RuVNC Viewer - Modern Rust/egui VNC viewer
// Copyright (C) 2026 BackBenchDevs
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

#[cxx::bridge(namespace = "vnc_bridge")]
pub mod ffi {
    #[derive(Debug, Clone)]
    struct DamageRect {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    }

    #[derive(Debug, Clone)]
    struct CredentialResult {
        username: String,
        password: String,
        ok: bool,
    }

    #[derive(Debug, Clone)]
    struct FramebufferInfo {
        width: i32,
        height: i32,
        stride: i32,
        data_ptr: u64,
    }

    unsafe extern "C++" {
        include!("headless_conn.h");

        type VncConnection;

        fn vnc_create() -> UniquePtr<VncConnection>;
        fn vnc_connect(conn: Pin<&mut VncConnection>, host: &str, port: i32) -> bool;
        fn vnc_process_messages(conn: Pin<&mut VncConnection>) -> bool;
        fn vnc_get_socket_fd(conn: &VncConnection) -> i32;
        fn vnc_is_connected(conn: &VncConnection) -> bool;
        fn vnc_get_framebuffer_info(conn: &VncConnection) -> FramebufferInfo;
        fn vnc_send_key_press(conn: Pin<&mut VncConnection>, key_code: u32, key_sym: u32);
        fn vnc_send_key_release(conn: Pin<&mut VncConnection>, key_code: u32);
        fn vnc_send_pointer(conn: Pin<&mut VncConnection>, x: i32, y: i32, button_mask: u8);
        fn vnc_send_clipboard(conn: Pin<&mut VncConnection>, text: &str);
        fn vnc_set_preferred_encoding(conn: Pin<&mut VncConnection>, encoding: i32);
        fn vnc_set_quality_level(conn: Pin<&mut VncConnection>, level: i32);
        fn vnc_set_compress_level(conn: Pin<&mut VncConnection>, level: i32);
        fn vnc_disconnect(conn: Pin<&mut VncConnection>);
    }

    extern "Rust" {
        fn on_init_done(width: i32, height: i32);
        fn on_frame_updated(rect: &DamageRect);
        fn on_get_credentials(secure: bool, need_username: bool) -> CredentialResult;
        fn on_show_message(flags: i32, title: &str, text: &str) -> bool;
        fn on_bell();
        fn on_cursor_changed(width: i32, height: i32, hotspot_x: i32, hotspot_y: i32, data: &[u8]);
        fn on_clipboard_announce(available: bool);
        fn on_clipboard_data(text: &str);
        fn on_connection_error(message: &str);
    }
}

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

struct BridgeState {
    inner: Mutex<CallbackState>,
    credential_condvar: Condvar,
}

static BRIDGE: once_cell::sync::Lazy<Arc<BridgeState>> = once_cell::sync::Lazy::new(|| {
    Arc::new(BridgeState {
        inner: Mutex::new(CallbackState::default()),
        credential_condvar: Condvar::new(),
    })
});

#[derive(Default)]
struct CallbackState {
    init_done: Option<(i32, i32)>,
    damage_rects: Vec<ffi::DamageRect>,
    credential_request: Option<CredentialRequest>,
    credential_response: Option<ffi::CredentialResult>,
    credential_cancelled: bool,
    cursor_data: Option<CursorCallbackData>,
    clipboard_text: Option<String>,
    #[allow(dead_code)]
    clipboard_available: Option<bool>,
    error: Option<String>,
    connection_host: String,
    connection_port: u16,
    connection_username: String,
}

pub struct CredentialRequest {
    #[allow(dead_code)]
    pub secure: bool,
    pub need_username: bool,
    pub host: String,
    pub port: u16,
    pub username: String,
}

#[allow(dead_code)]
struct CursorCallbackData {
    width: i32,
    height: i32,
    hotspot_x: i32,
    hotspot_y: i32,
    data: Vec<u8>,
}

pub fn poll_init_done() -> Option<(i32, i32)> {
    BRIDGE.inner.lock().ok()?.init_done.take()
}

pub fn poll_damage_rects() -> Vec<ffi::DamageRect> {
    BRIDGE
        .inner
        .lock()
        .ok()
        .map(|mut s| std::mem::take(&mut s.damage_rects))
        .unwrap_or_default()
}

pub fn poll_error() -> Option<String> {
    BRIDGE.inner.lock().ok()?.error.take()
}

pub fn poll_cursor_data() -> Option<(i32, i32, i32, i32, Vec<u8>)> {
    BRIDGE.inner.lock().ok()?.cursor_data.take().map(|c| {
        (c.width, c.height, c.hotspot_x, c.hotspot_y, c.data)
    })
}

pub fn poll_clipboard() -> Option<String> {
    BRIDGE.inner.lock().ok()?.clipboard_text.take()
}

/// Check if the connection thread is waiting for credentials.
/// Returns Some(request) if a password dialog should be shown.
pub fn poll_credential_request() -> Option<CredentialRequest> {
    BRIDGE.inner.lock().ok()?.credential_request.take()
}

pub fn set_connection_context(host: &str, port: u16, username: &str) {
    if let Ok(mut state) = BRIDGE.inner.lock() {
        state.connection_host = host.to_string();
        state.connection_port = port;
        state.connection_username = username.to_string();
        state.credential_response = None;
        state.credential_cancelled = false;
    }
}

/// Called by the UI thread after the user enters a password.
/// Wakes up the blocked connection thread.
pub fn provide_credentials(username: String, password: String) {
    log::debug!("Credentials provided for user '{}'", username);
    if let Ok(mut state) = BRIDGE.inner.lock() {
        state.credential_response = Some(ffi::CredentialResult {
            username,
            password,
            ok: true,
        });
    }
    BRIDGE.credential_condvar.notify_all();
}

/// Called by the UI thread if the user cancels the password dialog.
pub fn cancel_credentials() {
    log::debug!("Credential dialog cancelled by user");
    if let Ok(mut state) = BRIDGE.inner.lock() {
        state.credential_cancelled = true;
    }
    BRIDGE.credential_condvar.notify_all();
}

fn on_init_done(width: i32, height: i32) {
    log::info!("VNC init done: {}x{}", width, height);
    if let Ok(mut state) = BRIDGE.inner.lock() {
        state.init_done = Some((width, height));
    }
}

fn on_frame_updated(rect: &ffi::DamageRect) {
    log::debug!(
        "Frame updated: region ({},{}) {}x{}",
        rect.x, rect.y, rect.w, rect.h
    );
    if let Ok(mut state) = BRIDGE.inner.lock() {
        state.damage_rects.push(rect.clone());
    }
}

fn on_get_credentials(secure: bool, need_username: bool) -> ffi::CredentialResult {
    log::info!("Credential request (secure={}, need_username={})", secure, need_username);

    // First check if we already have credentials (pre-provided or from keyring)
    {
        let state = BRIDGE.inner.lock().unwrap();
        let host = state.connection_host.clone();
        let port = state.connection_port;
        let username = state.connection_username.clone();

        if let Some(ref resp) = state.credential_response {
            log::debug!("Using pre-provided credentials for '{}'", resp.username);
            return resp.clone();
        }

        if !host.is_empty() {
            let keyring_key = if username.is_empty() {
                format!("{}:{}", host, port)
            } else {
                format!("{}@{}:{}", username, host, port)
            };
            log::debug!("Checking keyring key='{}' (service='ruvnc-viewer')", keyring_key);
            if let Ok(password) = crate::credentials::get_password(&host, port, &username) {
                log::info!("Found stored password in keyring for {}:{}", host, port);
                return ffi::CredentialResult {
                    username,
                    password,
                    ok: true,
                };
            }
            log::debug!("No stored password found for keyring key='{}'", keyring_key);
        }
    }

    // No credentials available -- post a request for the UI thread and block
    {
        let mut state = BRIDGE.inner.lock().unwrap();
        state.credential_cancelled = false;
        state.credential_response = None;
        state.credential_request = Some(CredentialRequest {
            secure,
            need_username,
            host: state.connection_host.clone(),
            port: state.connection_port,
            username: state.connection_username.clone(),
        });
    }

    log::info!("Waiting for user to provide credentials...");

    let timeout = Duration::from_secs(120);
    let guard = BRIDGE.inner.lock().unwrap();
    let result = BRIDGE
        .credential_condvar
        .wait_timeout_while(guard, timeout, |state| {
            state.credential_response.is_none() && !state.credential_cancelled
        });

    match result {
        Ok((state, timeout_result)) => {
            if timeout_result.timed_out() {
                log::warn!("Credential request timed out after 120s");
                return ffi::CredentialResult {
                    username: String::new(),
                    password: String::new(),
                    ok: false,
                };
            }
            if state.credential_cancelled {
                log::info!("Credential request cancelled by user");
                return ffi::CredentialResult {
                    username: String::new(),
                    password: String::new(),
                    ok: false,
                };
            }
            if let Some(ref resp) = state.credential_response {
                log::info!("Credentials received for user '{}'", resp.username);
                return resp.clone();
            }
            ffi::CredentialResult {
                username: String::new(),
                password: String::new(),
                ok: false,
            }
        }
        Err(_) => {
            log::error!("Credential condvar lock poisoned");
            ffi::CredentialResult {
                username: String::new(),
                password: String::new(),
                ok: false,
            }
        }
    }
}

fn on_show_message(flags: i32, title: &str, text: &str) -> bool {
    log::warn!("VNC message (flags={}) [{}]: {}", flags, title, text);
    true
}

fn on_bell() {
    log::debug!("Bell event received from server");
}

fn on_cursor_changed(width: i32, height: i32, hotspot_x: i32, hotspot_y: i32, data: &[u8]) {
    log::debug!(
        "Cursor changed: {}x{} hotspot=({},{}) data={} bytes",
        width, height, hotspot_x, hotspot_y, data.len()
    );
    if let Ok(mut state) = BRIDGE.inner.lock() {
        state.cursor_data = Some(CursorCallbackData {
            width,
            height,
            hotspot_x,
            hotspot_y,
            data: data.to_vec(),
        });
    }
}

fn on_clipboard_announce(available: bool) {
    log::debug!("Clipboard announce: available={}", available);
    if let Ok(mut state) = BRIDGE.inner.lock() {
        state.clipboard_available = Some(available);
    }
}

fn on_clipboard_data(text: &str) {
    log::debug!("Clipboard data received: {} bytes", text.len());
    if let Ok(mut state) = BRIDGE.inner.lock() {
        state.clipboard_text = Some(text.to_string());
    }
}

fn on_connection_error(message: &str) {
    log::error!("VNC connection error: {}", message);
    if let Ok(mut state) = BRIDGE.inner.lock() {
        state.error = Some(message.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damage_rect_clone() {
        let rect = ffi::DamageRect { x: 10, y: 20, w: 100, h: 200 };
        let cloned = rect.clone();
        assert_eq!(cloned.x, 10);
        assert_eq!(cloned.y, 20);
        assert_eq!(cloned.w, 100);
        assert_eq!(cloned.h, 200);
    }

    #[test]
    fn test_damage_rect_debug() {
        let rect = ffi::DamageRect { x: 0, y: 0, w: 1920, h: 1080 };
        let dbg = format!("{:?}", rect);
        assert!(dbg.contains("1920"));
        assert!(dbg.contains("1080"));
    }

    #[test]
    fn test_credential_result_clone() {
        let cr = ffi::CredentialResult {
            username: "user".to_string(),
            password: "pass".to_string(),
            ok: true,
        };
        let cloned = cr.clone();
        assert_eq!(cloned.username, "user");
        assert_eq!(cloned.password, "pass");
        assert!(cloned.ok);
    }

    #[test]
    fn test_credential_result_empty() {
        let cr = ffi::CredentialResult {
            username: String::new(),
            password: String::new(),
            ok: false,
        };
        assert!(!cr.ok);
        assert!(cr.username.is_empty());
    }

    #[test]
    fn test_framebuffer_info_clone() {
        let info = ffi::FramebufferInfo {
            width: 1920,
            height: 1080,
            stride: 1920,
            data_ptr: 0xDEADBEEF,
        };
        let cloned = info.clone();
        assert_eq!(cloned.width, 1920);
        assert_eq!(cloned.height, 1080);
        assert_eq!(cloned.data_ptr, 0xDEADBEEF);
    }

    #[test]
    fn test_framebuffer_info_zero() {
        let info = ffi::FramebufferInfo {
            width: 0,
            height: 0,
            stride: 0,
            data_ptr: 0,
        };
        assert_eq!(info.width, 0);
        assert_eq!(info.data_ptr, 0);
    }

    #[test]
    fn test_provide_credentials_stores_response() {
        provide_credentials("testuser".to_string(), "testpass".to_string());
        let state = BRIDGE.inner.lock().unwrap();
        let resp = state.credential_response.as_ref().unwrap();
        assert_eq!(resp.username, "testuser");
        assert!(resp.ok);
    }

    #[test]
    fn test_on_connection_error_stores_message() {
        on_connection_error("test error message");
        let state = BRIDGE.inner.lock().unwrap();
        assert!(state.error.as_ref().unwrap().contains("test error"));
    }

    #[test]
    fn test_on_bell_does_not_panic() {
        on_bell();
    }

    #[test]
    fn test_on_show_message_returns_true() {
        assert!(on_show_message(0, "Title", "Text"));
    }

    #[test]
    fn test_on_clipboard_data_stores() {
        on_clipboard_data("hello clipboard");
        let state = BRIDGE.inner.lock().unwrap();
        assert_eq!(state.clipboard_text.as_ref().unwrap(), "hello clipboard");
    }

    #[test]
    fn test_on_clipboard_announce_stores() {
        on_clipboard_announce(true);
        let state = BRIDGE.inner.lock().unwrap();
        assert_eq!(state.clipboard_available, Some(true));
    }

    #[test]
    fn test_on_init_done_stores() {
        on_init_done(1920, 1080);
        let state = BRIDGE.inner.lock().unwrap();
        assert_eq!(state.init_done, Some((1920, 1080)));
    }

    #[test]
    fn test_on_frame_updated_accumulates() {
        {
            let mut state = BRIDGE.inner.lock().unwrap();
            state.damage_rects.clear();
        }
        on_frame_updated(&ffi::DamageRect { x: 0, y: 0, w: 100, h: 100 });
        on_frame_updated(&ffi::DamageRect { x: 50, y: 50, w: 200, h: 200 });
        let state = BRIDGE.inner.lock().unwrap();
        assert!(state.damage_rects.len() >= 2);
    }

    #[test]
    fn test_on_cursor_changed_stores() {
        let data = vec![255u8; 16];
        on_cursor_changed(2, 2, 1, 1, &data);
        let state = BRIDGE.inner.lock().unwrap();
        let cursor = state.cursor_data.as_ref().unwrap();
        assert_eq!(cursor.width, 2);
        assert_eq!(cursor.height, 2);
        assert_eq!(cursor.hotspot_x, 1);
        assert_eq!(cursor.data.len(), 16);
    }

    #[test]
    fn test_cancel_credentials_wakes_waiter() {
        cancel_credentials();
        let state = BRIDGE.inner.lock().unwrap();
        assert!(state.credential_cancelled);
    }

    #[test]
    fn test_provide_then_poll_clears() {
        provide_credentials("u".to_string(), "p".to_string());
        {
            let state = BRIDGE.inner.lock().unwrap();
            assert!(state.credential_response.is_some());
        }
    }
}
