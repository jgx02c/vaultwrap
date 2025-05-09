use shared::*;
use std::env;
use std::process::Command;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "run" => {
            if args.len() < 3 {
                print_usage();
                return Ok(());
            }
            
            // Extract the command and arguments
            let command = &args[2];
            let command_args: Vec<String> = args.iter().skip(3).cloned().collect();
            let full_command = format!("{} {}", command, command_args.join(" "));
            
            // Run the command with injected environment variables
            run_command(command, &command_args, &full_command).await?;
        },
        "activate" => {
            // Generate activation script for the current shell
            activate().await?;
        },
        "deactivate" => {
            // Generate deactivation script for the current shell
            deactivate()?;
        },
        _ => {
            print_usage();
        }
    }

    Ok(())
}

async fn run_command(command: &str, command_args: &[String], full_command: &str) -> anyhow::Result<()> {
    println!("[vaultwrap] Requesting environment variables for: {}", full_command);

    // Request secrets from vaultd
    let secrets = request_secrets(full_command).await?;
    
    if secrets.success {
        println!("[vaultwrap] Launching command with injected environment variables...");
        
        // Create the command
        let mut cmd = Command::new(command);
        cmd.args(command_args);

        // Inject environment variables
        if let Some(envs) = secrets.env_vars {
            for (k, v) in envs {
                println!("[vaultwrap] Injecting: {}", k);
                cmd.env(k, v);
            }
        }

        // Run the command
        let status = cmd.status()?;
        println!("[vaultwrap] Process exited with: {}", status);
    } else {
        eprintln!("[vaultwrap] Error: {:?}", secrets.message.unwrap_or_else(|| "Unknown error".to_string()));
    }
    
    Ok(())
}

async fn activate() -> anyhow::Result<()> {
    println!("[vaultwrap] Requesting environment variables for shell activation");

    // Request all secrets from vaultd for shell activation
    let secrets = request_secrets("shell-activation").await?;
    
    if secrets.success {
        if let Some(envs) = secrets.env_vars {
            // Output environment variables for shell consumption
            for (k, v) in &envs {
                // Properly escape the value for shell
                let escaped_value = v.replace("'", "'\\''");
                println!("export {}='{}'", k, escaped_value);
            }
            
            // Echo a message to stderr so user knows it worked
            eprintln!("[vaultwrap] Successfully loaded {} environment variables", envs.len());
        } else {
            eprintln!("[vaultwrap] Warning: No environment variables received from server");
        }
    } else {
        eprintln!("[vaultwrap] Error: {:?}", secrets.message.unwrap_or_else(|| "Unknown error".to_string()));
        // Return non-zero status code to shell
        std::process::exit(1);
    }
    
    Ok(())
}

fn deactivate() -> anyhow::Result<()> {
    // This function is only called when the user runs `vaultwrap deactivate` directly
    // instead of using the shell `deactivate` function
    
    println!("# This should be sourced, not executed directly");
    println!("# Please use:");
    println!("# source <(vaultwrap deactivate)");
    println!("");
    println!("if [ -n \"$VAULTWRAP_ACTIVE\" ]; then");
    println!("  unset VAULTWRAP_ACTIVE");
    println!("  export PS1=\"$VAULTWRAP_ORIGINAL_PS1\"");
    println!("  unset VAULTWRAP_ORIGINAL_PS1");
    println!("  unset -f vaultwrap_deactivate");
    println!("  echo \"[vaultwrap] Environment deactivated\"");
    println!("else");
    println!("  echo \"[vaultwrap] No active environment to deactivate\"");
    println!("fi");
    println!("unalias deactivate 2>/dev/null || true");
    
    Ok(())
}

async fn request_secrets(command: &str) -> anyhow::Result<SecretResponse> {
    // Create the request
    let req = SecretRequest {
        client_id: get_client_id(),
        command: command.to_string(),
    };

    // Connect to vaultd
    let mut stream = TcpStream::connect("127.0.0.1:4000").await?;
    
    // Send the request
    let req_bytes = serde_json::to_vec(&req)?;
    stream.write_all(&req_bytes).await?;

    // Read the response
    let mut buf = vec![0; 2048];
    let n = stream.read(&mut buf).await?;
    let res: SecretResponse = serde_json::from_slice(&buf[..n])?;
    
    Ok(res)
}

fn get_client_id() -> String {
    // Get machine-specific identifier
    match hostname::get() {
        Ok(name) => name.to_string_lossy().to_string(),
        Err(_) => "unknown-client".to_string(),
    }
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  vaultwrap run <command> [args...]     # Run a command with injected env vars");
    eprintln!("  vaultwrap activate                   # Activate env vars in current shell");
    eprintln!("  vaultwrap deactivate                 # Deactivate env vars in current shell");
    eprintln!("");
    eprintln!("Examples:");
    eprintln!("  vaultwrap run python3 app.py");
    eprintln!("  source <(vaultwrap activate)     # Activate in current shell");
    eprintln!("  deactivate                      # Deactivate in current shell");
}
