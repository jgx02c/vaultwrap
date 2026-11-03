use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// An `org/project/env` triple.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Target {
    pub org: String,
    pub project: String,
    pub env: String,
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/{}", self.org, self.project, self.env)
    }
}

impl FromStr for Target {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 3 || parts.iter().any(|p| p.is_empty()) {
            bail!("expected org/project/env, got '{s}'");
        }
        Ok(Target {
            org: parts[0].into(),
            project: parts[1].into(),
            env: parts[2].into(),
        })
    }
}

pub fn validate_name(kind: &str, name: &str) -> Result<()> {
    let ok = !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !ok {
        bail!("invalid {kind} name '{name}' (use letters, digits, '-', '_', '.')");
    }
    Ok(())
}

pub fn validate_env_key(key: &str) -> Result<()> {
    let mut chars = key.chars();
    let ok = matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !ok {
        bail!("invalid variable name '{key}'");
    }
    Ok(())
}

/// Variable name/value pairs, in order.
pub type Vars = Vec<(String, String)>;

/// A resolved environment variable and where it came from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedVar {
    pub key: String,
    pub value: String,
    /// Provider name, or "local" for values stored directly in the vault.
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolved {
    pub target: Target,
    pub protected: bool,
    pub vars: Vec<ResolvedVar>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Provider kinds understood by the daemon.
pub const PROVIDER_KINDS: &[&str] = &["azure-keyvault", "dotenv", "command"];

/// Keys in provider config whose values are never sent back to clients.
pub fn redact_config(cfg: &serde_json::Value) -> serde_json::Value {
    let mut cfg = cfg.clone();
    if let Some(obj) = cfg.as_object_mut() {
        for (k, v) in obj.iter_mut() {
            let k = k.to_ascii_lowercase();
            if k.contains("secret") || k.contains("password") || k.contains("token") {
                *v = serde_json::Value::String("********".into());
            }
        }
    }
    cfg
}
