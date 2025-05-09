use shared::*;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:4000").await?;
    println!("[vaultd] Vault server listening on port 4000...");
    println!("[vaultd] Ready to serve environment variables on demand");

    // Load our environment variables
    // In a real implementation, this would be loaded from an encrypted file
    let env_vars = load_environment_variables();

    loop {
        let (mut socket, addr) = listener.accept().await?;
        let env_vars = env_vars.clone();

        tokio::spawn(async move {
            println!("[vaultd] New connection from: {}", addr);
            
            let mut buf = vec![0; 1024];
            match socket.read(&mut buf).await {
                Ok(n) if n == 0 => return,
                Ok(n) => {
                    match serde_json::from_slice::<SecretRequest>(&buf[..n]) {
                        Ok(req) => {
                            println!("[vaultd] Request from client '{}' to run: {}", req.client_id, req.command);
                            
                            // Special handling for shell activation
                            if req.command == "shell-activation" {
                                println!("[vaultd] Shell activation request - providing all variables");
                                
                                let response = SecretResponse {
                                    success: true,
                                    env_vars: Some(env_vars.clone()),
                                    message: None,
                                };
                                
                                let out = serde_json::to_vec(&response).unwrap();
                                let _ = socket.write_all(&out).await;
                                return;
                            }
                            
                            // Check if command is allowed and provide environment variables
                            if is_command_allowed(&req.command) {
                                let response = SecretResponse {
                                    success: true,
                                    env_vars: Some(env_vars.clone()),
                                    message: None,
                                };
                                
                                println!("[vaultd] Authorized - sending {} environment variables", env_vars.len());
                                
                                let out = serde_json::to_vec(&response).unwrap();
                                let _ = socket.write_all(&out).await;
                            } else {
                                let response = SecretResponse {
                                    success: false,
                                    env_vars: None,
                                    message: Some(format!("Unauthorized command: {}", req.command)),
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
fn load_environment_variables() -> Vec<(String, String)> {
    // Try to load from secrets.toml file
    if let Ok(config_content) = std::fs::read_to_string("secrets.toml") {
        println!("[vaultd] Loading environment variables from secrets.toml");
        
        // Simple parsing of TOML-like format
        let mut env_vars = Vec::new();
        
        for line in config_content.lines() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Parse key = "value" format
            if let Some(pos) = line.find('=') {
                let key = line[..pos].trim().to_string();
                let mut value = line[pos+1..].trim().to_string();
                
                // Remove quotes if present
                if (value.starts_with('"') && value.ends_with('"')) || 
                   (value.starts_with('\'') && value.ends_with('\'')) {
                    value = value[1..value.len()-1].to_string();
                }
                
                println!("[vaultd] Loaded env var: {}", key);
                env_vars.push((key, value));
            }
        }
        
        if !env_vars.is_empty() {
            return env_vars;
        }
    }
    
    // Fall back to hardcoded values if file not found or empty
    println!("[vaultd] Using default environment variables (no secrets.toml found)");
    vec![
        ("DATABASE_URL".to_string(), "postgres://user:password@localhost:5432/mydb".to_string()),
        ("API_KEY".to_string(), "sk_test_12345abcdef".to_string()),
        ("SECRET_KEY".to_string(), "very_secret_key_do_not_share".to_string()),
        ("REDIS_URL".to_string(), "redis://localhost:6379".to_string()),
        ("ENVIRONMENT".to_string(), "development".to_string()),
    ]
}
