//! Confirming the computer login password, used to reset a forgotten master
//! password: whoever can unlock this Mac's Keychain already holds the vault key.

use anyhow::{bail, Result};

/// Show the native macOS password dialog and succeed only if the user
/// authenticates. Requires an administrator account.
pub fn confirm_computer_password(reason: &str) -> Result<()> {
    // Debug builds only: lets automated tests skip the dialog.
    #[cfg(debug_assertions)]
    if std::env::var_os("VAULTWRAP_TEST_SKIP_MAC_AUTH").is_some() {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "do shell script \"/usr/bin/true\" with prompt \"{}\" with administrator privileges",
            reason.replace('\\', "\\\\").replace('"', "\\\"")
        );
        let out = std::process::Command::new("/usr/bin/osascript")
            .args(["-e", &script])
            .output()?;
        if out.status.success() {
            return Ok(());
        }
        let err = String::from_utf8_lossy(&out.stderr);
        if err.contains("-128") {
            bail!("cancelled");
        }
        bail!("your Mac password could not be verified (an administrator account is required)");
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = reason;
        bail!("resetting with the computer password is only supported on macOS")
    }
}
