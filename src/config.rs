use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::PathBuf;

/// Configuration loaded from the TOML config file for Meta WhatsApp Cloud API.
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// Long-lived system user access token with whatsapp_business_messaging permission.
    pub access_token: String,
    /// The WhatsApp Business Phone Number ID (numeric). Find it in WhatsApp Manager.
    pub phone_number_id: String,
    /// Graph API version to use (e.g. "v21.0"). Defaults to v21.0 if omitted.
    #[serde(default = "default_api_version")]
    pub api_version: String,
}

fn default_api_version() -> String {
    "v21.0".to_string()
}

/// Returns the path to the config file, using `dirs::config_dir()` for cross-platform support.
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
            PathBuf::from(home).join(".config")
        })
        .join("mcp-server-whatsapp")
        .join("config.toml")
}

/// Load and validate the config file.
pub fn load() -> Result<Config> {
    let path = config_path();
    let content = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "Failed to read config file: {}\n\
             Create it with your Meta WhatsApp Cloud API credentials.\n\
             Example:\n\n\
             access_token = \"EAAxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\"\n\
             phone_number_id = \"123456789012345\"\n\
             # api_version = \"v21.0\"   # optional, defaults to v21.0",
            path.display()
        )
    })?;
    let mut config: Config =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))?;

    if config.access_token.trim().is_empty() {
        bail!(
            "access_token in {} is empty — set it to your Meta system user access token",
            path.display()
        );
    }
    if config.phone_number_id.trim().is_empty() {
        bail!(
            "phone_number_id in {} is empty — set it to your WhatsApp Business Phone Number ID",
            path.display()
        );
    }
    if config.api_version.trim().is_empty() {
        config.api_version = default_api_version();
    }

    Ok(config)
}
