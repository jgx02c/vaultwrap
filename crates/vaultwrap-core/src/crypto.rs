//! Vault key handling.
//!
//! The vault database is encrypted with a random 256-bit key. That key is kept
//! in two places:
//! - the OS keychain, so the vault unlocks with your computer login, and
//! - a recovery file (`vault.recovery`), wrapped with a key derived from your
//!   master password (Argon2id + XChaCha20-Poly1305).
//!
//! The master password never encrypts the database directly, so it can be
//! changed or reset without re-encrypting anything.

use crate::store::WrongPassword;
use anyhow::{bail, Context, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::path::Path;
use zeroize::{Zeroize, Zeroizing};

pub const MIN_PASSWORD_LEN: usize = 8;

/// The key that encrypts the vault database.
#[derive(Clone)]
pub struct VaultKey([u8; 32]);

impl VaultKey {
    pub fn generate() -> Self {
        let mut k = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut k);
        VaultKey(k)
    }

    pub fn to_hex(&self) -> Zeroizing<String> {
        Zeroizing::new(hex::encode(self.0))
    }

    pub fn from_hex(s: &str) -> Result<Self> {
        let mut bytes =
            Zeroizing::new(hex::decode(s.trim()).context("vault key is not valid hex")?);
        if bytes.len() != 32 {
            bail!("vault key has the wrong length");
        }
        let mut k = [0u8; 32];
        k.copy_from_slice(&bytes);
        bytes.zeroize();
        Ok(VaultKey(k))
    }

    /// SQLCipher raw-key form: `x'<64 hex>'`.
    pub fn sqlcipher_key(&self) -> Zeroizing<String> {
        Zeroizing::new(format!("x'{}'", hex::encode(self.0)))
    }
}

impl Drop for VaultKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

pub fn new_vault_id() -> String {
    let mut b = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut b);
    hex::encode(b)
}

/// On-disk recovery record. Contains no secrets in the clear.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recovery {
    pub version: u32,
    /// Random ID; namespaces the keychain entry for this vault.
    pub vault_id: String,
    pub kdf: String,
    pub m_cost_kib: u32,
    pub t_cost: u32,
    pub p_cost: u32,
    pub salt: String,
    pub nonce: String,
    pub wrapped_key: String,
}

const M_COST_KIB: u32 = 64 * 1024;
const T_COST: u32 = 3;
const P_COST: u32 = 1;

fn derive(password: &str, salt: &[u8], m: u32, t: u32, p: u32) -> Result<Zeroizing<[u8; 32]>> {
    let params =
        Params::new(m, t, p, Some(32)).map_err(|e| anyhow::anyhow!("argon2 params: {e}"))?;
    let mut out = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, out.as_mut())
        .map_err(|e| anyhow::anyhow!("argon2: {e}"))?;
    Ok(out)
}

impl Recovery {
    /// Wrap `key` with `password`.
    pub fn seal(key: &VaultKey, password: &str, vault_id: &str) -> Result<Self> {
        if password.chars().count() < MIN_PASSWORD_LEN {
            bail!("master password must be at least {MIN_PASSWORD_LEN} characters");
        }
        let mut salt = [0u8; 16];
        let mut nonce = [0u8; 24];
        rand::rngs::OsRng.fill_bytes(&mut salt);
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let kek = derive(password, &salt, M_COST_KIB, T_COST, P_COST)?;
        let cipher = XChaCha20Poly1305::new(kek.as_ref().into());
        let wrapped = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &key.0,
                    aad: vault_id.as_bytes(),
                },
            )
            .map_err(|_| anyhow::anyhow!("encrypting vault key failed"))?;
        Ok(Recovery {
            version: 1,
            vault_id: vault_id.to_string(),
            kdf: "argon2id".into(),
            m_cost_kib: M_COST_KIB,
            t_cost: T_COST,
            p_cost: P_COST,
            salt: hex::encode(salt),
            nonce: hex::encode(nonce),
            wrapped_key: hex::encode(wrapped),
        })
    }

    /// Recover the vault key, or [`WrongPassword`].
    pub fn open(&self, password: &str) -> Result<VaultKey> {
        if self.kdf != "argon2id" {
            bail!("unsupported key derivation '{}'", self.kdf);
        }
        let salt = hex::decode(&self.salt)?;
        let nonce = hex::decode(&self.nonce)?;
        let wrapped = hex::decode(&self.wrapped_key)?;
        if nonce.len() != 24 {
            bail!("corrupt recovery file");
        }
        let kek = derive(password, &salt, self.m_cost_kib, self.t_cost, self.p_cost)?;
        let cipher = XChaCha20Poly1305::new(kek.as_ref().into());
        let plain = Zeroizing::new(
            cipher
                .decrypt(
                    XNonce::from_slice(&nonce),
                    Payload {
                        msg: &wrapped,
                        aad: self.vault_id.as_bytes(),
                    },
                )
                .map_err(|_| WrongPassword)?,
        );
        if plain.len() != 32 {
            bail!("corrupt recovery file");
        }
        let mut k = [0u8; 32];
        k.copy_from_slice(&plain);
        Ok(VaultKey(k))
    }

    pub fn load(path: &Path) -> Result<Option<Self>> {
        match std::fs::read_to_string(path) {
            Ok(s) => Ok(Some(
                serde_json::from_str(&s).with_context(|| format!("parsing {}", path.display()))?,
            )),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Atomically write with mode 0600.
    pub fn save(&self, path: &Path) -> Result<()> {
        let tmp = path.with_extension("recovery.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_and_open() {
        let key = VaultKey::generate();
        let id = new_vault_id();
        let r = Recovery::seal(&key, "correct horse", &id).unwrap();
        assert_eq!(r.open("correct horse").unwrap().to_hex(), key.to_hex());
        let err = r.open("wrong horse").err().unwrap();
        assert!(err.downcast_ref::<WrongPassword>().is_some());
        // Tampering with the vault id (AAD) must fail too.
        let mut t = r.clone();
        t.vault_id = new_vault_id();
        assert!(t.open("correct horse").is_err());
        assert!(Recovery::seal(&key, "short", &id).is_err());
        assert_eq!(
            VaultKey::from_hex(&key.to_hex()).unwrap().to_hex(),
            key.to_hex()
        );
    }
}
