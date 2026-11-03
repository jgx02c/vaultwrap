//! Client side of the daemon socket. Used by the CLI and any GUI.

use crate::protocol::{Request, Response};
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

pub struct Client {
    reader: BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: tokio::net::unix::OwnedWriteHalf,
}

impl Client {
    pub async fn connect() -> Result<Self> {
        let path = crate::paths::socket_path();
        if path.parent().is_some_and(|p| !p.exists()) {
            crate::paths::ensure_home()?;
        }
        let stream = UnixStream::connect(&path)
            .await
            .with_context(|| format!("connecting to vaultwrap daemon at {}", path.display()))?;
        let (r, w) = stream.into_split();
        Ok(Client {
            reader: BufReader::new(r),
            writer: w,
        })
    }

    /// Connect, starting the daemon in the background (`<exe> daemon`) if needed.
    pub async fn connect_or_spawn(exe: &Path) -> Result<Self> {
        if let Ok(c) = Self::connect().await {
            return Ok(c);
        }
        spawn_daemon(exe)?;
        for _ in 0..50 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if let Ok(c) = Self::connect().await {
                return Ok(c);
            }
        }
        bail!(
            "started the daemon but could not connect to it; see {}",
            crate::paths::log_path().display()
        )
    }

    pub async fn call(&mut self, req: &Request) -> Result<Response> {
        let mut line = serde_json::to_vec(req)?;
        line.push(b'\n');
        self.writer.write_all(&line).await?;
        let mut buf = String::new();
        let n = self.reader.read_line(&mut buf).await?;
        if n == 0 {
            bail!("daemon closed the connection");
        }
        serde_json::from_str(&buf).context("invalid response from daemon")
    }
}

fn spawn_daemon(exe: &Path) -> Result<()> {
    use std::process::{Command, Stdio};
    crate::paths::ensure_home()?;
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true).append(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut opts, 0o600);
    let log = opts.open(crate::paths::log_path())?;
    let mut cmd = Command::new(exe);
    cmd.arg("daemon")
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Detach from the terminal's session so Ctrl-C / closing it doesn't kill the daemon.
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }
    cmd.spawn()
        .with_context(|| format!("starting daemon via {}", exe.display()))?;
    Ok(())
}
