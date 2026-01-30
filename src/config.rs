use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;
use std::str::FromStr;

/// Supported OLX countries
#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OlxCountry {
    #[default]
    #[serde(alias = "PT")]
    Pt,
    #[serde(alias = "PL")]
    Pl,
    #[serde(alias = "UA")]
    Ua,
    #[serde(alias = "RO")]
    Ro,
    #[serde(alias = "BG")]
    Bg,
    #[serde(alias = "KZ")]
    Kz,
    #[serde(alias = "UZ")]
    Uz,
}

impl OlxCountry {
    pub const fn api_base_url(self) -> &'static str {
        match self {
            Self::Pt => "https://www.olx.pt/api/v1/offers",
            Self::Pl => "https://www.olx.pl/api/v1/offers",
            Self::Ua => "https://www.olx.ua/api/v1/offers",
            Self::Ro => "https://www.olx.ro/api/v1/offers",
            Self::Bg => "https://www.olx.bg/api/v1/offers",
            Self::Kz => "https://www.olx.kz/api/v1/offers",
            Self::Uz => "https://www.olx.uz/api/v1/offers",
        }
    }
}

impl FromStr for OlxCountry {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pt" | "portugal" => Ok(Self::Pt),
            "pl" | "poland" => Ok(Self::Pl),
            "ua" | "ukraine" => Ok(Self::Ua),
            "ro" | "romania" => Ok(Self::Ro),
            "bg" | "bulgaria" => Ok(Self::Bg),
            "kz" | "kazakhstan" => Ok(Self::Kz),
            "uz" | "uzbekistan" => Ok(Self::Uz),
            _ => Err(format!("Unknown country: {s}. Valid: pt, pl, ua, ro, bg, kz, uz")),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub notifications: NotificationConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub deals: DealConfig,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct AuthConfig {
    /// Bearer token (optional - OLX search is public)
    #[serde(default)]
    pub bearer_token: Option<String>,
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
    /// OLX country (pt, pl, ua, ro, bg, kz, uz)
    #[serde(default)]
    pub country: OlxCountry,
    /// Custom base URL (overrides country)
    pub base_url: Option<String>,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    #[serde(default = "default_request_delay")]
    pub request_delay_ms: u64,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            country: OlxCountry::default(),
            base_url: None,
            user_agent: default_user_agent(),
            request_delay_ms: default_request_delay(),
        }
    }
}

impl ApiConfig {
    pub fn get_base_url(&self) -> &str {
        self.base_url.as_deref().unwrap_or_else(|| self.country.api_base_url())
    }
}

fn default_user_agent() -> String {
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string()
}

