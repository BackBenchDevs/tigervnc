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

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use once_cell::sync::Lazy;
use unic_langid::LanguageIdentifier;

const EN_US_FTL: &str = include_str!("../../i18n/en-US.ftl");

static BUNDLE: Lazy<FluentBundle<FluentResource>> = Lazy::new(|| {
    let locale = detect_locale();
    create_bundle(&locale)
});

fn detect_locale() -> LanguageIdentifier {
    if let Some(locale_str) = sys_locale::get_locale() {
        let normalized = locale_str.replace('_', "-");
        if let Ok(langid) = normalized.parse::<LanguageIdentifier>() {
            log::info!("Detected system locale: {}", langid);
            return langid;
        }
    }
    log::info!("Using default locale: en-US");
    "en-US".parse().unwrap()
}

fn create_bundle(locale: &LanguageIdentifier) -> FluentBundle<FluentResource> {
    let mut bundle = FluentBundle::new_concurrent(vec![locale.clone()]);

    let resource = FluentResource::try_new(EN_US_FTL.to_string())
        .expect("Failed to parse en-US.ftl");
    bundle
        .add_resource(resource)
        .expect("Failed to add en-US resource to bundle");

    bundle
}

/// Look up a simple message with no arguments.
pub fn t(id: &str) -> String {
    let bundle = &*BUNDLE;
    let msg = match bundle.get_message(id) {
        Some(m) => m,
        None => {
            log::warn!("Missing i18n key: {}", id);
            return id.to_string();
        }
    };
    let pattern = match msg.value() {
        Some(p) => p,
        None => return id.to_string(),
    };
    let mut errors = vec![];
    bundle.format_pattern(pattern, None, &mut errors).to_string()
}

/// Look up a message with named arguments.
pub fn t_args(id: &str, args: &[(&str, &dyn FluentArgValue)]) -> String {
    let bundle = &*BUNDLE;
    let msg = match bundle.get_message(id) {
        Some(m) => m,
        None => {
            log::warn!("Missing i18n key: {}", id);
            return id.to_string();
        }
    };
    let pattern = match msg.value() {
        Some(p) => p,
        None => return id.to_string(),
    };

    let mut fluent_args = FluentArgs::new();
    for (key, val) in args {
        val.write_to(&mut fluent_args, key);
    }

    let mut errors = vec![];
    bundle
        .format_pattern(pattern, Some(&fluent_args), &mut errors)
        .to_string()
}

/// Trait to allow passing different value types to t_args.
pub trait FluentArgValue {
    fn write_to<'a>(&self, args: &mut FluentArgs<'a>, key: &'a str);
}

impl FluentArgValue for &str {
    fn write_to<'a>(&self, args: &mut FluentArgs<'a>, key: &'a str) {
        args.set(key, self.to_string());
    }
}

impl FluentArgValue for String {
    fn write_to<'a>(&self, args: &mut FluentArgs<'a>, key: &'a str) {
        args.set(key, self.clone());
    }
}

impl FluentArgValue for i64 {
    fn write_to<'a>(&self, args: &mut FluentArgs<'a>, key: &'a str) {
        args.set(key, *self);
    }
}

impl FluentArgValue for usize {
    fn write_to<'a>(&self, args: &mut FluentArgs<'a>, key: &'a str) {
        args.set(key, *self as i64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_lookup() {
        let result = t("app-title");
        assert_eq!(result, "RuVNC Viewer");
    }

    #[test]
    fn test_missing_key_returns_key() {
        let result = t("nonexistent-key");
        assert_eq!(result, "nonexistent-key");
    }

    #[test]
    fn test_args_lookup() {
        let version: &dyn FluentArgValue = &"0.1.0";
        let result = t_args("app-version", &[("version", version)]);
        assert!(result.contains("0.1.0"));
    }

    #[test]
    fn test_pluralization() {
        let one: &dyn FluentArgValue = &1_i64;
        let result = t_args("status-servers", &[("count", one)]);
        assert!(result.contains("server"), "Expected 'server' in '{}'", result);
        assert!(!result.contains("servers"), "Should be singular for 1, got '{}'", result);

        let many: &dyn FluentArgValue = &5_i64;
        let result = t_args("status-servers", &[("count", many)]);
        assert!(result.contains("servers"), "Expected 'servers' in '{}'", result);
    }

    #[test]
    fn test_detect_locale_returns_valid() {
        let locale = detect_locale();
        assert!(!locale.to_string().is_empty());
    }
}
