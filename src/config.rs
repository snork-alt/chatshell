use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use crate::llm::LlmConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub shell: ShellConfig,
    pub llm: LlmConfig,
    pub hooks: Vec<HookConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    pub name: String,
    pub key_combination: String,
    pub action: String,
    pub description: Option<String>,
    pub enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            shell: ShellConfig {
                command: "/bin/bash".to_string(),
                args: vec!["-i".to_string()], // Interactive mode
                env: None,
            },
            llm: LlmConfig::default(),
            hooks: vec![
                HookConfig {
                    name: "example_hook".to_string(),
                    key_combination: "ctrl+;".to_string(),
                    action: "echo 'Hook triggered!'".to_string(),
                    description: Some("Example hook for Ctrl+;".to_string()),
                    enabled: true,
                },
            ],
        }
    }
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;
        
        let config: Config = toml::from_str(&content)
            .with_context(|| "Failed to parse config file")?;
        
        Ok(config)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .with_context(|| "Failed to serialize config")?;
        
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {:?}", path.as_ref()))?;
        
        Ok(())
    }

    pub fn get_default_config_path() -> String {
        if let Some(home) = std::env::var_os("HOME") {
            format!("{}/.config/chatshell/config.toml", home.to_string_lossy())
        } else {
            "./chatshell.toml".to_string()
        }
    }

    pub fn ensure_config_exists() -> Result<String> {
        let config_path = Self::get_default_config_path();
        
        if !Path::new(&config_path).exists() {
            // Create directory if it doesn't exist
            if let Some(parent) = Path::new(&config_path).parent() {
                fs::create_dir_all(parent)
                    .with_context(|| "Failed to create config directory")?;
            }
            
            // Create default config
            let default_config = Config::default();
            default_config.save_to_file(&config_path)?;
            println!("Created default config at: {}", config_path);
        }
        
        Ok(config_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.shell.command, "/bin/bash");
        assert!(!config.hooks.is_empty());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        
        assert_eq!(config.shell.command, deserialized.shell.command);
    }
}