const fn default_request_delay() -> u64 {
    1000
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct NotificationConfig {
    /// Generic webhook URL
    pub webhook_url: Option<String>,
    /// Discord webhook URL (uses Discord-specific formatting)
    pub discord_webhook_url: Option<String>,
    #[serde(default)]
    pub notify_on_new_listing: bool,
    #[serde(default)]
    pub notify_on_price_drop: bool,
    #[serde(default)]
    pub notify_on_deal: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DealConfig {
    /// Percentage below average to consider a "good deal" (e.g., 30 = 30% below avg)
    #[serde(default = "default_deal_threshold")]
    pub threshold_pct: f64,
    /// Target price - any listing at or below this is a deal
    pub target_price: Option<f64>,
}

impl Default for DealConfig {
    fn default() -> Self {
        Self { threshold_pct: default_deal_threshold(), target_price: None }
    }
}

const fn default_deal_threshold() -> f64 {
    20.0 // 20% below average by default
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

    /// Create a minimal config without a file (for CLI-only usage)
    pub fn minimal() -> Self {
        Self::default()
    }

    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        Self::load(path).unwrap_or_default()
    }

    fn validate(&self) -> Result<()> {
        if self.proxy.enabled && self.proxy.url.is_none() {
            anyhow::bail!("Proxy URL is required when proxy is enabled");
        }

        if self.deals.threshold_pct < 0.0 || self.deals.threshold_pct > 100.0 {
            anyhow::bail!("Deal threshold percentage must be between 0 and 100");
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
            [api]
            country = "pt"
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.auth.bearer_token.is_none());
        assert!(!config.proxy.enabled);
        assert_eq!(config.api.request_delay_ms, 1000);
        assert_eq!(config.api.country, OlxCountry::Pt);
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
            country = "pl"
            request_delay_ms = 2000

            [notifications]
            webhook_url = "https://webhook.example.com"
            discord_webhook_url = "https://discord.com/api/webhooks/123/abc"
            notify_on_new_listing = true

            [database]
            path = "custom.db"

            [deals]
            threshold_pct = 30.0
            target_price = 299.0
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.proxy.enabled);
        assert_eq!(config.proxy.url, Some("socks5://localhost:1080".to_string()));
        assert_eq!(config.api.request_delay_ms, 2000);
        assert_eq!(config.api.country, OlxCountry::Pl);
        assert_eq!(config.database.path, "custom.db");
        assert_eq!(config.deals.threshold_pct, 30.0);
        assert_eq!(config.deals.target_price, Some(299.0));
    }

    #[test]
    fn test_country_urls() {
        assert_eq!(OlxCountry::Pt.api_base_url(), "https://www.olx.pt/api/v1/offers");
        assert_eq!(OlxCountry::Pl.api_base_url(), "https://www.olx.pl/api/v1/offers");
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.auth.bearer_token.is_none());
        assert_eq!(config.deals.threshold_pct, 20.0);
    }

    #[test]
    fn test_validation_proxy_enabled_without_url() {
        let toml = r#"
            [proxy]
            enabled = true
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Proxy URL is required"));
    }

    #[test]
    fn test_validation_negative_deal_threshold() {
        let toml = r#"
            [deals]
            threshold_pct = -10.0
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("between 0 and 100"));
    }

    #[test]
    fn test_validation_deal_threshold_above_100() {
        let toml = r#"
            [deals]
            threshold_pct = 150.0
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("between 0 and 100"));
    }

    #[test]
    fn test_get_base_url_uses_country_default() {
        let config = ApiConfig { country: OlxCountry::Pt, base_url: None, ..Default::default() };
        assert_eq!(config.get_base_url(), "https://www.olx.pt/api/v1/offers");
    }

    #[test]
    fn test_get_base_url_uses_custom_override() {
        let config = ApiConfig {
            country: OlxCountry::Pt,
            base_url: Some("https://custom.api.example.com".to_string()),
            ..Default::default()
        };
        assert_eq!(config.get_base_url(), "https://custom.api.example.com");
    }

    #[test]
    fn test_minimal_config() {
        let config = Config::minimal();
        assert!(config.auth.bearer_token.is_none());
        assert!(!config.proxy.enabled);
        assert_eq!(config.deals.threshold_pct, 20.0);
    }

    #[test]
    fn test_load_or_default_with_nonexistent_file() {
        let config = Config::load_or_default("nonexistent.toml");
        assert_eq!(config.deals.threshold_pct, 20.0); // Should return default
    }

    #[test]
    fn test_proxy_config_default() {
        let proxy = ProxyConfig::default();
        assert!(!proxy.enabled);
        assert!(proxy.url.is_none());
    }

    #[test]
    fn test_notification_config_all_fields() {
        let toml = r#"
            [notifications]
            webhook_url = "https://webhook.example.com"
            discord_webhook_url = "https://discord.example.com"
            notify_on_new_listing = true
            notify_on_price_drop = true
            notify_on_deal = true
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.notifications.webhook_url, Some("https://webhook.example.com".to_string()));
        assert_eq!(config.notifications.discord_webhook_url, Some("https://discord.example.com".to_string()));
        assert!(config.notifications.notify_on_new_listing);
        assert!(config.notifications.notify_on_price_drop);
        assert!(config.notifications.notify_on_deal);
    }

    #[test]
    fn test_notification_config_defaults() {
        let config = NotificationConfig::default();
        assert!(config.webhook_url.is_none());
        assert!(config.discord_webhook_url.is_none());
        assert!(!config.notify_on_new_listing);
        assert!(!config.notify_on_price_drop);
        assert!(!config.notify_on_deal);
    }

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.path, "olx_tracker.db");
    }

    #[test]
    fn test_deal_config_default() {
        let config = DealConfig::default();
        assert_eq!(config.threshold_pct, 20.0);
        assert!(config.target_price.is_none());
    }

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.country, OlxCountry::Pt);
        assert!(config.base_url.is_none());
        assert_eq!(config.request_delay_ms, 1000);
        assert!(config.user_agent.contains("Mozilla"));
    }

    #[test]
    fn test_all_countries() {
        assert_eq!(OlxCountry::Pt.api_base_url(), "https://www.olx.pt/api/v1/offers");
        assert_eq!(OlxCountry::Pl.api_base_url(), "https://www.olx.pl/api/v1/offers");
        assert_eq!(OlxCountry::Ro.api_base_url(), "https://www.olx.ro/api/v1/offers");
        assert_eq!(OlxCountry::Bg.api_base_url(), "https://www.olx.bg/api/v1/offers");
        assert_eq!(OlxCountry::Ua.api_base_url(), "https://www.olx.ua/api/v1/offers");
        assert_eq!(OlxCountry::Kz.api_base_url(), "https://www.olx.kz/api/v1/offers");
        assert_eq!(OlxCountry::Uz.api_base_url(), "https://www.olx.uz/api/v1/offers");
    }
}
