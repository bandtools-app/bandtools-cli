use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub const DEFAULT_API_URL: &str = "https://bandtools.app/api/v1";
pub const TOKEN_ENV: &str = "BANDTOOLS_API_TOKEN";
pub const LEGACY_TOKEN_ENV: &str = "BT_API_TOKEN";
pub const API_URL_ENV: &str = "BANDTOOLS_API_URL";
pub const LEGACY_API_URL_ENV: &str = "BT_API_URL";
pub const CONFIG_ENV: &str = "BANDTOOLS_CONFIG";
pub const LEGACY_CONFIG_ENV: &str = "BT_CONFIG";
pub const XDG_CONFIG_HOME_ENV: &str = "XDG_CONFIG_HOME";
pub const HOME_ENV: &str = "HOME";

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
    let base = env::var_os(XDG_CONFIG_HOME_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os(HOME_ENV).map(|home| PathBuf::from(home).join(".config")))
        .context("could not determine the user config directory")?;
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

pub fn load(path: &Path) -> Result<FileConfig> {
    if !path.exists() {
        return Ok(FileConfig::default());
    }

    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("failed to parse config file {}", path.display()))
}

pub fn save(path: &Path, config: &FileConfig) -> Result<()> {
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
    use std::{
        ffi::OsString,
        sync::{Mutex, OnceLock},
    };

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_lock() -> &'static Mutex<()> {
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvGuard {
        vars: Vec<(&'static str, Option<OsString>)>,
    }

    impl EnvGuard {
        fn new(names: &[&'static str]) -> Self {
            Self {
                vars: names
                    .iter()
                    .map(|name| (*name, env::var_os(name)))
                    .collect(),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (name, value) in &self.vars {
                unsafe {
                    if let Some(value) = value {
                        env::set_var(name, value);
                    } else {
                        env::remove_var(name);
                    }
                }
            }
        }
    }

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

    #[test]
    fn default_config_path_uses_xdg_config_home_first() {
        let _guard = env_lock().lock().unwrap();
        let _env_guard = EnvGuard::new(&[XDG_CONFIG_HOME_ENV, HOME_ENV]);
        let temp = tempfile::tempdir().unwrap();

        unsafe {
            env::set_var(XDG_CONFIG_HOME_ENV, temp.path());
            env::set_var(HOME_ENV, "/tmp/ignored-home");
        }

        assert_eq!(
            default_config_path().unwrap(),
            temp.path().join("bandtools").join("config.toml")
        );
    }

    #[test]
    fn default_config_path_falls_back_to_home_dot_config() {
        let _guard = env_lock().lock().unwrap();
        let _env_guard = EnvGuard::new(&[XDG_CONFIG_HOME_ENV, HOME_ENV]);
        let temp = tempfile::tempdir().unwrap();

        unsafe {
            env::remove_var(XDG_CONFIG_HOME_ENV);
            env::set_var(HOME_ENV, temp.path());
        }

        assert_eq!(
            default_config_path().unwrap(),
            temp.path()
                .join(".config")
                .join("bandtools")
                .join("config.toml")
        );
    }

    #[test]
    fn config_path_prefers_cli_path() {
        let _guard = env_lock().lock().unwrap();
        let _env_guard = EnvGuard::new(&[CONFIG_ENV]);
        let temp = tempfile::tempdir().unwrap();
        let cli_path = temp.path().join("cli.toml");

        unsafe {
            env::set_var(CONFIG_ENV, temp.path().join("env.toml"));
        }

        assert_eq!(config_path(Some(cli_path.clone())).unwrap(), cli_path);
    }

    #[test]
    fn config_path_prefers_bandtools_config_env_over_legacy_env() {
        let _guard = env_lock().lock().unwrap();
        let _env_guard = EnvGuard::new(&[CONFIG_ENV, LEGACY_CONFIG_ENV]);
        let temp = tempfile::tempdir().unwrap();
        let preferred = temp.path().join("preferred.toml");

        unsafe {
            env::set_var(CONFIG_ENV, &preferred);
            env::set_var(LEGACY_CONFIG_ENV, temp.path().join("legacy.toml"));
        }

        assert_eq!(config_path(None).unwrap(), preferred);
    }
}
