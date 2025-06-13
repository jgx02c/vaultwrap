use crate::config::load_config;
use crate::EnvSub;
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

pub fn run(sub: EnvSub) {
    match sub {
        EnvSub::List => list_environments(),
        EnvSub::Add { env } => add_environment(env),
        EnvSub::Delete { env } => delete_environment(env),
        EnvSub::Keys { env } => list_keys(env),
    }
}

fn get_connection() -> (String, u16) {
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

    (conn.host.clone(), conn.port)
}

fn send_request(request: SecretRequest) -> SecretResponse {
    let (host, port) = get_connection();
    let address = format!("{}:{}", host, port);

    let mut stream = match TcpStream::connect(&address) {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("Failed to connect to VaultWrap server at {}: {}", address, e);
            std::process::exit(1);
        }
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

    match serde_json::from_slice(&response_buf[..n]) {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Failed to parse response: {}", e);
            std::process::exit(1);
        }
    }
}

fn list_environments() {
    let request = SecretRequest {
        client_id: "vaultwrap-cli".to_string(),
        command: "list-environments".to_string(),
        environment: None,
        variables: None,
    };

    let response = send_request(request);

    if response.success {
        if let Some(envs) = response.environments {
            println!("Environments:");
            for env in envs {
                println!("  {}", env);
            }
        } else {
            println!("No environments found.");
        }
    } else {
        eprintln!("Error: {}", response.message.unwrap_or("Unknown error".to_string()));
        std::process::exit(1);
    }
}

fn add_environment(env: String) {
    let request = SecretRequest {
        client_id: "vaultwrap-cli".to_string(),
        command: "create-environment".to_string(),
        environment: Some(env.clone()),
        variables: None,
    };

    let response = send_request(request);

    if response.success {
        println!("Environment '{}' created successfully.", env);
    } else {
        eprintln!("Error: {}", response.message.unwrap_or("Unknown error".to_string()));
        std::process::exit(1);
    }
}

fn delete_environment(env: String) {
    let request = SecretRequest {
        client_id: "vaultwrap-cli".to_string(),
        command: "delete-environment".to_string(),
        environment: Some(env.clone()),
        variables: None,
    };

    let response = send_request(request);

    if response.success {
        println!("Environment '{}' deleted successfully.", env);
    } else {
        eprintln!("Error: {}", response.message.unwrap_or("Unknown error".to_string()));
        std::process::exit(1);
    }
}

fn list_keys(env: String) {
    let request = SecretRequest {
        client_id: "vaultwrap-cli".to_string(),
        command: "shell-activation".to_string(),
        environment: Some(env.clone()),
        variables: None,
    };

    let response = send_request(request);

    if response.success {
        if let Some(vars) = response.env_vars {
            println!("Keys in environment '{}':", env);
            for (key, value) in vars {
                println!("  {} = {}", key, value);
            }
        } else {
            println!("No keys found in environment '{}'.", env);
        }
    } else {
        eprintln!("Error: {}", response.message.unwrap_or("Unknown error".to_string()));
        std::process::exit(1);
    }
} 