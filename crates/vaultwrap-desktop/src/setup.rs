//! Machine setup the app performs for the user: putting the `vaultwrap` CLI
//! on PATH, the daemon's login service, and opening the app at login.

use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const CLI_LINK: &str = "/usr/local/bin/vaultwrap";
const APP_AGENT: &str = "dev.vaultwrap.app";

/// The CLI binary shipped next to this executable (Contents/MacOS/vaultwrap in
/// the bundle, target/<profile>/vaultwrap in development).
pub fn bundled_cli() -> Result<PathBuf> {
    let exe = std::env::current_exe()?.canonicalize()?;
    let dir = exe.parent().context("no exe dir")?;
    let cli = dir.join("vaultwrap");
    if cli.is_file() {
        return Ok(cli);
    }
    bail!(
        "vaultwrap CLI not found next to {} — run `make build`",
        exe.display()
    )
}

pub fn in_app_bundle() -> bool {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().contains(".app/Contents/MacOS/"))
        .unwrap_or(false)
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default())
}

fn launch_agents() -> PathBuf {
    home().join("Library/LaunchAgents")
}

#[derive(Serialize)]
pub struct SetupInfo {
    pub version: String,
    pub in_bundle: bool,
    pub cli_path: Option<String>,
    /// `vaultwrap` on PATH resolves to this app's CLI.
    pub cli_linked: bool,
    /// Something else named vaultwrap is at the link location.
    pub cli_other: Option<String>,
    pub service_installed: bool,
    /// The service runs this app's CLI (not an older/dev copy).
    pub service_current: bool,
    pub open_at_login: bool,
    pub shell: String,
    /// Running against the mock-data demo vault (`make demo`).
    pub demo: bool,
}

pub fn info() -> SetupInfo {
    let cli = bundled_cli().ok();
    let link_target = std::fs::canonicalize(CLI_LINK).ok();
    let cli_linked = matches!((&cli, &link_target), (Some(c), Some(t)) if c == t);
    let plist = launch_agents().join("dev.vaultwrap.daemon.plist");
    let plist_text = std::fs::read_to_string(&plist).unwrap_or_default();
    let service_current = cli
        .as_ref()
        .is_some_and(|c| plist_text.contains(&*c.to_string_lossy()));
    SetupInfo {
        version: vaultwrap_core::VERSION.into(),
        in_bundle: in_app_bundle(),
        cli_path: cli.as_ref().map(|p| p.display().to_string()),
        cli_linked,
        cli_other: if cli_linked {
            None
        } else {
            link_target.map(|p| p.display().to_string())
        },
        service_installed: !plist_text.is_empty(),
        service_current,
        open_at_login: launch_agents().join(format!("{APP_AGENT}.plist")).exists(),
        shell: login_shell(),
        demo: std::env::var_os("VAULTWRAP_DEMO").is_some(),
    }
}

pub fn login_shell() -> String {
    let sh = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    Path::new(&sh)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "zsh".into())
}

/// Symlink /usr/local/bin/vaultwrap to the bundled CLI (asks for an admin password).
pub fn install_cli() -> Result<()> {
    let cli = bundled_cli()?;
    let sh = format!(
        "mkdir -p /usr/local/bin && ln -sf '{}' '{}'",
        cli.display().to_string().replace('\'', "'\\''"),
        CLI_LINK
    );
    let script = format!(
        "do shell script \"{}\" with prompt \"Vaultwrap wants to install the vaultwrap command line tool.\" with administrator privileges",
        sh.replace('\\', "\\\\").replace('"', "\\\"")
    );
    let out = Command::new("osascript").args(["-e", &script]).output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        if err.contains("-128") {
            bail!("cancelled");
        }
        bail!("could not install the command line tool: {}", err.trim());
    }
    Ok(())
}

/// (Re)install the daemon's login service using the bundled CLI.
pub fn install_service() -> Result<String> {
    let cli = bundled_cli()?;
    let out = Command::new(&cli).args(["service", "install"]).output()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if !out.status.success() {
        bail!("{}", text.trim());
    }
    Ok(text)
}

/// Open the menu-bar app at login (in the background, no window).
pub fn set_open_at_login(enabled: bool) -> Result<()> {
    let path = launch_agents().join(format!("{APP_AGENT}.plist"));
    if !enabled {
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        return Ok(());
    }
    let exe = std::env::current_exe()?.canonicalize()?;
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{APP_AGENT}</string>
  <key>ProgramArguments</key>
  <array><string>{}</string><string>--background</string></array>
  <key>RunAtLoad</key><true/>
  <key>ProcessType</key><string>Interactive</string>
</dict>
</plist>
"#,
        esc(&exe.display().to_string())
    );
    std::fs::create_dir_all(launch_agents())?;
    std::fs::write(&path, plist)?;
    Ok(())
}

/// First launch of the installed app: make sure the daemon runs at login from
/// this bundle, and that the app itself opens at login.
pub fn first_run() {
    if !in_app_bundle() || std::env::var_os("VAULTWRAP_SKIP_SETUP").is_some() {
        return;
    }
    let st = info();
    if !st.service_current {
        if let Err(e) = install_service() {
            eprintln!("[vaultwrap-desktop] service install failed: {e:#}");
        }
    }
    let marker = vaultwrap_core::paths::home().join(".desktop-initialized");
    if !marker.exists() {
        let _ = set_open_at_login(true);
        let _ = vaultwrap_core::paths::ensure_home().and_then(|_| std::fs::write(&marker, b"1"));
    }
}
