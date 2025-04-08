use shared::*;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:4000").await?;
    println!("[vaultd] Listening on port 4000...");

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = vec![0; 1024];
            match socket.read(&mut buf).await {
                Ok(n) if n == 0 => return,
                Ok(n) => {
                    let req: SecretRequest = serde_json::from_slice(&buf[..n]).unwrap();
                    println!("[vaultd] Received request from: {}", req.client_id);

                    // TEMP: simulate secrets
                    if req.command == "python3 app.py" {
                        let response = SecretResponse {
                            success: true,
                            env_vars: Some(vec![
                                ("DATABASE_URL".to_string(), "postgres://secret".to_string()),
                                ("API_KEY".to_string(), "sk_test_123".to_string()),
                            ]),
                            message: None,
                        };
                        let out = serde_json::to_vec(&response).unwrap();
                        let _ = socket.write_all(&out).await;
                    } else {
                        let response = SecretResponse {
                            success: false,
                            env_vars: None,
                            message: Some("Unauthorized command".to_string()),
                        };
                        let out = serde_json::to_vec(&response).unwrap();
                        let _ = socket.write_all(&out).await;
                    }
                }
                Err(e) => {
                    eprintln!("[vaultd] Error: {}", e);
                }
            }
        });
    }
}
