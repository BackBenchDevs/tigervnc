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

use thiserror::Error;

const SERVICE_NAME: &str = "ruvnc-viewer";

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum CredentialError {
    #[error("keyring error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("no password stored for {0}")]
    NotFound(String),
}

fn entry_key(host: &str, port: u16, username: &str) -> String {
    if username.is_empty() {
        format!("{}:{}", host, port)
    } else {
        format!("{}@{}:{}", username, host, port)
    }
}

pub fn store_password(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
) -> Result<(), CredentialError> {
    let key = entry_key(host, port, username);
    log::debug!("Storing password in keyring: key='{}'", key);
    let entry = keyring::Entry::new(SERVICE_NAME, &key)?;
    entry.set_password(password)?;
    log::info!("Password saved to keyring for {}:{}", host, port);
    Ok(())
}

pub fn get_password(host: &str, port: u16, username: &str) -> Result<String, CredentialError> {
    let key = entry_key(host, port, username);
    let entry = keyring::Entry::new(SERVICE_NAME, &key)?;
    match entry.get_password() {
        Ok(pw) => Ok(pw),
        Err(keyring::Error::NoEntry) => Err(CredentialError::NotFound(key)),
        Err(e) => Err(CredentialError::Keyring(e)),
    }
}

#[allow(dead_code)]
pub fn delete_password(host: &str, port: u16, username: &str) -> Result<(), CredentialError> {
    let key = entry_key(host, port, username);
    let entry = keyring::Entry::new(SERVICE_NAME, &key)?;
    entry.delete_credential()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_key_no_user() {
        assert_eq!(entry_key("10.0.0.1", 5900, ""), "10.0.0.1:5900");
    }

    #[test]
    fn test_entry_key_with_user() {
        assert_eq!(entry_key("10.0.0.1", 5901, "admin"), "admin@10.0.0.1:5901");
    }

    #[test]
    fn test_entry_key_special_chars() {
        assert_eq!(
            entry_key("my-host.example.com", 5900, "user@domain"),
            "user@domain@my-host.example.com:5900"
        );
    }

    #[test]
    fn test_entry_key_high_port() {
        assert_eq!(entry_key("h", 65535, ""), "h:65535");
    }

    #[test]
    fn test_entry_key_port_1() {
        assert_eq!(entry_key("h", 1, "u"), "u@h:1");
    }

    #[test]
    fn test_service_name_constant() {
        assert_eq!(SERVICE_NAME, "ruvnc-viewer");
    }

    #[test]
    fn test_error_display_not_found() {
        let err = CredentialError::NotFound("test-key".to_string());
        assert_eq!(format!("{}", err), "no password stored for test-key");
    }
}
