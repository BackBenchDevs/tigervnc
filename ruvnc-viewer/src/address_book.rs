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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEntry {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub group: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub is_team: bool,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_connected: Option<DateTime<Utc>>,
}

impl Default for ServerEntry {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: String::new(),
            host: String::new(),
            port: 5900,
            group: "Default".to_string(),
            tags: Vec::new(),
            username: String::new(),
            notes: String::new(),
            is_team: false,
            created_at: Some(Utc::now()),
            last_connected: None,
        }
    }
}

/// Convert a VNC display number (0-99) to a TCP port, or pass through if >= 100.
pub fn resolve_port(port: u16) -> u16 {
    if port < 100 {
        5900 + port
    } else {
        port
    }
}

/// If the TCP port is in the VNC range (5900-5999), return the display number.
pub fn display_number(port: u16) -> Option<u16> {
    if (5900..6000).contains(&port) {
        Some(port - 5900)
    } else {
        None
    }
}

impl ServerEntry {
    pub fn display_address(&self) -> String {
        if let Some(dn) = display_number(self.port) {
            if dn == 0 {
                self.host.clone()
            } else {
                format!("{}:{}", self.host, dn)
            }
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    pub fn matches_search(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let q = query.to_lowercase();
        self.name.to_lowercase().contains(&q)
            || self.host.to_lowercase().contains(&q)
            || self.group.to_lowercase().contains(&q)
            || self.tags.iter().any(|t| t.to_lowercase().contains(&q))
            || self.notes.to_lowercase().contains(&q)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressBook {
    pub servers: Vec<ServerEntry>,
    #[serde(default)]
    pub groups: Vec<String>,
}

impl Default for AddressBook {
    fn default() -> Self {
        Self {
            servers: Vec::new(),
            groups: vec!["Default".to_string()],
        }
    }
}

impl AddressBook {
    pub fn storage_path() -> PathBuf {
        let base = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ruvnc-viewer");
        fs::create_dir_all(&base).ok();
        base.join("servers.json")
    }

    pub fn load() -> Self {
        let path = Self::storage_path();
        match fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::storage_path();
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
    }

    pub fn add(&mut self, entry: ServerEntry) {
        if !self.groups.contains(&entry.group) {
            self.groups.push(entry.group.clone());
        }
        self.servers.push(entry);
        self.save().ok();
    }

    pub fn remove(&mut self, id: &str) {
        self.servers.retain(|s| s.id != id);
        self.save().ok();
    }

    pub fn update(&mut self, entry: ServerEntry) {
        if let Some(existing) = self.servers.iter_mut().find(|s| s.id == entry.id) {
            *existing = entry;
        }
        self.save().ok();
    }

    pub fn find(&self, id: &str) -> Option<&ServerEntry> {
        self.servers.iter().find(|s| s.id == id)
    }

    pub fn find_by_host_port(&self, host: &str, port: u16) -> Option<&ServerEntry> {
        self.servers
            .iter()
            .find(|s| s.host == host && s.port == port)
    }

    #[allow(dead_code)]
    pub fn servers_by_group(&self) -> HashMap<String, Vec<&ServerEntry>> {
        let mut map: HashMap<String, Vec<&ServerEntry>> = HashMap::new();
        for server in &self.servers {
            map.entry(server.group.clone()).or_default().push(server);
        }
        map
    }

    pub fn mark_connected(&mut self, id: &str) {
        if let Some(entry) = self.servers.iter_mut().find(|s| s.id == id) {
            entry.last_connected = Some(Utc::now());
        }
        self.save().ok();
    }

    pub fn merge_team_servers(&mut self, team_servers: Vec<ServerEntry>) {
        let local_ids: std::collections::HashSet<String> = self
            .servers
            .iter()
            .filter(|s| !s.is_team)
            .map(|s| s.id.clone())
            .collect();

        self.servers.retain(|s| !s.is_team);

        for mut server in team_servers {
            if !local_ids.contains(&server.id) {
                server.is_team = true;
                self.servers.push(server);
            }
        }
        self.save().ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(name: &str, host: &str, port: u16) -> ServerEntry {
        ServerEntry {
            name: name.to_string(),
            host: host.to_string(),
            port,
            ..Default::default()
        }
    }

    // --- ServerEntry tests ---

    #[test]
    fn test_default_entry_has_uuid() {
        let e1 = ServerEntry::default();
        let e2 = ServerEntry::default();
        assert!(!e1.id.is_empty());
        assert_ne!(e1.id, e2.id, "each default entry gets a unique id");
    }

    #[test]
    fn test_default_entry_fields() {
        let entry = ServerEntry::default();
        assert_eq!(entry.port, 5900);
        assert_eq!(entry.group, "Default");
        assert!(entry.tags.is_empty());
        assert!(entry.username.is_empty());
        assert!(entry.notes.is_empty());
        assert!(!entry.is_team);
        assert!(entry.created_at.is_some());
        assert!(entry.last_connected.is_none());
    }

    #[test]
    fn test_display_address_default_port() {
        let entry = make_entry("s", "10.0.0.1", 5900);
        assert_eq!(entry.display_address(), "10.0.0.1");
    }

    #[test]
    fn test_display_address_custom_port() {
        let entry = make_entry("s", "10.0.0.1", 5901);
        assert_eq!(entry.display_address(), "10.0.0.1:1");
    }

    #[test]
    fn test_display_address_hostname() {
        let entry = make_entry("s", "myserver.local", 5900);
        assert_eq!(entry.display_address(), "myserver.local");
    }

    #[test]
    fn test_display_address_non_vnc_port() {
        let entry = make_entry("s", "10.0.0.1", 9000);
        assert_eq!(entry.display_address(), "10.0.0.1:9000");
    }

    #[test]
    fn test_display_address_display_99() {
        let entry = make_entry("s", "host", 5999);
        assert_eq!(entry.display_address(), "host:99");
    }

    #[test]
    fn test_resolve_port_display_number() {
        assert_eq!(resolve_port(0), 5900);
        assert_eq!(resolve_port(1), 5901);
        assert_eq!(resolve_port(99), 5999);
    }

    #[test]
    fn test_resolve_port_tcp_port() {
        assert_eq!(resolve_port(100), 100);
        assert_eq!(resolve_port(5901), 5901);
        assert_eq!(resolve_port(9000), 9000);
    }

    #[test]
    fn test_display_number_vnc_range() {
        assert_eq!(display_number(5900), Some(0));
        assert_eq!(display_number(5901), Some(1));
        assert_eq!(display_number(5999), Some(99));
    }

    #[test]
    fn test_display_number_non_vnc() {
        assert_eq!(display_number(80), None);
        assert_eq!(display_number(6000), None);
        assert_eq!(display_number(9000), None);
    }

    #[test]
    fn test_search_empty_query_matches_all() {
        let entry = make_entry("anything", "anywhere", 5900);
        assert!(entry.matches_search(""));
    }

    #[test]
    fn test_search_by_name() {
        let entry = make_entry("Dev-Linux-01", "10.0.0.1", 5900);
        assert!(entry.matches_search("dev"));
        assert!(entry.matches_search("DEV"));
        assert!(entry.matches_search("linux"));
        assert!(entry.matches_search("01"));
    }

    #[test]
    fn test_search_by_host() {
        let entry = make_entry("s", "192.168.1.50", 5900);
        assert!(entry.matches_search("192.168"));
        assert!(entry.matches_search("168.1"));
    }

    #[test]
    fn test_search_by_group() {
        let mut entry = make_entry("s", "h", 5900);
        entry.group = "Production".to_string();
        assert!(entry.matches_search("prod"));
    }

    #[test]
    fn test_search_by_tags() {
        let mut entry = make_entry("s", "h", 5900);
        entry.tags = vec!["ubuntu".to_string(), "headless".to_string()];
        assert!(entry.matches_search("ubuntu"));
        assert!(entry.matches_search("headless"));
        assert!(!entry.matches_search("centos"));
    }

    #[test]
    fn test_search_by_notes() {
        let mut entry = make_entry("s", "h", 5900);
        entry.notes = "This is the staging box".to_string();
        assert!(entry.matches_search("staging"));
    }

    #[test]
    fn test_search_no_match() {
        let entry = make_entry("Alpha", "10.0.0.1", 5900);
        assert!(!entry.matches_search("zzzzz"));
    }

    // --- AddressBook tests ---

    #[test]
    fn test_default_book_has_default_group() {
        let book = AddressBook::default();
        assert!(book.servers.is_empty());
        assert_eq!(book.groups, vec!["Default".to_string()]);
    }

    #[test]
    fn test_add_creates_new_group() {
        let mut book = AddressBook::default();
        let mut entry = make_entry("s", "h", 5900);
        entry.group = "NewGroup".to_string();
        book.add(entry);
        assert!(book.groups.contains(&"NewGroup".to_string()));
    }

    #[test]
    fn test_add_does_not_duplicate_group() {
        let mut book = AddressBook::default();
        let e1 = make_entry("s1", "h1", 5900);
        let e2 = make_entry("s2", "h2", 5900);
        book.add(e1);
        book.add(e2);
        let count = book
            .groups
            .iter()
            .filter(|g| g.as_str() == "Default")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_find_existing() {
        let mut book = AddressBook::default();
        let entry = make_entry("s", "h", 5900);
        let id = entry.id.clone();
        book.add(entry);
        assert!(book.find(&id).is_some());
    }

    #[test]
    fn test_find_missing() {
        let book = AddressBook::default();
        assert!(book.find("nonexistent").is_none());
    }

    #[test]
    fn test_remove() {
        let mut book = AddressBook::default();
        let entry = make_entry("s", "h", 5900);
        let id = entry.id.clone();
        book.add(entry);
        assert_eq!(book.servers.len(), 1);
        book.remove(&id);
        assert_eq!(book.servers.len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_is_noop() {
        let mut book = AddressBook::default();
        book.add(make_entry("s", "h", 5900));
        book.remove("does-not-exist");
        assert_eq!(book.servers.len(), 1);
    }

    #[test]
    fn test_update_existing() {
        let mut book = AddressBook::default();
        let entry = make_entry("old_name", "h", 5900);
        let id = entry.id.clone();
        book.add(entry);

        let mut updated = book.find(&id).unwrap().clone();
        updated.name = "new_name".to_string();
        book.update(updated);

        assert_eq!(book.find(&id).unwrap().name, "new_name");
    }

    #[test]
    fn test_update_nonexistent_is_noop() {
        let mut book = AddressBook::default();
        let ghost = ServerEntry {
            id: "ghost".to_string(),
            name: "ghost".to_string(),
            ..Default::default()
        };
        book.update(ghost);
        assert!(book.servers.is_empty());
    }

    #[test]
    fn test_mark_connected() {
        let mut book = AddressBook::default();
        let entry = make_entry("s", "h", 5900);
        let id = entry.id.clone();
        book.add(entry);
        assert!(book.find(&id).unwrap().last_connected.is_none());
        book.mark_connected(&id);
        assert!(book.find(&id).unwrap().last_connected.is_some());
    }

    #[test]
    fn test_servers_by_group() {
        let mut book = AddressBook::default();
        let mut e1 = make_entry("s1", "h1", 5900);
        e1.group = "A".to_string();
        let mut e2 = make_entry("s2", "h2", 5900);
        e2.group = "B".to_string();
        let mut e3 = make_entry("s3", "h3", 5900);
        e3.group = "A".to_string();
        book.add(e1);
        book.add(e2);
        book.add(e3);

        let by_group = book.servers_by_group();
        assert_eq!(by_group.get("A").unwrap().len(), 2);
        assert_eq!(by_group.get("B").unwrap().len(), 1);
    }

    #[test]
    fn test_merge_team_servers_adds_new() {
        let mut book = AddressBook::default();
        book.add(make_entry("local", "localhost", 5900));

        let team = vec![ServerEntry {
            id: "team-1".to_string(),
            name: "Team".to_string(),
            host: "10.0.0.5".to_string(),
            is_team: true,
            ..Default::default()
        }];

        book.merge_team_servers(team);
        assert_eq!(book.servers.len(), 2);
        assert!(book.servers.iter().any(|s| s.id == "team-1" && s.is_team));
    }

    #[test]
    fn test_merge_team_servers_replaces_old_team() {
        let mut book = AddressBook::default();
        book.add(make_entry("local", "localhost", 5900));

        let team_v1 = vec![ServerEntry {
            id: "team-1".to_string(),
            name: "Old Team".to_string(),
            host: "10.0.0.1".to_string(),
            is_team: true,
            ..Default::default()
        }];
        book.merge_team_servers(team_v1);
        assert_eq!(book.servers.len(), 2);

        let team_v2 = vec![ServerEntry {
            id: "team-2".to_string(),
            name: "New Team".to_string(),
            host: "10.0.0.2".to_string(),
            is_team: true,
            ..Default::default()
        }];
        book.merge_team_servers(team_v2);

        assert_eq!(book.servers.len(), 2);
        assert!(!book.servers.iter().any(|s| s.id == "team-1"));
        assert!(book.servers.iter().any(|s| s.id == "team-2"));
    }

    #[test]
    fn test_merge_team_does_not_overwrite_local_with_same_id() {
        let mut book = AddressBook::default();
        let local = ServerEntry {
            id: "shared-id".to_string(),
            name: "My Local".to_string(),
            host: "localhost".to_string(),
            is_team: false,
            ..Default::default()
        };
        book.add(local);

        let team = vec![ServerEntry {
            id: "shared-id".to_string(),
            name: "Team Override".to_string(),
            host: "10.0.0.1".to_string(),
            is_team: true,
            ..Default::default()
        }];
        book.merge_team_servers(team);

        assert_eq!(book.servers.len(), 1);
        assert_eq!(book.find("shared-id").unwrap().name, "My Local");
    }

    // --- Serialization tests ---

    #[test]
    fn test_server_entry_json_roundtrip() {
        let entry = ServerEntry {
            id: "test-id".to_string(),
            name: "Test".to_string(),
            host: "10.0.0.1".to_string(),
            port: 5901,
            group: "Dev".to_string(),
            tags: vec!["linux".to_string(), "gpu".to_string()],
            username: "admin".to_string(),
            notes: "Some notes".to_string(),
            is_team: false,
            created_at: Some(Utc::now()),
            last_connected: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let back: ServerEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(back.id, "test-id");
        assert_eq!(back.name, "Test");
        assert_eq!(back.host, "10.0.0.1");
        assert_eq!(back.port, 5901);
        assert_eq!(back.tags.len(), 2);
        assert_eq!(back.username, "admin");
    }

    #[test]
    fn test_address_book_json_roundtrip() {
        let mut book = AddressBook::default();
        book.add(make_entry("s1", "h1", 5900));
        book.add(make_entry("s2", "h2", 5901));

        let json = serde_json::to_string(&book).unwrap();
        let back: AddressBook = serde_json::from_str(&json).unwrap();

        assert_eq!(back.servers.len(), 2);
        assert!(back.groups.contains(&"Default".to_string()));
    }

    #[test]
    fn test_deserialize_minimal_json() {
        let json = r#"{
            "id": "min",
            "name": "Minimal",
            "host": "h",
            "port": 5900,
            "group": "G",
            "tags": []
        }"#;
        let entry: ServerEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.id, "min");
        assert!(entry.username.is_empty());
        assert!(!entry.is_team);
    }

    #[test]
    fn test_deserialize_empty_book_json() {
        let json = r#"{"servers":[]}"#;
        let book: AddressBook = serde_json::from_str(json).unwrap();
        assert!(book.servers.is_empty());
        assert!(book.groups.is_empty());
    }
}
