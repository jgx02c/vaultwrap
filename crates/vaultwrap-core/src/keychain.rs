//! Where the vault key lives so the vault unlocks with your computer login.
//!
//! Entries are namespaced as service `dev.vaultwrap`, account
//! `vault-key:<vault id>`, so several vaults never collide and the entry is
//! easy to find in Keychain Access.
//!
//! Backends:
//! - default: the OS keychain (macOS Keychain, Windows Credential Manager,
//!   Secret Service on Linux)
//! - `VAULTWRAP_KEYCHAIN_FILE=<path>`: a plain JSON file, for development,
//!   tests and the demo only
//! - `VAULTWRAP_NO_KEYCHAIN=1`: disabled; the master password is needed after
//!   every daemon start

use anyhow::Result;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const SERVICE: &str = "dev.vaultwrap";

pub fn account(vault_id: &str) -> String {
    format!("vault-key:{vault_id}")
}

enum Backend {
    Os,
    File(PathBuf),
    Disabled,
}

fn backend() -> Backend {
    if let Some(p) = std::env::var_os("VAULTWRAP_KEYCHAIN_FILE") {
        return Backend::File(PathBuf::from(p));
    }
    if std::env::var_os("VAULTWRAP_NO_KEYCHAIN").is_some() {
        return Backend::Disabled;
    }
    Backend::Os
}

/// Human-readable description of where keys are stored.
pub fn describe() -> String {
    match backend() {
        Backend::Os if cfg!(target_os = "macos") => "macOS Keychain".into(),
        Backend::Os => "system keychain".into(),
        Backend::File(p) => format!("dev keychain file {}", p.display()),
        Backend::Disabled => "disabled".into(),
    }
}

pub fn enabled() -> bool {
    !matches!(backend(), Backend::Disabled)
}

fn read_file(p: &PathBuf) -> BTreeMap<String, String> {
    std::fs::read_to_string(p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_file(p: &PathBuf, m: &BTreeMap<String, String>) -> Result<()> {
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(p, serde_json::to_string_pretty(m)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn os_entry(vault_id: &str) -> Result<keyring::Entry> {
    Ok(keyring::Entry::new(SERVICE, &account(vault_id))?)
}

pub fn get(vault_id: &str) -> Option<String> {
    match backend() {
        Backend::Os => os_entry(vault_id).ok()?.get_password().ok(),
        Backend::File(p) => read_file(&p).get(&account(vault_id)).cloned(),
        Backend::Disabled => None,
    }
}

pub fn set(vault_id: &str, secret: &str) -> Result<()> {
    match backend() {
        Backend::Os => Ok(os_entry(vault_id)?.set_password(secret)?),
        Backend::File(p) => {
            let mut m = read_file(&p);
            m.insert(account(vault_id), secret.to_string());
            write_file(&p, &m)
        }
        Backend::Disabled => Ok(()),
    }
}

pub fn delete(vault_id: &str) -> Result<()> {
    match backend() {
        Backend::Os => match os_entry(vault_id)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        },
        Backend::File(p) => {
            let mut m = read_file(&p);
            if m.remove(&account(vault_id)).is_some() {
                write_file(&p, &m)?;
            }
            Ok(())
        }
        Backend::Disabled => Ok(()),
    }
}

/// Remove the entry the v0.2.0-dev builds used (service `vaultwrap`).
pub fn delete_legacy() {
    if let Backend::Os = backend() {
        let user = format!("master-password:{}", crate::paths::db_path().display());
        if let Ok(e) = keyring::Entry::new("vaultwrap", &user) {
            let _ = e.delete_credential();
        }
    }
}
