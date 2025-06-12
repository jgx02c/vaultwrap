// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use shared::{SecretRequest, SecretResponse};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
struct ServerConfig {
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
}

// Global state for server configuration
static SERVER_CONFIG: Mutex<Option<ServerConfig>> = Mutex::new(None);

#[derive(Serialize, Deserialize)]
struct Environment {
    name: String,
    variables: HashMap<String, String>,
}

// Tauri command to set server configuration
#[tauri::command]
async fn set_server_config(host: String, port: u16, username: Option<String>, password: Option<String>) -> Result<String, String> {
    let config = ServerConfig {
        host,
        port,
        username,
        password,
    };
    
    let mut server_config = SERVER_CONFIG.lock().unwrap();
    *server_config = Some(config);
    
    Ok("Server configuration updated".to_string())
}

// Tauri command to test connection
#[tauri::command]
async fn test_connection(host: String, port: u16) -> Result<String, String> {
    let config = ServerConfig {
        host,
        port,
        username: None,
        password: None,
    };
    
    match send_request_with_config("list-environments", None, &config).await {
        Ok(_) => Ok("Connection successful".to_string()),
        Err(e) => Err(format!("Connection failed: {}", e))
    }
}

// Tauri command to list all environments
#[tauri::command]
async fn list_environments() -> Result<Vec<String>, String> {
    let config = {
        let server_config = SERVER_CONFIG.lock().unwrap();
        server_config.as_ref().ok_or("No server configuration set")?.clone()
    };
    
    match send_request_with_config("list-environments", None, &config).await {
        Ok(response) => {
            if response.success {
                Ok(response.environments.unwrap_or_default())
            } else {
                Err(response.message.unwrap_or("Failed to list environments".to_string()))
            }
        }
        Err(e) => Err(format!("Network error: {}", e))
    }
}

// Tauri command to get variables for a specific environment
#[tauri::command]
async fn get_environment(env_name: String) -> Result<HashMap<String, String>, String> {
    let config = {
        let server_config = SERVER_CONFIG.lock().unwrap();
        server_config.as_ref().ok_or("No server configuration set")?.clone()
    };
    
    match send_request_with_config("shell-activation", Some(env_name.clone()), &config).await {
        Ok(response) => {
            if response.success {
                let mut vars = HashMap::new();
                if let Some(env_vars) = response.env_vars {
                    for (key, value) in env_vars {
                        vars.insert(key, value);
                    }
                }
                Ok(vars)
            } else {
                Err(response.message.unwrap_or(format!("Environment '{}' not found", env_name)))
            }
        }
        Err(e) => Err(format!("Network error: {}", e))
    }
}

// Tauri command to save environments (this will require extending the server protocol)
#[tauri::command]
async fn save_environment(env_name: String, variables: HashMap<String, String>) -> Result<String, String> {
    // For now, we'll return a placeholder since we need to extend the server protocol
    // to support saving environments
    Ok(format!("Saving environment '{}' with {} variables (not yet implemented in server)", env_name, variables.len()))
}

// Helper function to send requests to vaultd server with specific config
async fn send_request_with_config(command: &str, environment: Option<String>, config: &ServerConfig) -> Result<SecretResponse, Box<dyn std::error::Error>> {
    let address = format!("{}:{}", config.host, config.port);
    let mut stream = TcpStream::connect(&address).await?;
    
    let request = SecretRequest {
        client_id: "vaultd-gui".to_string(),
        command: command.to_string(),
        environment,
    };
    
    let request_json = serde_json::to_vec(&request)?;
    stream.write_all(&request_json).await?;
    
    let mut response_buf = vec![0; 4096];
    let n = stream.read(&mut response_buf).await?;
    
    let response: SecretResponse = serde_json::from_slice(&response_buf[..n])?;
    Ok(response)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            set_server_config,
            test_connection,
            list_environments,
            get_environment,
            save_environment
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
