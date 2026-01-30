use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub auth: AuthConfig,
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub notifications: NotificationConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub bearer_token: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ProxyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiConfig {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    #[serde(default = "default_request_delay")]
    pub request_delay_ms: u64,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            user_agent: default_user_agent(),
            request_delay_ms: default_request_delay(),
        }
    }
}

fn default_base_url() -> String {
    "https://www.olx.pt/api/v1/offers".to_string()
}

fn default_user_agent() -> String {
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string()
}

const fn default_request_delay() -> u64 {
    1000
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct NotificationConfig {
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub notify_on_new_listing: bool,
    #[serde(default)]
    pub notify_on_price_drop: bool,
    #[serde(default)]
    pub notify_on_deal: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self { path: default_db_path() }
    }
}

fn default_db_path() -> String {
    "olx_tracker.db".to_string()
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;

        let config: Self =
            toml::from_str(&content).with_context(|| "Failed to parse config file")?;

        config.validate()?;
        Ok(config)
    }

    pub fn load_default() -> Result<Self> {
        let paths = ["config.toml", "~/.config/olx-tracker/config.toml"];

        for path in &paths {
            let expanded = shellexpand::tilde(path);
            if Path::new(expanded.as_ref()).exists() {
                return Self::load(expanded.as_ref());
            }
        }

        anyhow::bail!(
            "No config file found. Create config.toml or ~/.config/olx-tracker/config.toml"
        )
    }

    fn validate(&self) -> Result<()> {
        if self.auth.bearer_token.is_empty() {
            anyhow::bail!("Bearer token is required in config");
        }

        if self.proxy.enabled && self.proxy.url.is_none() {
            anyhow::bail!("Proxy URL is required when proxy is enabled");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
            [auth]
            bearer_token = "test_token"
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.auth.bearer_token, "test_token");
        assert!(!config.proxy.enabled);
        assert_eq!(config.api.request_delay_ms, 1000);
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
            [auth]
            bearer_token = "test_token"

            [proxy]
            enabled = true
            url = "socks5://localhost:1080"

            [api]
            base_url = "https://custom.api.com"
            request_delay_ms = 2000

            [notifications]
            webhook_url = "https://webhook.example.com"
            notify_on_new_listing = true

            [database]
            path = "custom.db"
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.proxy.enabled);
        assert_eq!(config.proxy.url, Some("socks5://localhost:1080".to_string()));
        assert_eq!(config.api.request_delay_ms, 2000);
        assert_eq!(config.database.path, "custom.db");
    }
}
