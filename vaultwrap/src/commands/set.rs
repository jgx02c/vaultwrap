use crate::config::load_config;
use std::io::{Write, Read};
use std::net::TcpStream;
use std::collections::HashMap;
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

pub fn run(env: String) {
    let config = load_config();
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

    // Build request
    let request = SecretRequest {
        client_id: "vaultwrap-cli".to_string(),
        command: "shell-activation".to_string(),
        environment: Some(env.clone()),
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
            for (key, value) in vars {
                println!("export {}='{}'", key, value.replace('\'', "'\\''"));
            }
        }
    } else {
        eprintln!("Error: {}", response.message.unwrap_or("Unknown error".to_string()));
        std::process::exit(1);
    }
} 