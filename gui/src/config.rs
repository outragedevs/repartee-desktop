//! Minimal GUI config (`~/.repartee/gui.toml`). The full repartee config is far
//! richer; the MVP only needs enough to connect to one server.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: String,
    pub port: u16,
    pub tls: bool,
    pub nick: String,
    pub username: String,
    pub realname: String,
    #[serde(default)]
    pub channels: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        // A unique-ish nick so first launch doesn't collide; user can edit.
        let suffix = std::process::id() % 10_000;
        Self {
            server: "irc.libera.chat".to_string(),
            port: 6697,
            tls: true,
            nick: format!("reptee{suffix}"),
            username: "repartee".to_string(),
            realname: "Repartee GUI (iced) — https://repart.ee".to_string(),
            channels: vec![],
        }
    }
}

impl Config {
    /// Path to the GUI config file.
    #[must_use]
    pub fn path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".repartee")
            .join("gui.toml")
    }

    /// Load the config, writing a default template on first run.
    #[must_use]
    pub fn load_or_init() -> Self {
        let path = Self::path();
        if let Ok(s) = std::fs::read_to_string(&path) {
            match toml::from_str(&s) {
                Ok(cfg) => return cfg,
                Err(e) => tracing::warn!("invalid {}: {e}; using defaults", path.display()),
            }
        }
        let cfg = Self::default();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(s) = toml::to_string_pretty(&cfg) {
            if let Err(e) = std::fs::write(&path, s) {
                tracing::warn!("could not write {}: {e}", path.display());
            } else {
                tracing::info!("wrote default config to {}", path.display());
            }
        }
        cfg
    }
}
