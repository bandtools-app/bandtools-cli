use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub const DEFAULT_API_URL: &str = "https://bandtools.app/api/v1";
pub const TOKEN_ENV: &str = "BANDTOOLS_API_TOKEN";
pub const LEGACY_TOKEN_ENV: &str = "BT_API_TOKEN";
pub const API_URL_ENV: &str = "BANDTOOLS_API_URL";
pub const LEGACY_API_URL_ENV: &str = "BT_API_URL";
pub const CONFIG_ENV: &str = "BANDTOOLS_CONFIG";
pub const LEGACY_CONFIG_ENV: &str = "BT_CONFIG";

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct FileConfig {
    pub api_token: Option<String>,
    pub api_url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedConfig {
    pub api_token: String,
    pub api_url: String,
    pub config_path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct ConfigOverrides {
    pub api_token: Option<String>,
    pub api_url: Option<String>,
    pub config_path: Option<PathBuf>,
}

pub fn default_config_path() -> Result<PathBuf> {
    let base = dirs::config_dir().context("could not determine the user config directory")?;
    Ok(base.join("bandtools").join("config.toml"))
}

pub fn config_path(cli_path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = cli_path {
        return Ok(path);
    }

    if let Ok(path) = env::var(CONFIG_ENV).or_else(|_| env::var(LEGACY_CONFIG_ENV)) {
        return Ok(PathBuf::from(path));
    }

    default_config_path()
}

pub fn load(path: &PathBuf) -> Result<FileConfig> {
    if !path.exists() {
        return Ok(FileConfig::default());
    }

    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("failed to parse config file {}", path.display()))
}

pub fn save(path: &PathBuf, config: &FileConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }

    let raw = toml::to_string_pretty(config).context("failed to serialize config")?;
    fs::write(path, raw).with_context(|| format!("failed to write config file {}", path.display()))
}

pub fn resolve(overrides: ConfigOverrides) -> Result<ResolvedConfig> {
    let path = config_path(overrides.config_path.clone())?;
    let file = load(&path)?;

    let api_token = overrides
        .api_token
        .or_else(|| env::var(TOKEN_ENV).ok())
        .or_else(|| env::var(LEGACY_TOKEN_ENV).ok())
        .or(file.api_token)
        .filter(|token| !token.trim().is_empty());

    let api_url = overrides
        .api_url
        .or_else(|| env::var(API_URL_ENV).ok())
        .or_else(|| env::var(LEGACY_API_URL_ENV).ok())
        .or(file.api_url)
        .unwrap_or_else(|| DEFAULT_API_URL.to_string());

    let Some(api_token) = api_token else {
        bail!(
            "missing BandTools API token; pass --api-token, set {TOKEN_ENV}, or configure api_token"
        );
    };

    Ok(ResolvedConfig {
        api_token,
        api_url: normalise_api_url(&api_url)?,
        config_path: path,
    })
}

pub fn normalise_api_url(raw: &str) -> Result<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        bail!("API URL cannot be empty");
    }

    let parsed = url::Url::parse(trimmed).context("API URL must be an absolute URL")?;
    match parsed.scheme() {
        "http" | "https" => Ok(trimmed.to_string()),
        scheme => bail!("unsupported API URL scheme {scheme:?}; use http or https"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises_api_url() {
        assert_eq!(
            normalise_api_url("http://localhost:3000/api/v1/").unwrap(),
            "http://localhost:3000/api/v1"
        );
    }

    #[test]
    fn rejects_relative_api_url() {
        assert!(normalise_api_url("/api/v1").is_err());
    }
}
