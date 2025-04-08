use shared::*;
use std::env;
use std::process::Command;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: vaultwrap run <command>");
        return Ok(());
    }

    let command_to_run = args.join(" ");
    let req = SecretRequest {
        client_id: "intern1".to_string(),
        command: command_to_run.clone(),
    };

    let mut stream = TcpStream::connect("127.0.0.1:4000").await?;
    let req_bytes = serde_json::to_vec(&req)?;
    stream.write_all(&req_bytes).await?;

    let mut buf = vec![0; 2048];
    let n = stream.read(&mut buf).await?;
    let res: SecretResponse = serde_json::from_slice(&buf[..n])?;

    if res.success {
        println!("[vaultwrap] Launching command with secrets...");
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command_to_run);

        if let Some(envs) = res.env_vars {
            for (k, v) in envs {
                cmd.env(k, v);
            }
        }

        let status = cmd.status()?;
        println!("[vaultwrap] Process exited with: {}", status);
    } else {
        eprintln!("Error: {:?}", res.message);
    }

    Ok(())
}
