use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use dirs::home_dir;

#[derive(Serialize, Deserialize, Clone)]
pub struct Connection {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub connections: HashMap<String, Connection>,
    pub default: Option<String>,
    pub last_set_env: Option<String>,
    pub runtime_injection: RuntimeInjectionConfig,
}

#[derive(Serialize, Deserialize)]
pub struct RuntimeInjectionConfig {
    pub enabled: bool,
    pub commands: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            connections: HashMap::new(),
            default: None,
            last_set_env: None,
            runtime_injection: RuntimeInjectionConfig {
                enabled: false,
                commands: Vec::new(),
            },
        }
    }
}

pub fn config_path() -> PathBuf {
    let mut path = home_dir().expect("Could not find home directory");
    path.push(".vaultwrap");
    fs::create_dir_all(&path).ok();
    path.push("config.json");
    path
}

pub fn load_config() -> Config {
    let path = config_path();
    if path.exists() {
        let data = fs::read_to_string(path).expect("Failed to read config");
        serde_json::from_str(&data).unwrap_or_else(|_| Config::default())
    } else {
        Config::default()
    }
}

pub fn save_config(config: &Config) {
    let path = config_path();
    let data = serde_json::to_string_pretty(config).expect("Failed to serialize config");
    fs::write(path, data).expect("Failed to write config");
} 