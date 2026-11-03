use std::path::PathBuf;

/// Root directory for vault data. Override with `VAULTWRAP_HOME`.
/// macOS: ~/Library/Application Support/vaultwrap, Linux: ~/.local/share/vaultwrap
pub fn home() -> PathBuf {
    if let Some(h) = std::env::var_os("VAULTWRAP_HOME") {
        return PathBuf::from(h);
    }
    directories::BaseDirs::new()
        .map(|b| b.data_dir().join("vaultwrap"))
        .unwrap_or_else(|| PathBuf::from(".vaultwrap"))
}

pub fn ensure_home() -> std::io::Result<PathBuf> {
    let h = home();
    std::fs::create_dir_all(&h)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&h, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(h)
}

pub fn db_path() -> PathBuf {
    home().join("vault.db")
}

/// Vault key wrapped with the master password (no secrets in the clear).
pub fn recovery_path() -> PathBuf {
    home().join("vault.recovery")
}

pub fn socket_path() -> PathBuf {
    if let Some(s) = std::env::var_os("VAULTWRAP_SOCKET") {
        return PathBuf::from(s);
    }
    let p = home().join("vaultd.sock");
    // Unix socket paths are limited to ~104 bytes. For long homes, use a
    // short per-user temp path keyed by the home directory instead.
    if p.as_os_str().len() < 100 {
        return p;
    }
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in home().as_os_str().as_encoded_bytes() {
        hash = (hash ^ *b as u64).wrapping_mul(0x100000001b3);
    }
    // Fixed location (not $TMPDIR, which differs under launchd), private to this user.
    #[cfg(unix)]
    let dir = PathBuf::from(format!("/tmp/vaultwrap-{}", unsafe { libc::getuid() }));
    #[cfg(not(unix))]
    let dir = std::env::temp_dir().join("vaultwrap");
    if std::fs::create_dir_all(&dir).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
            // Refuse a directory someone else planted.
            if std::fs::metadata(&dir)
                .map(|m| m.uid() != unsafe { libc::getuid() })
                .unwrap_or(true)
            {
                return p;
            }
        }
    }
    dir.join(format!("{hash:x}.sock"))
}

pub fn log_path() -> PathBuf {
    home().join("vaultd.log")
}

/// Expand a leading `~/`.
pub fn expand_tilde(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(b) = directories::BaseDirs::new() {
            return b.home_dir().join(rest);
        }
    }
    PathBuf::from(p)
}
