//! Install the daemon as a per-user background service that starts at login:
//! launchd on macOS, systemd --user on Linux.

use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::process::Command;

#[cfg(target_os = "macos")]
const LABEL: &str = "dev.vaultwrap.daemon";

fn exe() -> Result<PathBuf> {
    let exe = std::env::current_exe()?.canonicalize()?;
    let s = exe.display().to_string();
    if s.contains("/target/debug/") || s.contains("/target/release/") {
        eprintln!(
            "warning: installing the service from a build directory ({s}).\n\
             Install the binary first so the path stays stable: `cargo install --path crates/vaultwrap`"
        );
    }
    Ok(exe)
}

/// Env the daemon needs: PATH so `az`, `op`, `doppler`... resolve, plus our overrides.
fn service_env() -> Vec<(String, String)> {
    let mut env = vec![(
        "PATH".to_string(),
        std::env::var("PATH").unwrap_or_default(),
    )];
    for k in ["VAULTWRAP_HOME", "VAULTWRAP_SOCKET", "AZURE_CONFIG_DIR"] {
        if let Ok(v) = std::env::var(k) {
            env.push((k.into(), v));
        }
    }
    env
}

fn run(cmd: &mut Command) -> Result<bool> {
    Ok(cmd
        .status()
        .with_context(|| format!("running {cmd:?}"))?
        .success())
}

#[cfg(target_os = "macos")]
fn plist_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home)
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist")))
}

#[cfg(target_os = "macos")]
fn domain() -> String {
    format!("gui/{}", unsafe { libc::getuid() })
}

#[cfg(target_os = "macos")]
pub fn install(idle_lock: u64) -> Result<()> {
    fn esc(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
    let exe = exe()?;
    let log = vaultwrap_core::paths::log_path();
    vaultwrap_core::paths::ensure_home()?;
    let env: String = service_env()
        .iter()
        .map(|(k, v)| format!("    <key>{}</key><string>{}</string>\n", esc(k), esc(v)))
        .collect();
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
    <string>daemon</string>
    <string>--takeover</string>
    <string>--idle-lock</string>
    <string>{idle_lock}</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>ProcessType</key><string>Background</string>
  <key>EnvironmentVariables</key>
  <dict>
{env}  </dict>
  <key>StandardOutPath</key><string>{log}</string>
  <key>StandardErrorPath</key><string>{log}</string>
</dict>
</plist>
"#,
        exe = esc(&exe.display().to_string()),
        log = esc(&log.display().to_string()),
    );
    let path = plist_path()?;
    std::fs::create_dir_all(path.parent().unwrap())?;
    // Replace any existing registration.
    let _ = Command::new("launchctl")
        .args(["bootout", &format!("{}/{LABEL}", domain())])
        .output();
    std::fs::write(&path, plist)?;
    if !run(Command::new("launchctl").args(["bootstrap", &domain(), &path.display().to_string()]))?
    {
        bail!("launchctl bootstrap failed");
    }
    println!("Installed launchd agent {LABEL}");
    println!("  plist: {}", path.display());
    println!("  log:   {}", log.display());
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn uninstall() -> Result<()> {
    let _ = Command::new("launchctl")
        .args(["bootout", &format!("{}/{LABEL}", domain())])
        .output();
    let path = plist_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    println!("Removed launchd agent {LABEL}");
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn installed() -> Result<Option<String>> {
    let path = plist_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let loaded = Command::new("launchctl")
        .args(["print", &format!("{}/{LABEL}", domain())])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    Ok(Some(format!(
        "launchd ({})",
        if loaded { "loaded" } else { "not loaded" }
    )))
}

#[cfg(target_os = "linux")]
fn unit_path() -> Result<PathBuf> {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
        .context("HOME not set")?;
    Ok(base.join("systemd/user/vaultwrap.service"))
}

#[cfg(target_os = "linux")]
pub fn install(idle_lock: u64) -> Result<()> {
    let exe = exe()?;
    let env: String = service_env()
        .iter()
        .map(|(k, v)| format!("Environment=\"{}={}\"\n", k, v.replace('"', "\\\"")))
        .collect();
    let unit = format!(
        "[Unit]\nDescription=vaultwrap secrets daemon\n\n[Service]\nExecStart=\"{}\" daemon --takeover --idle-lock {idle_lock}\n{env}Restart=always\n\n[Install]\nWantedBy=default.target\n",
        exe.display()
    );
    let path = unit_path()?;
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, unit)?;
    run(Command::new("systemctl").args(["--user", "daemon-reload"]))?;
    if !run(Command::new("systemctl").args(["--user", "enable", "--now", "vaultwrap.service"]))? {
        bail!("systemctl enable failed");
    }
    println!("Installed systemd user service: {}", path.display());
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn uninstall() -> Result<()> {
    let _ =
        run(Command::new("systemctl").args(["--user", "disable", "--now", "vaultwrap.service"]));
    let path = unit_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    let _ = run(Command::new("systemctl").args(["--user", "daemon-reload"]));
    println!("Removed systemd user service");
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn installed() -> Result<Option<String>> {
    Ok(unit_path()?.exists().then(|| "systemd --user".to_string()))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn install(_: u64) -> Result<()> {
    bail!("service install is only supported on macOS and Linux")
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn uninstall() -> Result<()> {
    bail!("service install is only supported on macOS and Linux")
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn installed() -> Result<Option<String>> {
    Ok(None)
}
