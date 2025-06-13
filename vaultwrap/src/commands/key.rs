use crate::config::load_config;
use crate::KeySub;
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

pub fn run(sub: KeySub) {
    match sub {
        KeySub::Add { env, key, value } => add_key(env, key, value),
        KeySub::Delete { env, key } => delete_key(env, key),
        KeySub::Update { env, old_key, new_key, new_value } => update_key(env, old_key, new_key, new_value),
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

fn fetch_env_vars(env: &str) -> HashMap<String, String> {
    let request = SecretRequest {
        client_id: "vaultwrap-cli".to_string(),
        command: "shell-activation".to_string(),
        environment: Some(env.to_string()),
        variables: None,
    };

    let response = send_request(request);

    if response.success {
        response.env_vars
            .unwrap_or_default()
            .into_iter()
            .collect()
    } else {
        eprintln!("Error fetching environment '{}': {}", env, response.message.unwrap_or("Unknown error".to_string()));
        std::process::exit(1);
    }
}

fn save_env_vars(env: &str, vars: HashMap<String, String>, success_msg: &str) {
    let request = SecretRequest {
        client_id: "vaultwrap-cli".to_string(),
        command: "save-environment".to_string(),
        environment: Some(env.to_string()),
        variables: Some(vars),
    };

    let response = send_request(request);

    if response.success {
        println!("{}", success_msg);
    } else {
        eprintln!("Error: {}", response.message.unwrap_or("Unknown error".to_string()));
        std::process::exit(1);
    }
}

fn add_key(env: String, key: String, value: String) {
    let mut vars = fetch_env_vars(&env);
    vars.insert(key.clone(), value.clone());
    save_env_vars(&env, vars, &format!("Key '{}' added to environment '{}'.", key, env));
}

fn delete_key(env: String, key: String) {
    let mut vars = fetch_env_vars(&env);
    if vars.remove(&key).is_some() {
        save_env_vars(&env, vars, &format!("Key '{}' deleted from environment '{}'.", key, env));
    } else {
        eprintln!("Key '{}' not found in environment '{}'.", key, env);
        std::process::exit(1);
    }
}

fn update_key(env: String, old_key: String, new_key: String, new_value: String) {
    let mut vars = fetch_env_vars(&env);
    if vars.remove(&old_key).is_some() {
        vars.insert(new_key.clone(), new_value.clone());
        save_env_vars(&env, vars, &format!("Key '{}' updated to '{}' = '{}' in environment '{}'.", old_key, new_key, new_value, env));
    } else {
        eprintln!("Key '{}' not found in environment '{}'.", old_key, env);
        std::process::exit(1);
    }
} 