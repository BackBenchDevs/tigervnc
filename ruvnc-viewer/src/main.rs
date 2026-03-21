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

mod address_book;
mod bridge;
mod connection;
mod credentials;
pub mod i18n;
mod renderer;
mod sync;
mod tunnel;
mod ui;

use eframe::NativeOptions;
use log::{debug, info};

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("ruvnc_viewer=debug,warn"),
    )
    .init();
    info!("RuVNC Viewer v{} starting", env!("CARGO_PKG_VERSION"));

    let config_path = address_book::AddressBook::storage_path();
    info!("Config path: {}", config_path.display());
    debug!("Config file exists: {}", config_path.exists());

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("RuVNC Viewer"),
        ..Default::default()
    };

    eframe::run_native(
        "RuVNC Viewer",
        options,
        Box::new(|cc| Ok(Box::new(ui::App::new(cc)))),
    )
}

#[cfg(test)]
mod integration_tests {
    use crate::address_book::{AddressBook, ServerEntry};
    use crate::connection::{ConnectionManager, ConnectionState};
    use crate::renderer::FrameBuffer;
    use crate::sync::SyncConfig;

    // --- Address Book lifecycle ---

    #[test]
    fn test_full_address_book_crud() {
        let mut book = AddressBook::default();

        let entry = ServerEntry {
            name: "Test Server".to_string(),
            host: "192.168.1.100".to_string(),
            port: 5900,
            group: "Testing".to_string(),
            tags: vec!["linux".to_string(), "dev".to_string()],
            ..Default::default()
        };
        let id = entry.id.clone();

        // Create
        book.add(entry);
        assert_eq!(book.servers.len(), 1);
        assert!(book.groups.contains(&"Testing".to_string()));

        // Read
        let found = book.find(&id).unwrap();
        assert_eq!(found.name, "Test Server");

        // Update
        let mut updated = found.clone();
        updated.name = "Updated Server".to_string();
        book.update(updated);
        assert_eq!(book.find(&id).unwrap().name, "Updated Server");

        // Mark connected
        book.mark_connected(&id);
        assert!(book.find(&id).unwrap().last_connected.is_some());

        // Delete
        book.remove(&id);
        assert_eq!(book.servers.len(), 0);
    }

    // --- Search across fields ---

    #[test]
    fn test_search_across_all_fields() {
        let entry = ServerEntry {
            name: "Production DB".to_string(),
            host: "db.example.com".to_string(),
            port: 5900,
            group: "Infrastructure".to_string(),
            tags: vec!["postgres".to_string(), "primary".to_string()],
            notes: "Main database server, handle with care".to_string(),
            ..Default::default()
        };

        assert!(entry.matches_search("production"));
        assert!(entry.matches_search("example.com"));
        assert!(entry.matches_search("infrastructure"));
        assert!(entry.matches_search("postgres"));
        assert!(entry.matches_search("handle with care"));
        assert!(!entry.matches_search("nonexistent"));
    }

    // --- Team merge correctness ---

    #[test]
    fn test_team_merge_full_lifecycle() {
        let mut book = AddressBook::default();

        // Add local server
        let local = ServerEntry {
            id: "local-1".to_string(),
            name: "My Local Server".to_string(),
            host: "localhost".to_string(),
            is_team: false,
            ..Default::default()
        };
        book.add(local);

        // First team sync
        let team_v1 = vec![
            ServerEntry {
                id: "team-1".to_string(),
                name: "Team Prod".to_string(),
                host: "10.0.0.1".to_string(),
                is_team: true,
                ..Default::default()
            },
            ServerEntry {
                id: "team-2".to_string(),
                name: "Team Staging".to_string(),
                host: "10.0.0.2".to_string(),
                is_team: true,
                ..Default::default()
            },
        ];
        book.merge_team_servers(team_v1);
        assert_eq!(book.servers.len(), 3);

        // Second team sync replaces team entries
        let team_v2 = vec![ServerEntry {
            id: "team-3".to_string(),
            name: "Team New".to_string(),
            host: "10.0.0.3".to_string(),
            is_team: true,
            ..Default::default()
        }];
        book.merge_team_servers(team_v2);

        assert_eq!(book.servers.len(), 2);
        assert!(book.servers.iter().any(|s| s.id == "local-1" && !s.is_team));
        assert!(book.servers.iter().any(|s| s.id == "team-3" && s.is_team));
        assert!(!book.servers.iter().any(|s| s.id == "team-1"));
        assert!(!book.servers.iter().any(|s| s.id == "team-2"));
    }

