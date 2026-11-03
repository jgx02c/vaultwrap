//! Opening, creating and resetting the vault. Ties together the encrypted
//! store, the password-wrapped recovery file and the OS keychain.

use crate::crypto::{new_vault_id, Recovery, VaultKey};
use crate::store::Store;
use crate::{keychain, paths};
use anyhow::{bail, Context, Result};
use std::path::PathBuf;

/// An open vault and the key that opened it.
pub struct Unlocked {
    pub store: Store,
    pub key: VaultKey,
    pub vault_id: String,
}

pub fn exists() -> bool {
    paths::db_path().exists()
}

pub fn recovery() -> Result<Option<Recovery>> {
    Recovery::load(&paths::recovery_path())
}

pub fn vault_id() -> Option<String> {
    recovery().ok().flatten().map(|r| r.vault_id)
}

/// Whether this vault's key is currently stored in the keychain.
pub fn key_in_keychain() -> bool {
    vault_id().is_some_and(|id| keychain::get(&id).is_some())
}

fn save_to_keychain(id: &str, key: &VaultKey) -> Option<String> {
    match keychain::set(id, &key.to_hex()) {
        Ok(()) => None,
        Err(e) => Some(format!(
            "could not save the vault key to the {}: {e:#}",
            keychain::describe()
        )),
    }
}

/// Create a new vault. Returns the vault and an optional warning.
pub fn create(password: &str) -> Result<(Unlocked, Option<String>)> {
    if exists() {
        bail!("a vault already exists at {}", paths::db_path().display());
    }
    paths::ensure_home()?;
    let key = VaultKey::generate();
    let vault_id = new_vault_id();
    let rec = Recovery::seal(&key, password, &vault_id)?;
    rec.save(&paths::recovery_path())?;
    let store = Store::create(&paths::db_path(), &key)?;
    let warn = save_to_keychain(&vault_id, &key);
    Ok((
        Unlocked {
            store,
            key,
            vault_id,
        },
        warn,
    ))
}

/// Unlock using the key stored in the keychain. `None` if there is no key there.
pub fn open_with_keychain() -> Option<Result<Unlocked>> {
    let rec = recovery().ok()??;
    let hex = keychain::get(&rec.vault_id)?;
    Some((|| {
        let key = VaultKey::from_hex(&hex)?;
        let store = Store::open(&paths::db_path(), &key)
            .context("the key in the keychain does not open this vault")?;
        Ok(Unlocked {
            store,
            key,
            vault_id: rec.vault_id.clone(),
        })
    })())
}

/// Unlock with the master password, then (re)save the key to the keychain.
/// Also upgrades vaults from earlier builds that were keyed by the password.
pub fn open_with_password(password: &str) -> Result<(Unlocked, Option<String>)> {
    if !exists() {
        bail!(crate::store::NotFound("vault".into()));
    }
    let db = paths::db_path();
    let unlocked = match recovery()? {
        Some(rec) => {
            let key = rec.open(password)?;
            let store = match Store::open(&db, &key) {
                Ok(s) => s,
                // Interrupted upgrade: recovery written, database not yet rekeyed.
                Err(_) => {
                    let s = Store::open_legacy(&db, password)?;
                    s.rekey(&key)?;
                    s
                }
            };
            Unlocked {
                store,
                key,
                vault_id: rec.vault_id,
            }
        }
        None => {
            let store = Store::open_legacy(&db, password)?;
            let key = VaultKey::generate();
            let vault_id = new_vault_id();
            // Write the recovery file first so the new key is never lost.
            Recovery::seal(&key, password, &vault_id)?.save(&paths::recovery_path())?;
            store.rekey(&key)?;
            keychain::delete_legacy();
            Unlocked {
                store,
                key,
                vault_id,
            }
        }
    };
    let warn = save_to_keychain(&unlocked.vault_id, &unlocked.key);
    Ok((unlocked, warn))
}

/// Check a master password without changing anything.
pub fn verify_password(password: &str) -> Result<()> {
    match recovery()? {
        Some(rec) => rec.open(password).map(|_| ()),
        None => Store::open_legacy(&paths::db_path(), password).map(|_| ()),
    }
}

/// Set a new master password. Needs an unlocked vault, not the old password,
/// so a forgotten password can be reset from a keychain-unlocked session.
pub fn set_password(u: &Unlocked, new_password: &str) -> Result<()> {
    Recovery::seal(&u.key, new_password, &u.vault_id)?.save(&paths::recovery_path())
}

/// Remove this vault's key from the keychain. The master password will be
/// needed on the next unlock, which stores the key again.
pub fn forget_keychain() -> Result<()> {
    match vault_id() {
        Some(id) => keychain::delete(&id),
        None => Ok(()),
    }
}

/// Move the vault aside (never deleted) so a new one can be created.
/// Returns the backup directory.
pub fn erase() -> Result<PathBuf> {
    let id = vault_id();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup = paths::home().join("backups").join(format!("vault-{stamp}"));
    std::fs::create_dir_all(&backup)?;
    let db = paths::db_path();
    let mut moved = false;
    for p in [
        db.clone(),
        db.with_extension("db-wal"),
        db.with_extension("db-shm"),
        paths::recovery_path(),
    ] {
        if p.exists() {
            std::fs::rename(&p, backup.join(p.file_name().unwrap()))?;
            moved = true;
        }
    }
    if !moved {
        std::fs::remove_dir(&backup).ok();
        bail!("there is no vault to erase");
    }
    if let Some(id) = id {
        let _ = keychain::delete(&id);
    }
    Ok(backup)
}
