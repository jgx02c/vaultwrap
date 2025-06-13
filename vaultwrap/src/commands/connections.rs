use crate::config::{Config, Connection, load_config, save_config};
use crate::ConnectionsSub;

pub fn connect(
    host: String,
    username: Option<String>,
    password: Option<String>,
    save: Option<String>,
    default: bool,
) {
    // Parse host:port
    let (host, port) = if let Some((h, p)) = host.split_once(':') {
        (h.to_string(), p.parse::<u16>().unwrap_or(4000))
    } else {
        (host, 4000)
    };

    let mut config = load_config();

    let conn = Connection {
        host,
        port,
        username,
        password,
    };

    let name = save.unwrap_or_else(|| "default".to_string());
    config.connections.insert(name.clone(), conn);

    if default || config.default.is_none() {
        config.default = Some(name.clone());
    }

    save_config(&config);
    println!("Connection '{}' saved{}.", name, if default { " and set as default" } else { "" });
}

pub fn run(sub: ConnectionsSub) {
    match sub {
        ConnectionsSub::List => {
            let config = load_config();
            if config.connections.is_empty() {
                println!("No saved connections.");
                return;
            }
            println!("Saved connections:");
            for (name, conn) in config.connections.iter() {
                let def = if Some(name) == config.default.as_ref() { " (default)" } else { "" };
                println!("  {}: {}:{}{}", name, conn.host, conn.port, def);
            }
        }
        ConnectionsSub::Use { name } => {
            let mut config = load_config();
            if config.connections.contains_key(&name) {
                config.default = Some(name.clone());
                save_config(&config);
                println!("Default connection set to '{}'", name);
            } else {
                eprintln!("No such connection: '{}'", name);
                std::process::exit(1);
            }
        }
        ConnectionsSub::Remove { name } => {
            let mut config = load_config();
            if config.connections.remove(&name).is_some() {
                if config.default.as_ref() == Some(&name) {
                    config.default = config.connections.keys().next().cloned();
                }
                save_config(&config);
                println!("Connection '{}' removed.", name);
            } else {
                eprintln!("No such connection: '{}'", name);
                std::process::exit(1);
            }
        }
        ConnectionsSub::Show => {
            let config = load_config();
            if let Some(def) = &config.default {
                if let Some(conn) = config.connections.get(def) {
                    println!("Current connection ({}): {}:{}",
                        def, conn.host, conn.port);
                } else {
                    println!("Default connection '{}' not found.", def);
                }
            } else {
                println!("No default connection set.");
            }
        }
    }
} 