use crate::address_book::ServerEntry;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub url: String,
    pub interval_secs: u64,
    pub enabled: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            interval_secs: 300,
            enabled: false,
        }
    }
}

impl SyncConfig {
    fn storage_path() -> PathBuf {
        let base = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tigervnc-plus");
        fs::create_dir_all(&base).ok();
        base.join("sync_config.json")
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
}

struct SyncState {
    result: Option<Vec<ServerEntry>>,
    syncing: bool,
    last_error: Option<String>,
}

pub struct TeamSyncManager {
    config: SyncConfig,
    state: Arc<Mutex<SyncState>>,
}

impl TeamSyncManager {
    pub fn new() -> Self {
        let config = SyncConfig::load();
        Self::with_config(config)
    }

    pub fn with_config(config: SyncConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(SyncState {
                result: None,
                syncing: false,
                last_error: None,
            })),
        }
    }

    pub fn trigger_sync(&self) {
        if self.config.url.is_empty() {
            warn!("Team sync URL not configured");
            return;
        }

        {
            let mut state = self.state.lock().unwrap();
            if state.syncing {
                info!("Sync already in progress");
                return;
            }
            state.syncing = true;
            state.last_error = None;
        }

        let url = self.config.url.clone();
        let state = self.state.clone();

        thread::spawn(move || {
            info!("Starting team sync from: {}", url);

            match fetch_team_servers_blocking(&url) {
                Ok(servers) => {
                    info!("Team sync fetched {} servers", servers.len());
                    let mut s = state.lock().unwrap();
                    s.result = Some(servers);
                    s.syncing = false;
                }
                Err(e) => {
                    error!("Team sync failed: {}", e);
                    let mut s = state.lock().unwrap();
                    s.last_error = Some(e.to_string());
                    s.syncing = false;
                }
            }
        });
    }

    pub fn take_result(&mut self) -> Option<Vec<ServerEntry>> {
        let mut state = self.state.lock().unwrap();
        state.result.take()
    }

    pub fn is_syncing(&self) -> bool {
        self.state.lock().unwrap().syncing
    }

    pub fn last_error(&self) -> Option<String> {
        self.state.lock().unwrap().last_error.clone()
    }

    pub fn config(&self) -> &SyncConfig {
        &self.config
    }

    pub fn set_url(&mut self, url: String) {
        self.config.url = url;
        self.config.enabled = !self.config.url.is_empty();
        self.config.save().ok();
    }
}

