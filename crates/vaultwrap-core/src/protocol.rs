//! Newline-delimited JSON over a Unix socket (mode 0600, same-uid peers only).
//! One request per line, one response per line.

use crate::model::Target;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    Ping,
    Status,
    /// Create the vault. Its key is saved to the keychain.
    Init {
        password: String,
    },
    /// Unlock with the master password (and save the key to the keychain).
    Unlock {
        password: String,
    },
    /// Unlock with the key in the keychain (after an explicit lock).
    UnlockKeychain,
    /// Check the master password without changing state.
    VerifyPassword {
        password: String,
    },
    /// Lock until the user unlocks again.
    Lock,
    /// Remove the vault key from the keychain.
    Forget,
    /// Set a new master password (vault must be unlocked; old one not needed).
    SetPassword {
        new_password: String,
    },
    /// Move the vault to a backup folder so a new one can be created.
    EraseVault,
    Tree,

    OrgAdd {
        name: String,
    },
    OrgRemove {
        name: String,
    },

    ProviderAdd {
        org: String,
        name: String,
        kind: String,
        config: Value,
        #[serde(default)]
        replace: bool,
    },
    ProviderRemove {
        org: String,
        name: String,
    },
    /// Fetch from a provider and return the variable names it yields.
    ProviderTest {
        org: String,
        name: String,
        #[serde(default)]
        options: Value,
        target: Option<Target>,
    },

    ProjectAdd {
        org: String,
        name: String,
    },
    ProjectRemove {
        org: String,
        name: String,
    },

    EnvAdd {
        target: Target,
        #[serde(default)]
        protected: bool,
    },
    EnvRemove {
        target: Target,
    },
    EnvProtect {
        target: Target,
        protected: bool,
    },
    EnvClone {
        from: Target,
        to: Target,
    },

    SourceAdd {
        target: Target,
        provider: String,
        #[serde(default)]
        options: Value,
    },
    SourceRemove {
        target: Target,
        provider: String,
    },

    SecretSet {
        target: Target,
        vars: Vec<(String, String)>,
    },
    SecretUnset {
        target: Target,
        keys: Vec<String>,
    },

    /// Merge all sources + local values for an environment.
    Resolve {
        target: Target,
        #[serde(default)]
        refresh: bool,
    },
    /// Drop the in-memory cache of remote values.
    Refresh,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Locked,
    NoVault,
    WrongPassword,
    NotFound,
    Invalid,
    Provider,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub data: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<ErrorCode>,
}

impl Response {
    pub fn ok(data: impl Serialize) -> Self {
        Response {
            ok: true,
            data: serde_json::to_value(data).unwrap_or(Value::Null),
            error: None,
            code: None,
        }
    }
    pub fn err(code: ErrorCode, msg: impl Into<String>) -> Self {
        Response {
            ok: false,
            data: Value::Null,
            error: Some(msg.into()),
            code: Some(code),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub version: String,
    pub pid: u32,
    pub home: String,
    pub vault_exists: bool,
    pub unlocked: bool,
    /// Locked on purpose; the keychain won't silently unlock it.
    pub user_locked: bool,
    /// This vault's key is stored in the keychain.
    pub key_in_keychain: bool,
    /// Where keys are stored, e.g. "macOS Keychain".
    pub keychain: String,
    pub vault_id: Option<String>,
    pub idle_lock_minutes: u64,
}