    // --- Connection manager state machine ---

    #[test]
    fn test_connection_manager_state_transitions() {
        let mgr = ConnectionManager::new();
        assert_eq!(mgr.state(), ConnectionState::Disconnected);
        assert!(!mgr.is_connected());
        assert!(!mgr.has_error());
    }

    #[test]
    fn test_connection_manager_input_queue_ordering() {
        let mgr = ConnectionManager::new();
        mgr.send_key_press(0x41, 0x41);
        mgr.send_pointer(100, 200, 1);
        mgr.send_clipboard("test");
        mgr.send_key_release(0x41);
        mgr.set_encoding(7);
        mgr.set_quality(8);
        mgr.set_compress(2);

        let guard = mgr.inner.lock().unwrap();
        assert_eq!(guard.input_queue.len(), 7);
    }

    // --- JSON serialization ---

    #[test]
    fn test_server_entry_json_roundtrip() {
        let entry = ServerEntry {
            name: "JSON Test".to_string(),
            host: "10.0.0.5".to_string(),
            port: 5902,
            group: "Dev".to_string(),
            tags: vec!["test".to_string()],
            username: "admin".to_string(),
            ..Default::default()
        };

        let json = serde_json::to_string(&entry).unwrap();
        let back: ServerEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(back.name, "JSON Test");
        assert_eq!(back.host, "10.0.0.5");
        assert_eq!(back.port, 5902);
        assert_eq!(back.tags, vec!["test"]);
        assert_eq!(back.username, "admin");
    }

    #[test]
    fn test_address_book_json_roundtrip() {
        let mut book = AddressBook::default();
        book.add(ServerEntry {
            name: "s1".to_string(),
            host: "h1".to_string(),
            ..Default::default()
        });
        book.add(ServerEntry {
            name: "s2".to_string(),
            host: "h2".to_string(),
            port: 5901,
            ..Default::default()
        });

        let json = serde_json::to_string(&book).unwrap();
        let back: AddressBook = serde_json::from_str(&json).unwrap();
        assert_eq!(back.servers.len(), 2);
    }

    #[test]
    fn test_sync_config_json_roundtrip() {
        let config = SyncConfig {
            url: "https://gist.github.com/raw/abc123".to_string(),
            interval_secs: 600,
            enabled: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: SyncConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.url, config.url);
        assert_eq!(back.interval_secs, 600);
        assert!(back.enabled);
    }

    // --- Display address ---

    #[test]
    fn test_display_address_variants() {
        let default_port = ServerEntry {
            host: "myhost".to_string(),
            port: 5900,
            ..Default::default()
        };
        assert_eq!(default_port.display_address(), "myhost");

        let custom_port = ServerEntry {
            host: "myhost".to_string(),
            port: 5901,
            ..Default::default()
        };
        assert_eq!(custom_port.display_address(), "myhost:1");

        let ip_addr = ServerEntry {
            host: "192.168.1.1".to_string(),
            port: 5900,
            ..Default::default()
        };
        assert_eq!(ip_addr.display_address(), "192.168.1.1");
    }

    // --- FrameBuffer ---

    #[test]
    fn test_framebuffer_pixel_operations() {
        let mut fb = FrameBuffer::new(100, 100);
        assert_eq!(fb.pixel_at(0, 0), [0, 0, 0, 0]);

        fb.set_pixel(50, 50, [255, 128, 64, 255]);
        assert_eq!(fb.pixel_at(50, 50), [255, 128, 64, 255]);

        // Out of bounds read returns sentinel
        assert_eq!(fb.pixel_at(200, 200), [0, 0, 0, 255]);
    }

    // --- Groups ---

    #[test]
    fn test_servers_by_group() {
        let mut book = AddressBook::default();
        for i in 0..5 {
            let mut entry = ServerEntry {
                name: format!("s{}", i),
                host: format!("h{}", i),
                ..Default::default()
            };
            entry.group = if i < 3 {
                "A".to_string()
            } else {
                "B".to_string()
            };
            book.add(entry);
        }

        let by_group = book.servers_by_group();
        assert_eq!(by_group["A"].len(), 3);
        assert_eq!(by_group["B"].len(), 2);
    }
}
