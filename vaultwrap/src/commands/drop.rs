use crate::config::{load_config, save_config};
use std::io::{Write, Read};
use std::net::TcpStream;
use std::collections::HashMap;
use std::env;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct SecretRequest {
    client_id: String,
    command: String,
    environment: Option<String>,
    variables: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize)]
struct SecretResponse {
    success: bool,
    env_vars: Option<Vec<(String, String)>>,
    message: Option<String>,
    environments: Option<Vec<String>>,
}

pub fn run(shell_output: bool) {
    let mut config = load_config();
    
    let env_name = match config.last_set_env.as_ref() {
        Some(env) => env.clone(),
        None => {
            eprintln!("No environment was previously set. Nothing to drop.");
            std::process::exit(1);
        }
    };

    let conn_name = match config.default.as_ref() {
        Some(name) => name,
        None => {
            eprintln!("No default connection set. Use 'vaultwrap connect' first.");
            std::process::exit(1);
        }
    };
    
    let conn = match config.connections.get(conn_name) {
        Some(conn) => conn,
        None => {
            eprintln!("Default connection '{}' not found in config.", conn_name);
            std::process::exit(1);
        }
    };

    let address = format!("{}:{}", conn.host, conn.port);

    // Connect to the server
    let mut stream = match TcpStream::connect(&address) {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("Failed to connect to VaultWrap server at {}: {}", address, e);
            std::process::exit(1);
        }
    };

    // Build request to get the environment variables
    let request = SecretRequest {
        client_id: "vaultwrap-cli".to_string(),
        command: "shell-activation".to_string(),
        environment: Some(env_name.clone()),
        variables: None,
    };

    let request_json = serde_json::to_vec(&request).unwrap();
    if let Err(e) = stream.write_all(&request_json) {
        eprintln!("Failed to send request: {}", e);
        std::process::exit(1);
    }

    let mut response_buf = vec![0; 4096];
    let n = match stream.read(&mut response_buf) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Failed to read response: {}", e);
            std::process::exit(1);
        }
    };

    let response: SecretResponse = match serde_json::from_slice(&response_buf[..n]) {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Failed to parse response: {}", e);
            std::process::exit(1);
        }
    };

    if response.success {
        if let Some(vars) = response.env_vars {
            if shell_output {
                // Output shell commands for eval
                println!("if [ -n \"$VAULTWRAP_OLD_PS1\" ]; then");
                println!("    export PS1=\"$VAULTWRAP_OLD_PS1\"");
                println!("    unset VAULTWRAP_OLD_PS1");
                println!("fi");
                
                for (key, _) in vars {
                    println!("unset {}", key);
                }
            } else {
                // Directly modify current process environment
                if let Ok(old_ps1) = env::var("VAULTWRAP_OLD_PS1") {
                    env::set_var("PS1", old_ps1);
                    env::remove_var("VAULTWRAP_OLD_PS1");
                }
                
                for (key, _) in vars {
                    env::remove_var(key);
                }
                
                println!("Environment '{}' deactivated.", env_name);
            }
            
            // Clear the last set environment from config
            config.last_set_env = None;
            save_config(&config);
        } else {
            eprintln!("No variables found in environment '{}'.", env_name);
        }
    } else {
        eprintln!("Error: {}", response.message.unwrap_or("Unknown error".to_string()));
        std::process::exit(1);
    }
} 