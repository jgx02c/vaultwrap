use shared::*;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;
use toml::Value;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:4000").await?;
    println!("[vaultd] Vault server listening on port 4000...");
    println!("[vaultd] Ready to serve environment variables on demand");

    // Load our environment variables
    // In a real implementation, this would be loaded from an encrypted file
    let envs = load_environment_variables();

    loop {
        let (mut socket, addr) = listener.accept().await?;
        let envs = envs.clone();

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
                                let response = SecretResponse {
                                    success: true,
                                    env_vars: None,
                                    message: None,
                                    environments: Some(envs.keys().cloned().collect()),
                                };
                                let out = serde_json::to_vec(&response).unwrap();
                                let _ = socket.write_all(&out).await;
                                return;
                            }

                            // Determine which environment to use
                            let env_name = req.environment.as_deref().unwrap_or("dev");
                            let env_vars = envs.get(env_name);
                            if env_vars.is_none() {
                                let response = SecretResponse {
                                    success: false,
                                    env_vars: None,
                                    message: Some(format!("Environment '{}' not found", env_name)),
                                    environments: Some(envs.keys().cloned().collect()),
                                };
                                let out = serde_json::to_vec(&response).unwrap();
                                let _ = socket.write_all(&out).await;
                                return;
                            }
                            let env_vars_vec = env_vars.unwrap().iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                            
                            // Special handling for shell activation
                            if req.command == "shell-activation" {
                                println!("[vaultd] Shell activation request - providing all variables for environment: {}", env_name);
                                let response = SecretResponse {
                                    success: true,
                                    env_vars: Some(env_vars_vec.clone()),
                                    message: None,
                                    environments: Some(envs.keys().cloned().collect()),
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
                                    environments: Some(envs.keys().cloned().collect()),
                                };
                                
                                println!("[vaultd] Authorized - sending {} environment variables for environment '{}'", env_vars_vec.len(), env_name);
                                
                                let out = serde_json::to_vec(&response).unwrap();
                                let _ = socket.write_all(&out).await;
                            } else {
                                let response = SecretResponse {
                                    success: false,
                                    env_vars: None,
                                    message: Some(format!("Unauthorized command: {}", req.command)),
                                    environments: Some(envs.keys().cloned().collect()),
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