fn fetch_team_servers_blocking(url: &str) -> Result<Vec<ServerEntry>, Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let response = reqwest::get(url).await?;

        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()).into());
        }

        let text = response.text().await?;

        let servers: Vec<ServerEntry> = if let Ok(s) = serde_json::from_str(&text) {
            s
        } else {
            #[derive(Deserialize)]
            struct Wrapper {
                servers: Vec<ServerEntry>,
            }
            let wrapper: Wrapper = serde_json::from_str(&text)?;
            wrapper.servers
        };

        Ok(servers)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- SyncConfig tests ---

    #[test]
    fn test_sync_config_default() {
        let config = SyncConfig::default();
        assert!(config.url.is_empty());
        assert_eq!(config.interval_secs, 300);
        assert!(!config.enabled);
    }

    #[test]
    fn test_sync_config_json_roundtrip() {
        let config = SyncConfig {
            url: "https://example.com/servers.json".to_string(),
            interval_secs: 600,
            enabled: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: SyncConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.url, "https://example.com/servers.json");
        assert_eq!(back.interval_secs, 600);
        assert!(back.enabled);
    }

    #[test]
    fn test_sync_config_deserialize_minimal() {
        let json = r#"{"url":"","interval_secs":300,"enabled":false}"#;
        let config: SyncConfig = serde_json::from_str(json).unwrap();
        assert!(config.url.is_empty());
        assert!(!config.enabled);
    }

    // --- TeamSyncManager tests ---

    fn test_manager() -> TeamSyncManager {
        TeamSyncManager::with_config(SyncConfig::default())
    }

    #[test]
    fn test_team_sync_manager_init() {
        let mgr = test_manager();
        assert!(!mgr.is_syncing());
        assert!(mgr.last_error().is_none());
    }

    #[test]
    fn test_trigger_sync_no_url() {
        let mgr = test_manager();
        mgr.trigger_sync();
        assert!(!mgr.is_syncing());
    }

    #[test]
    fn test_set_url_enables() {
        let mut mgr = test_manager();
        mgr.set_url("https://example.com/servers.json".to_string());
        assert_eq!(mgr.config().url, "https://example.com/servers.json");
        assert!(mgr.config().enabled);
    }

    #[test]
    fn test_set_empty_url_disables() {
        let mut mgr = test_manager();
        mgr.set_url("https://example.com".to_string());
        assert!(mgr.config().enabled);
        mgr.set_url(String::new());
        assert!(!mgr.config().enabled);
    }

    #[test]
    fn test_take_result_returns_none_initially() {
        let mut mgr = test_manager();
        assert!(mgr.take_result().is_none());
    }

    #[test]
    fn test_take_result_consumes() {
        let mgr = test_manager();
        {
            let mut state = mgr.state.lock().unwrap();
            state.result = Some(vec![ServerEntry {
                id: "t1".to_string(),
                name: "Test".to_string(),
                host: "h".to_string(),
                ..Default::default()
            }]);
        }
        let mut mgr = mgr;
        let result = mgr.take_result();
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 1);
        assert!(mgr.take_result().is_none());
    }

    // --- JSON parsing tests ---

    #[test]
    fn test_parse_team_json_bare_array() {
        let json = r#"[
            {
                "id": "t1",
                "name": "Team Server 1",
                "host": "10.0.0.1",
                "port": 5900,
                "group": "Team",
                "tags": ["prod"]
            },
            {
                "id": "t2",
                "name": "Team Server 2",
                "host": "10.0.0.2",
                "port": 5901,
                "group": "Team",
                "tags": []
            }
        ]"#;
        let servers: Vec<ServerEntry> = serde_json::from_str(json).unwrap();
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0].name, "Team Server 1");
        assert_eq!(servers[1].port, 5901);
    }

    #[test]
    fn test_parse_team_json_wrapper_object() {
        let json = r#"{
            "servers": [
                {
                    "id": "t2",
                    "name": "Team Server 2",
                    "host": "10.0.0.2",
                    "port": 5901,
                    "group": "Team",
                    "tags": []
                }
            ]
        }"#;

        #[derive(Deserialize)]
        struct Wrapper {
            servers: Vec<ServerEntry>,
        }
        let wrapper: Wrapper = serde_json::from_str(json).unwrap();
        assert_eq!(wrapper.servers.len(), 1);
        assert_eq!(wrapper.servers[0].host, "10.0.0.2");
    }

    #[test]
    fn test_parse_empty_array() {
        let json = "[]";
        let servers: Vec<ServerEntry> = serde_json::from_str(json).unwrap();
        assert!(servers.is_empty());
    }

    #[test]
    fn test_parse_server_with_optional_fields() {
        let json = r#"{
            "id": "opt",
            "name": "Optional",
            "host": "h",
            "port": 5900,
            "group": "G",
            "tags": [],
            "username": "admin",
            "notes": "some notes",
            "is_team": true
        }"#;
        let entry: ServerEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.username, "admin");
        assert_eq!(entry.notes, "some notes");
        assert!(entry.is_team);
    }

    #[test]
    fn test_parse_server_without_optional_fields() {
        let json = r#"{
            "id": "min",
            "name": "Minimal",
            "host": "h",
            "port": 5900,
            "group": "G",
            "tags": []
        }"#;
        let entry: ServerEntry = serde_json::from_str(json).unwrap();
        assert!(entry.username.is_empty());
        assert!(entry.notes.is_empty());
        assert!(!entry.is_team);
        assert!(entry.created_at.is_none());
    }
}
