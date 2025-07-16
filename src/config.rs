use config::{Config, ConfigError, Environment, File};
use dirs_next::home_dir;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Settings {
    pub api_base: String,
    pub client_id: String,
    pub credentials_dir: PathBuf,
    pub profile: String,
    pub default_project: Option<String>,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        // 1) Which profile? default or prod
        let profile = std::env::var("ACCOMPLISH_ENV").unwrap_or_else(|_| "default".into());

        // 2) Path to ~/.accomplish/config.toml
        let mut path =
            home_dir().ok_or_else(|| ConfigError::Message("Could not find home dir".into()))?;
        path.push(".accomplish/config.toml");

        // 3) Create default config if it doesn't exist
        Self::ensure_default_config(&path)?;

        // 4) Load file + ENV
        let cfg = Config::builder()
            .add_source(File::with_name(path.to_str().unwrap()).required(false))
            .add_source(Environment::with_prefix("ACCOMPLISH").separator("__"))
            .build()?;

        // 5) Extract each setting under the chosen profile
        let api_base = cfg.get_string(&format!("{profile}.api_base"))?;
        let client_id = cfg.get_string(&format!("{profile}.client_id"))?;
        let cred_dir_raw = cfg.get_string(&format!("{profile}.credentials_dir"))?;

        // 6) Expand leading '~' if present
        let credentials_dir = if let Some(path_without_tilde) = cred_dir_raw.strip_prefix("~/") {
            let mut home = home_dir().ok_or_else(|| {
                ConfigError::Message("Cannot expand '~' in credentials_dir".into())
            })?;
            home.push(path_without_tilde);
            home
        } else {
            PathBuf::from(cred_dir_raw)
        };

        // 7) Optional global default project
        let default_project = match cfg.get_string(&format!("{profile}.default_project")) {
            Ok(s) if !s.is_empty() => Some(s),
            _ => None,
        };

        Ok(Settings {
            api_base,
            client_id,
            credentials_dir,
            profile,
            default_project,
        })
    }

    fn ensure_default_config(config_path: &Path) -> Result<(), ConfigError> {
        // Check if config file already exists
        if config_path.exists() {
            return Ok(());
        }

        // Create the directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ConfigError::Message(format!("Failed to create config directory: {e}"))
            })?;
        }

        // Create default configuration content
        let default_config = r#"[default]
api_base = "https://accomplish.dev"
client_id = "90w0AXnlNgnh2XBJdexYjw"
credentials_dir = "~/.accomplish"
"#;

        // Write the default configuration
        fs::write(config_path, default_config).map_err(|e| {
            ConfigError::Message(format!("Failed to create default config file: {e}"))
        })?;

        Ok(())
    }
}

pub fn lookup_default_project_for_dir(start: &Path) -> Option<String> {
    // First, check for local .accomplish.toml files up the directory tree
    let mut current = Some(start);
    while let Some(dir) = current {
        let config_path = dir.join(".accomplish.toml");
        if config_path.exists() {
            if let Ok(config) = Config::builder()
                .add_source(File::with_name(config_path.to_str().unwrap()))
                .build()
            {
                if let Ok(project) = config.get_string("project.default_project") {
                    return Some(project);
                }
            }
        }
        current = dir.parent();
    }

    // If no local config found, check global directories config
    lookup_global_project_for_dir(start)
}

fn lookup_global_project_for_dir(dir: &Path) -> Option<String> {
    let home = home_dir()?;
    let global_config_path = home.join(".accomplish/directories.toml");

    if !global_config_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&global_config_path).ok()?;
    let config: GlobalConfig = toml::from_str(&content).ok()?;

    let dir_key = dir.to_string_lossy().to_string();
    config
        .directories
        .get(&dir_key)
        .map(|entry| entry.project_identifier.clone())
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
struct GlobalConfig {
    directories: std::collections::HashMap<String, DirectoryEntry>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct DirectoryEntry {
    project_identifier: String,
    directory_type: String,
    git_remote: Option<String>,
}
