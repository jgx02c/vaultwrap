use shared::*;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;
use toml::Value;
use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:4000").await?;
    println!("[vaultd] Vault server listening on port 4000...");
    println!("[vaultd] Ready to serve environment variables on demand");

    // Load our environment variables into shared state
    let envs = Arc::new(Mutex::new(load_environment_variables()));

    loop {
        let (mut socket, addr) = listener.accept().await?;
        let envs = Arc::clone(&envs);

        tokio::spawn(async move {
            println!("[vaultd] New connection from: {}", addr);
            
            let mut buf = vec![0; 1024];
            match socket.read(&mut buf).await {
                Ok(n) if n == 0 => return,
                Ok(n) => {
                    match serde_json::from_slice::<SecretRequest>(&buf[..n]) {
                        Ok(req) => {
                            println!("[vaultd] Request from client '{}' to run: {}", req.client_id, req.command);
                            
                            // Handle list-environments command
                            if req.command == "list-environments" {
                                let environments = {
                                    let envs_guard = envs.lock().await;
                                    envs_guard.keys().cloned().collect()
                                };
                                let response = SecretResponse {
                                    success: true,
                                    env_vars: None,
                                    message: None,
                                    environments: Some(environments),
                                };
                                let out = serde_json::to_vec(&response).unwrap();
                                let _ = socket.write_all(&out).await;
                                return;
                            }

                            // Handle save-environment command
                            if req.command == "save-environment" {
                                let env_name = req.environment.as_deref().unwrap_or("dev");
                                
                                if let Some(variables) = req.variables {
                                    println!("[vaultd] Saving {} variables to environment '{}'", variables.len(), env_name);
                                    
                                    match save_environment_variables(env_name, variables).await {
                                        Ok(_) => {
                                            println!("[vaultd] Successfully saved environment '{}'", env_name);
                                            
                                            // Reload environment variables after save
                                            println!("[vaultd] Reloading environment variables from secrets.toml");
                                            let environments = {
                                                let mut envs_guard = envs.lock().await;
                                                *envs_guard = load_environment_variables();
                                                envs_guard.keys().cloned().collect()
                                            };
                                            
                                            let response = SecretResponse {
                                                success: true,
                                                env_vars: None,
                                                message: Some(format!("Environment '{}' saved successfully", env_name)),
                                                environments: Some(environments),
                                            };
                                            let out = serde_json::to_vec(&response).unwrap();
                                            let _ = socket.write_all(&out).await;
                                        },
                                        Err(e) => {
                                            println!("[vaultd] Failed to save environment '{}': {}", env_name, e);
                                            let environments = {
                                                let envs_guard = envs.lock().await;
                                                envs_guard.keys().cloned().collect()
                                            };
                                            let response = SecretResponse {
                                                success: false,
                                                env_vars: None,
                                                message: Some(format!("Failed to save environment '{}': {}", env_name, e)),
                                                environments: Some(environments),
                                            };
                                            let out = serde_json::to_vec(&response).unwrap();
                                            let _ = socket.write_all(&out).await;
                                        }
                                    }
                                } else {
                                    let environments = {
                                        let envs_guard = envs.lock().await;
                                        envs_guard.keys().cloned().collect()
                                    };
                                    let response = SecretResponse {
                                        success: false,
                                        env_vars: None,
                                        message: Some("No variables provided for save operation".to_string()),
                                        environments: Some(environments),
                                    };
                                    let out = serde_json::to_vec(&response).unwrap();
                                    let _ = socket.write_all(&out).await;
                                }
                                return;
                            }

                            // Determine which environment to use
                            let env_name = req.environment.as_deref().unwrap_or("dev");
                            let (env_vars_vec, environments) = {
                                let envs_guard = envs.lock().await;
                                let env_vars = envs_guard.get(env_name);
                                if env_vars.is_none() {
                                    let environments = envs_guard.keys().cloned().collect();
                                    drop(envs_guard);
                                    let response = SecretResponse {
                                        success: false,
                                        env_vars: None,
                                        message: Some(format!("Environment '{}' not found", env_name)),
                                        environments: Some(environments),
                                    };
                                    let out = serde_json::to_vec(&response).unwrap();
                                    let _ = socket.write_all(&out).await;
                                    return;
                                }
                                let env_vars_vec: Vec<(String, String)> = env_vars.unwrap().iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                                let environments = envs_guard.keys().cloned().collect();
                                (env_vars_vec, environments)
                            };
                            
                            // Special handling for shell activation
                            if req.command == "shell-activation" {
                                println!("[vaultd] Shell activation request - providing all variables for environment: {}", env_name);
                                let response = SecretResponse {
                                    success: true,
                                    env_vars: Some(env_vars_vec.clone()),
                                    message: None,
                                    environments: Some(environments),
                                };
                                let out = serde_json::to_vec(&response).unwrap();
                                let _ = socket.write_all(&out).await;
                                return;
                            }
                            
                            // Check if command is allowed and provide environment variables
                            if is_command_allowed(&req.command) {
                                let response = SecretResponse {
                                    success: true,
                                    env_vars: Some(env_vars_vec.clone()),
                                    message: None,
                                    environments: Some(environments),
                                };
                                
                                println!("[vaultd] Authorized - sending {} environment variables for environment '{}'", env_vars_vec.len(), env_name);
                                
                                let out = serde_json::to_vec(&response).unwrap();
                                let _ = socket.write_all(&out).await;
                            } else {
                                let response = SecretResponse {
                                    success: false,
                                    env_vars: None,
                                    message: Some(format!("Unauthorized command: {}", req.command)),
                                    environments: Some(environments),
                                };
                                
                                println!("[vaultd] Unauthorized command: {}", req.command);
                                
                                let out = serde_json::to_vec(&response).unwrap();
                                let _ = socket.write_all(&out).await;
                            }
                        },
                        Err(e) => {
                            eprintln!("[vaultd] Failed to parse request: {}", e);
                            
                            let response = SecretResponse {
                                success: false,
                                env_vars: None,
                                message: Some("Invalid request format".to_string()),
                                environments: None,
                            };
                            
                            let out = serde_json::to_vec(&response).unwrap();
                            let _ = socket.write_all(&out).await;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[vaultd] Error reading from socket: {}", e);
                }
            }
        });
    }
}

// In a real implementation, this would check against a list of allowed commands
fn is_command_allowed(command: &str) -> bool {
    // Special case for shell activation
    if command == "shell-activation" {
        return true;
    }
    
    // Allow echo commands for retrieving individual variables
    if command.starts_with("echo ") {
        return true;
    }
    
    // For now, allow any command that starts with "python3" or "node"
    command.starts_with("python3") || command.starts_with("node")
}

// In a real implementation, this would load from an encrypted file or a secure service
fn load_environment_variables() -> HashMap<String, HashMap<String, String>> {
    let mut envs: HashMap<String, HashMap<String, String>> = HashMap::new();
    if let Ok(config_content) = std::fs::read_to_string("secrets.toml") {
        println!("[vaultd] Loading environment variables from secrets.toml");
        if let Ok(toml_value) = config_content.parse::<Value>() {
            if let Some(table) = toml_value.as_table() {
                // First, collect root-level variables (not in sections)
                let mut root_vars = HashMap::new();
                for (key, value) in table.iter() {
                    if let Some(val) = value.as_str() {
                        root_vars.insert(key.clone(), val.to_string());
                    }
                }
                
                // Add root variables to a "global" environment if any exist
                if !root_vars.is_empty() {
                    envs.insert("global".to_string(), root_vars);
                }
                
                // Then process environment sections like [dev], [prod], etc.
                for (env_name, env_vars) in table.iter() {
                    if let Some(vars_table) = env_vars.as_table() {
                        let mut vars = HashMap::new();
                        for (k, v) in vars_table.iter() {
                            if let Some(val) = v.as_str() {
                                vars.insert(k.clone(), val.to_string());
                            }
                        }
                        envs.insert(env_name.clone(), vars);
                    }
                }
            }
        }
    }
    envs
}

async fn save_environment_variables(env_name: &str, variables: HashMap<String, String>) -> anyhow::Result<()> {
    // Read the current secrets.toml file
    let config_content = fs::read_to_string("secrets.toml")?;
    let mut toml_value: Value = config_content.parse()?;
    
    // Update the specific environment section
    if let Some(table) = toml_value.as_table_mut() {
        // Insert or update the environment section
        let mut env_table = toml::map::Map::new();
        for (key, value) in variables {
            env_table.insert(key, Value::String(value));
        }
        table.insert(env_name.to_string(), Value::Table(env_table));
    }
    
    // Write back to file
    let toml_content = toml::to_string(&toml_value)?;
    fs::write("secrets.toml", toml_content)?;
    
    println!("[vaultd] Updated secrets.toml with environment '{}'", env_name);
    Ok(())
}
