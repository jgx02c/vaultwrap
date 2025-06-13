use crate::config::{load_config, save_config};

pub fn enable() {
    let mut config = load_config();
    
    if config.runtime_injection.commands.is_empty() {
        println!("No commands configured for runtime injection.");
        println!("Add commands first using: vaultwrap add <command>");
        println!("Example: vaultwrap add cargo");
        return;
    }
    
    config.runtime_injection.enabled = true;
    save_config(&config);
    
    println!("Runtime injection enabled for commands: {}", 
             config.runtime_injection.commands.join(", "));
    println!("When these commands are run, environment variables will be automatically injected.");
}

pub fn disable() {
    let mut config = load_config();
    
    config.runtime_injection.enabled = false;
    save_config(&config);
    
    println!("Runtime injection disabled.");
}

pub fn add_command(command: String) {
    let mut config = load_config();
    
    if config.runtime_injection.commands.contains(&command) {
        println!("Command '{}' is already in the runtime injection list.", command);
        return;
    }
    
    config.runtime_injection.commands.push(command.clone());
    save_config(&config);
    
    println!("Added '{}' to runtime injection commands.", command);
    println!("Current commands: {}", config.runtime_injection.commands.join(", "));
    
    if !config.runtime_injection.enabled {
        println!("Note: Runtime injection is currently disabled. Use 'vaultwrap enable' to activate it.");
    }
}

pub fn remove_command(command: String) {
    let mut config = load_config();
    
    if let Some(pos) = config.runtime_injection.commands.iter().position(|x| x == &command) {
        config.runtime_injection.commands.remove(pos);
        save_config(&config);
        
        println!("Removed '{}' from runtime injection commands.", command);
        
        if config.runtime_injection.commands.is_empty() {
            println!("No commands left in runtime injection list.");
            if config.runtime_injection.enabled {
                config.runtime_injection.enabled = false;
                save_config(&config);
                println!("Runtime injection automatically disabled.");
            }
        } else {
            println!("Remaining commands: {}", config.runtime_injection.commands.join(", "));
        }
    } else {
        println!("Command '{}' is not in the runtime injection list.", command);
    }
}

pub fn status() {
    let config = load_config();
    
    if config.runtime_injection.enabled {
        println!("Runtime injection: ENABLED");
        if config.runtime_injection.commands.is_empty() {
            println!("Intercepted commands: None (this shouldn't happen)");
        } else {
            println!("Intercepted commands: {}", config.runtime_injection.commands.join(", "));
        }
        
        if let Some(env) = &config.last_set_env {
            println!("Current environment: {}", env);
        } else {
            println!("Current environment: None");
        }
    } else {
        println!("Runtime injection: DISABLED");
        if !config.runtime_injection.commands.is_empty() {
            println!("Configured commands: {}", config.runtime_injection.commands.join(", "));
        } else {
            println!("No commands configured for runtime injection.");
        }
    }
    
    if let Some(default_conn) = &config.default {
        println!("Default connection: {}", default_conn);
    } else {
        println!("Default connection: None");
    }
} 