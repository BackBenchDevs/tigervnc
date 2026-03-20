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

use crate::bridge::ffi;
use log::{debug, error, info, warn};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum InputCommand {
    KeyPress(u32, u32),
    KeyRelease(u32),
    Pointer(i32, i32, u8),
    Clipboard(String),
    SetEncoding(i32),
    SetQuality(i32),
    SetCompress(i32),
    Disconnect,
}

pub(crate) struct ConnectionInner {
    pub(crate) state: ConnectionState,
    pub(crate) framebuffer_width: u32,
    pub(crate) framebuffer_height: u32,
    pub(crate) framebuffer_stride: u32,
    pub(crate) framebuffer_data: Vec<u8>,
    pub(crate) framebuffer_generation: u64,
    pub(crate) should_disconnect: bool,
    pub(crate) input_queue: Vec<InputCommand>,
}

pub struct ConnectionManager {
    pub(crate) inner: Arc<Mutex<ConnectionInner>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ConnectionInner {
                state: ConnectionState::Disconnected,
                framebuffer_width: 0,
                framebuffer_height: 0,
                framebuffer_stride: 0,
                framebuffer_data: Vec::new(),
                framebuffer_generation: 0,
                should_disconnect: false,
                input_queue: Vec::new(),
            })),
        }
    }

    pub fn connect(&mut self, host: &str, port: u16, username: &str) {
        info!(
            "Initiating VNC connection to {}:{} (user='{}')",
            host, port, if username.is_empty() { "<none>" } else { username }
        );

        crate::bridge::set_connection_context(host, port, username);

        let host = host.to_string();
        let port = port as i32;
        let inner = self.inner.clone();

        {
            let mut guard = inner.lock().unwrap();
            guard.state = ConnectionState::Connecting;
            guard.should_disconnect = false;
            guard.input_queue.clear();
        }

        thread::spawn(move || {
            debug!("Connection thread started for {}:{}", host, port);
            let mut conn = ffi::vnc_create();
            debug!("VncConnection object created, attempting TCP connect to {}:{}", host, port);

            if !ffi::vnc_connect(conn.pin_mut(), &host, port) {
                error!("VNC connection FAILED to {}:{} -- check that a VNC server is running at that address", host, port);
                let mut guard = inner.lock().unwrap();
                guard.state = ConnectionState::Error(
                    format!("Connection failed to {}:{}", host, port),
                );
                return;
            }

            info!("VNC connection established to {}:{}", host, port);

            {
                let mut guard = inner.lock().unwrap();
                guard.state = ConnectionState::Connected;
            }

            let mut loop_count: u64 = 0;
            loop {
                loop_count += 1;
                {
                    let mut guard = inner.lock().unwrap();
                    if guard.should_disconnect {
                        info!("Disconnect requested for {}:{}", host, port);
                        break;
                    }

                    let commands: Vec<InputCommand> = guard.input_queue.drain(..).collect();
                    drop(guard);

                    if !commands.is_empty() {
                        debug!("Processing {} queued input commands", commands.len());
                    }

                    // Coalesce consecutive pointer-move events (same button mask)
                    // to avoid flooding the socket -- only send the latest position.
                    let mut last_pointer: Option<(i32, i32, u8)> = None;
                    let mut coalesced: Vec<InputCommand> = Vec::with_capacity(commands.len());
                    for cmd in commands {
                        match cmd {
                            InputCommand::Pointer(x, y, mask) => {
                                if let Some((_, _, prev_mask)) = last_pointer {
                                    if prev_mask == mask {
                                        last_pointer = Some((x, y, mask));
                                        continue;
                                    }
                                    coalesced.push(InputCommand::Pointer(
                                        last_pointer.unwrap().0,
                                        last_pointer.unwrap().1,
                                        prev_mask,
                                    ));
                                }
                                last_pointer = Some((x, y, mask));
                            }
                            other => {
                                if let Some((px, py, pm)) = last_pointer.take() {
                                    coalesced.push(InputCommand::Pointer(px, py, pm));
                                }
                                coalesced.push(other);
                            }
                        }
                    }
                    if let Some((px, py, pm)) = last_pointer.take() {
                        coalesced.push(InputCommand::Pointer(px, py, pm));
                    }

                    for cmd in coalesced {
                        match cmd {
                            InputCommand::KeyPress(code, sym) => {
                                debug!("Sending key press: code=0x{:X} sym=0x{:X}", code, sym);
                                ffi::vnc_send_key_press(conn.pin_mut(), code, sym);
                            }
                            InputCommand::KeyRelease(code) => {
                                debug!("Sending key release: code=0x{:X}", code);
                                ffi::vnc_send_key_release(conn.pin_mut(), code);
                            }
                            InputCommand::Pointer(x, y, mask) => {
                                debug!("Sending pointer: ({}, {}) buttons=0x{:02X}", x, y, mask);
                                ffi::vnc_send_pointer(conn.pin_mut(), x, y, mask);
                            }
                            InputCommand::Clipboard(text) => {
                                debug!("Sending clipboard: {} bytes", text.len());
                                ffi::vnc_send_clipboard(conn.pin_mut(), &text);
                            }
                            InputCommand::SetEncoding(enc) => {
                                debug!("Setting preferred encoding: {}", enc);
                                ffi::vnc_set_preferred_encoding(conn.pin_mut(), enc);
                            }
                            InputCommand::SetQuality(q) => {
                                debug!("Setting quality level: {}", q);
                                ffi::vnc_set_quality_level(conn.pin_mut(), q);
                            }
                            InputCommand::SetCompress(c) => {
                                debug!("Setting compress level: {}", c);
                                ffi::vnc_set_compress_level(conn.pin_mut(), c);
                            }
                            InputCommand::Disconnect => {
                                info!("Disconnect command received for {}:{}", host, port);
                                let mut guard = inner.lock().unwrap();
                                guard.should_disconnect = true;
                                break;
                            }
                        }
                    }
                }

                let fd = ffi::vnc_get_socket_fd(&conn);
                if fd < 0 {
                    warn!("Socket fd is invalid (<0) for {}:{}, closing connection", host, port);
                    break;
                }

                if !ffi::vnc_process_messages(conn.pin_mut()) {
                    if !ffi::vnc_is_connected(&conn) {
                        warn!("Connection to {}:{} lost (server closed or network error)", host, port);
                        break;
                    }
                    thread::sleep(std::time::Duration::from_millis(1));
                    continue;
                }

                let fb_info = ffi::vnc_get_framebuffer_info(&conn);
                if fb_info.width > 0 && fb_info.height > 0 && fb_info.data_ptr != 0 {
                    let w = fb_info.width as u32;
                    let h = fb_info.height as u32;
                    let stride = fb_info.stride as u32;
                    let byte_len = (h as usize) * (stride as usize) * 4;

                    // Copy pixel data on the connection thread where the C++ pointer is valid
                    let pixel_copy = unsafe {
                        std::slice::from_raw_parts(fb_info.data_ptr as *const u8, byte_len)
                            .to_vec()
                    };

                    let mut guard = inner.lock().unwrap();
                    if guard.framebuffer_width != w || guard.framebuffer_height != h {
                        info!(
                            "Framebuffer size: {}x{} (was {}x{})",
                            w, h, guard.framebuffer_width, guard.framebuffer_height
                        );
                    }
                    guard.framebuffer_width = w;
                    guard.framebuffer_height = h;
                    guard.framebuffer_stride = stride;
                    guard.framebuffer_data = pixel_copy;
                    guard.framebuffer_generation += 1;
                }

                if loop_count == 1 {
                    debug!("Message loop running for {}:{} (fd={})", host, port, fd);
                }
            }

            debug!("Sending disconnect to {}:{}", host, port);
            ffi::vnc_disconnect(conn.pin_mut());

            let mut guard = inner.lock().unwrap();
            if guard.state == ConnectionState::Connected {
                guard.state = ConnectionState::Disconnected;
            }
            info!("VNC connection to {}:{} closed", host, port);
        });
    }

    pub fn disconnect(&mut self) {
        info!("Disconnect requested by user");
        let mut guard = self.inner.lock().unwrap();
        guard.should_disconnect = true;
    }

    pub fn send_key_press(&self, key_code: u32, key_sym: u32) {
        let mut guard = self.inner.lock().unwrap();
        guard.input_queue.push(InputCommand::KeyPress(key_code, key_sym));
    }

    pub fn send_key_release(&self, key_code: u32) {
        let mut guard = self.inner.lock().unwrap();
        guard.input_queue.push(InputCommand::KeyRelease(key_code));
    }

    pub fn send_pointer(&self, x: i32, y: i32, button_mask: u8) {
        let mut guard = self.inner.lock().unwrap();
        guard.input_queue.push(InputCommand::Pointer(x, y, button_mask));
    }

    pub fn send_clipboard(&self, text: &str) {
        let mut guard = self.inner.lock().unwrap();
        guard
            .input_queue
            .push(InputCommand::Clipboard(text.to_string()));
    }

    #[allow(dead_code)]
    pub fn set_encoding(&self, encoding: i32) {
        let mut guard = self.inner.lock().unwrap();
        guard.input_queue.push(InputCommand::SetEncoding(encoding));
    }

    #[allow(dead_code)]
    pub fn set_quality(&self, level: i32) {
        let mut guard = self.inner.lock().unwrap();
        guard.input_queue.push(InputCommand::SetQuality(level));
    }

    #[allow(dead_code)]
    pub fn set_compress(&self, level: i32) {
        let mut guard = self.inner.lock().unwrap();
        guard.input_queue.push(InputCommand::SetCompress(level));
    }

    pub fn is_connected(&self) -> bool {
        let guard = self.inner.lock().unwrap();
        guard.state == ConnectionState::Connected
    }

    pub fn has_error(&self) -> bool {
        let guard = self.inner.lock().unwrap();
        matches!(guard.state, ConnectionState::Error(_))
    }

    #[allow(dead_code)]
    pub fn state(&self) -> ConnectionState {
        let guard = self.inner.lock().unwrap();
        guard.state.clone()
    }

    #[allow(dead_code)]
    pub fn framebuffer_size(&self) -> (u32, u32) {
        let guard = self.inner.lock().unwrap();
        (guard.framebuffer_width, guard.framebuffer_height)
    }

    pub fn framebuffer_snapshot(&self) -> (u32, u32, u32, Vec<u8>, u64) {
        let guard = self.inner.lock().unwrap();
        (
            guard.framebuffer_width,
            guard.framebuffer_height,
            guard.framebuffer_stride,
            guard.framebuffer_data.clone(),
            guard.framebuffer_generation,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let mgr = ConnectionManager::new();
        assert!(!mgr.is_connected());
        assert!(!mgr.has_error());
        assert_eq!(mgr.state(), ConnectionState::Disconnected);
        assert_eq!(mgr.framebuffer_size(), (0, 0));
    }

    #[test]
    fn test_disconnect_when_not_connected() {
        let mut mgr = ConnectionManager::new();
        mgr.disconnect();
        assert!(!mgr.is_connected());
        assert_eq!(mgr.state(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_send_key_press_queues() {
        let mgr = ConnectionManager::new();
        mgr.send_key_press(0x41, 0x41);
        let guard = mgr.inner.lock().unwrap();
        assert_eq!(guard.input_queue.len(), 1);
        assert!(matches!(guard.input_queue[0], InputCommand::KeyPress(0x41, 0x41)));
    }

    #[test]
    fn test_send_key_release_queues() {
        let mgr = ConnectionManager::new();
        mgr.send_key_release(0x41);
        let guard = mgr.inner.lock().unwrap();
        assert_eq!(guard.input_queue.len(), 1);
        assert!(matches!(guard.input_queue[0], InputCommand::KeyRelease(0x41)));
    }

    #[test]
    fn test_send_pointer_queues() {
        let mgr = ConnectionManager::new();
        mgr.send_pointer(100, 200, 1);
        let guard = mgr.inner.lock().unwrap();
        assert_eq!(guard.input_queue.len(), 1);
        assert!(matches!(guard.input_queue[0], InputCommand::Pointer(100, 200, 1)));
    }

    #[test]
    fn test_send_clipboard_queues() {
        let mgr = ConnectionManager::new();
        mgr.send_clipboard("hello world");
        let guard = mgr.inner.lock().unwrap();
        assert_eq!(guard.input_queue.len(), 1);
        if let InputCommand::Clipboard(ref text) = guard.input_queue[0] {
            assert_eq!(text, "hello world");
        } else {
            panic!("expected Clipboard command");
        }
    }

    #[test]
    fn test_set_encoding_queues() {
        let mgr = ConnectionManager::new();
        mgr.set_encoding(7);
        let guard = mgr.inner.lock().unwrap();
        assert!(matches!(guard.input_queue[0], InputCommand::SetEncoding(7)));
    }

    #[test]
    fn test_set_quality_queues() {
        let mgr = ConnectionManager::new();
        mgr.set_quality(8);
        let guard = mgr.inner.lock().unwrap();
        assert!(matches!(guard.input_queue[0], InputCommand::SetQuality(8)));
    }

    #[test]
    fn test_set_compress_queues() {
        let mgr = ConnectionManager::new();
        mgr.set_compress(2);
        let guard = mgr.inner.lock().unwrap();
        assert!(matches!(guard.input_queue[0], InputCommand::SetCompress(2)));
    }

    #[test]
    fn test_multiple_commands_queue_in_order() {
        let mgr = ConnectionManager::new();
        mgr.send_key_press(0x41, 0x41);
        mgr.send_key_release(0x41);
        mgr.send_pointer(50, 60, 0);
        mgr.send_clipboard("test");
        mgr.set_encoding(7);
        mgr.set_quality(8);
        mgr.set_compress(2);

        let guard = mgr.inner.lock().unwrap();
        assert_eq!(guard.input_queue.len(), 7);
        assert!(matches!(guard.input_queue[0], InputCommand::KeyPress(..)));
        assert!(matches!(guard.input_queue[1], InputCommand::KeyRelease(..)));
        assert!(matches!(guard.input_queue[2], InputCommand::Pointer(..)));
        assert!(matches!(guard.input_queue[3], InputCommand::Clipboard(..)));
        assert!(matches!(guard.input_queue[4], InputCommand::SetEncoding(..)));
        assert!(matches!(guard.input_queue[5], InputCommand::SetQuality(..)));
        assert!(matches!(guard.input_queue[6], InputCommand::SetCompress(..)));
    }

    #[test]
    fn test_connection_state_clone_eq() {
        let s1 = ConnectionState::Disconnected;
        let s2 = s1.clone();
        assert_eq!(s1, s2);

        let s3 = ConnectionState::Error("fail".to_string());
        let s4 = s3.clone();
        assert_eq!(s3, s4);

        assert_ne!(s1, s3);
    }

    #[test]
    fn test_connection_state_debug() {
        let s = ConnectionState::Connecting;
        assert_eq!(format!("{:?}", s), "Connecting");
    }

    #[test]
    fn test_manager_is_thread_safe() {
        let mgr = ConnectionManager::new();
        let inner = mgr.inner.clone();
        let handle = std::thread::spawn(move || {
            let guard = inner.lock().unwrap();
            guard.state.clone()
        });
        let state = handle.join().unwrap();
        assert_eq!(state, ConnectionState::Disconnected);
    }
}
