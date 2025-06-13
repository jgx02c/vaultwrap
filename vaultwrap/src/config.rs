use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use dirs::home_dir;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Connection {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub default: Option<String>,
    pub connections: std::collections::HashMap<String, Connection>,
    pub last_set_env: Option<String>,
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
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Config::default()
    }
}

pub fn save_config(config: &Config) {
    let path = config_path();
    let data = serde_json::to_string_pretty(config).expect("Failed to serialize config");
    fs::write(path, data).expect("Failed to write config");
} 