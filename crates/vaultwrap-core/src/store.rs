//! Encrypted vault backed by SQLCipher (AES-256). The whole database file
//! (names, values, provider credentials, cached remote values) is encrypted
//! with a random 256-bit vault key; see [`crate::crypto`] for how that key is
//! stored (OS keychain + password-wrapped recovery file).

use crate::crypto::VaultKey;
use crate::model::{validate_env_key, validate_name, Target};
use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct WrongPassword;
impl std::fmt::Display for WrongPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("wrong master password")
    }
}
impl std::error::Error for WrongPassword {}

#[derive(Debug)]
pub struct NotFound(pub String);
impl std::fmt::Display for NotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} not found", self.0)
    }
}
impl std::error::Error for NotFound {}

const SCHEMA_VERSION: i64 = 1;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS orgs (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS providers (
    id INTEGER PRIMARY KEY,
    org_id INTEGER NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    config TEXT NOT NULL,
    UNIQUE(org_id, name)
);
CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY,
    org_id INTEGER NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    UNIQUE(org_id, name)
);
CREATE TABLE IF NOT EXISTS environments (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    protected INTEGER NOT NULL DEFAULT 0,
    UNIQUE(project_id, name)
);
CREATE TABLE IF NOT EXISTS sources (
    id INTEGER PRIMARY KEY,
    env_id INTEGER NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    provider_id INTEGER NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    options TEXT NOT NULL DEFAULT '{}'
);
CREATE TABLE IF NOT EXISTS secrets (
    env_id INTEGER NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (env_id, key)
);
CREATE TABLE IF NOT EXISTS source_cache (
    source_id INTEGER PRIMARY KEY REFERENCES sources(id) ON DELETE CASCADE,
    fetched_at INTEGER NOT NULL,
    data TEXT NOT NULL
);
"#;

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRow {
    pub id: i64,
    pub org: String,
    pub name: String,
    pub kind: String,
    pub config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRow {
    pub id: i64,
    pub position: i64,
    pub provider: ProviderRow,
    pub options: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvNode {
    pub name: String,
    pub protected: bool,
    pub sources: Vec<SourceSummary>,
    /// Names of locally stored variables (values are never included).
    pub local_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSummary {
    pub provider: String,
    pub kind: String,
    pub options: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectNode {
    pub name: String,
    pub envs: Vec<EnvNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderNode {
    pub name: String,
    pub kind: String,
    /// Config with credentials redacted.
    pub config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgNode {
    pub name: String,
    pub providers: Vec<ProviderNode>,
    pub projects: Vec<ProjectNode>,
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn exists(path: &Path) -> bool {
        path.exists()
    }

    /// Create a new vault encrypted with `key`. Fails if one already exists.
    pub fn create(path: &Path, key: &VaultKey) -> Result<Self> {
        if path.exists() {
            bail!("a vault already exists at {}", path.display());
        }
        let store = Self::open_inner(path, &key.sqlcipher_key())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(store)
    }

    /// Open an existing vault, returning [`WrongPassword`] if the key is wrong.
    pub fn open(path: &Path, key: &VaultKey) -> Result<Self> {
        if !path.exists() {
            bail!(NotFound(format!("vault at {}", path.display())));
        }
        Self::open_inner(path, &key.sqlcipher_key())
    }

    /// Open a vault from v0.2.0-dev, which was keyed directly by the password.
    pub fn open_legacy(path: &Path, password: &str) -> Result<Self> {
        if !path.exists() {
            bail!(NotFound(format!("vault at {}", path.display())));
        }
        Self::open_inner(path, password)
    }

    /// Re-encrypt the whole database under a new key.
    pub fn rekey(&self, key: &VaultKey) -> Result<()> {
        // WAL must be off for rekey to rewrite every page.
        self.conn.pragma_update(None, "journal_mode", "DELETE")?;
        self.conn
            .pragma_update(None, "rekey", &*key.sqlcipher_key())?;
        self.conn.pragma_update(None, "journal_mode", "WAL")?;
        Ok(())
    }

    fn open_inner(path: &Path, key: &str) -> Result<Self> {
        let conn = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        conn.pragma_update(None, "key", key)?;
        // Any read fails with "file is not a database" when the key is wrong.
        if conn
            .query_row("SELECT count(*) FROM sqlite_master", [], |r| {
                r.get::<_, i64>(0)
            })
            .is_err()
        {
            return Err(WrongPassword.into());
        }
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let store = Store { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(SCHEMA)?;
        self.conn.execute(
            "INSERT OR IGNORE INTO meta(key, value) VALUES ('schema_version', ?1)",
            params![SCHEMA_VERSION.to_string()],
        )?;
        Ok(())
    }

    // ---------- lookups ----------

    fn org_id(&self, org: &str) -> Result<i64> {
        self.conn
            .query_row("SELECT id FROM orgs WHERE name = ?1", params![org], |r| {
                r.get(0)
            })
            .optional()?
            .ok_or_else(|| NotFound(format!("org '{org}'")).into())
    }

    fn project_id(&self, org: &str, project: &str) -> Result<i64> {
        let org_id = self.org_id(org)?;
        self.conn
            .query_row(
                "SELECT id FROM projects WHERE org_id = ?1 AND name = ?2",
                params![org_id, project],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| NotFound(format!("project '{org}/{project}'")).into())
    }

    fn env_id(&self, t: &Target) -> Result<i64> {
        let pid = self.project_id(&t.org, &t.project)?;
        self.conn
            .query_row(
                "SELECT id FROM environments WHERE project_id = ?1 AND name = ?2",
                params![pid, t.env],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| NotFound(format!("environment '{t}'")).into())
    }

    // ---------- orgs ----------

    pub fn org_add(&self, name: &str) -> Result<()> {
        validate_name("org", name)?;
        self.conn
            .execute(
                "INSERT INTO orgs(name, created_at) VALUES (?1, ?2)",
                params![name, now()],
            )
            .map_err(|e| unique_err(e, format!("org '{name}' already exists")))?;
        Ok(())
    }

    pub fn org_remove(&self, name: &str) -> Result<()> {
        let id = self.org_id(name)?;
        self.conn
            .execute("DELETE FROM orgs WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ---------- providers ----------

    pub fn provider_add(
        &self,
        org: &str,
        name: &str,
        kind: &str,
        config: &Value,
        replace: bool,
    ) -> Result<()> {
        validate_name("provider", name)?;
        if !crate::model::PROVIDER_KINDS.contains(&kind) {
            bail!(
                "unknown provider kind '{kind}' (expected one of: {})",
                crate::model::PROVIDER_KINDS.join(", ")
            );
        }
        let org_id = self.org_id(org)?;
        let cfg = serde_json::to_string(config)?;
        if replace {
            let updated = self.conn.execute(
                "UPDATE providers SET kind = ?3, config = ?4 WHERE org_id = ?1 AND name = ?2",
                params![org_id, name, kind, cfg],
            )?;
            if updated > 0 {
                return Ok(());
            }
        }
        self.conn
            .execute(
                "INSERT INTO providers(org_id, name, kind, config) VALUES (?1, ?2, ?3, ?4)",
                params![org_id, name, kind, cfg],
            )
            .map_err(|e| {
                unique_err(
                    e,
                    format!("provider '{org}/{name}' already exists (use --replace)"),
                )
            })?;
        Ok(())
    }

    pub fn provider_remove(&self, org: &str, name: &str) -> Result<()> {
        let org_id = self.org_id(org)?;
        let n = self.conn.execute(
            "DELETE FROM providers WHERE org_id = ?1 AND name = ?2",
            params![org_id, name],
        )?;
        if n == 0 {
            bail!(NotFound(format!("provider '{org}/{name}'")));
        }
        Ok(())
    }

    pub fn provider(&self, org: &str, name: &str) -> Result<ProviderRow> {
        let org_id = self.org_id(org)?;
        self.conn
            .query_row(
                "SELECT id, name, kind, config FROM providers WHERE org_id = ?1 AND name = ?2",
                params![org_id, name],
                |r| {
                    Ok(ProviderRow {
                        id: r.get(0)?,
                        org: org.to_string(),
                        name: r.get(1)?,
                        kind: r.get(2)?,
                        config: parse_json(r.get::<_, String>(3)?),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| NotFound(format!("provider '{org}/{name}'")).into())
    }

    // ---------- projects / environments ----------

    pub fn project_add(&self, org: &str, name: &str) -> Result<()> {
        validate_name("project", name)?;
        let org_id = self.org_id(org)?;
        self.conn
            .execute(
                "INSERT INTO projects(org_id, name) VALUES (?1, ?2)",
                params![org_id, name],
            )
            .map_err(|e| unique_err(e, format!("project '{org}/{name}' already exists")))?;
        Ok(())
    }

    pub fn project_remove(&self, org: &str, name: &str) -> Result<()> {
        let id = self.project_id(org, name)?;
        self.conn
            .execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn env_add(&self, t: &Target, protected: bool) -> Result<()> {
        validate_name("environment", &t.env)?;
        let pid = self.project_id(&t.org, &t.project)?;
        self.conn
            .execute(
                "INSERT INTO environments(project_id, name, protected) VALUES (?1, ?2, ?3)",
                params![pid, t.env, protected as i64],
            )
            .map_err(|e| unique_err(e, format!("environment '{t}' already exists")))?;
        Ok(())
    }

    pub fn env_set_protected(&self, t: &Target, protected: bool) -> Result<()> {
        let id = self.env_id(t)?;
        self.conn.execute(
            "UPDATE environments SET protected = ?2 WHERE id = ?1",
            params![id, protected as i64],
        )?;
        Ok(())
    }

    pub fn env_protected(&self, t: &Target) -> Result<bool> {
        let id = self.env_id(t)?;
        Ok(self.conn.query_row(
            "SELECT protected FROM environments WHERE id = ?1",
            params![id],
            |r| r.get::<_, i64>(0),
        )? != 0)
    }

    pub fn env_remove(&self, t: &Target) -> Result<()> {
        let id = self.env_id(t)?;
        self.conn
            .execute("DELETE FROM environments WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Copy local secrets and sources from one environment into a new one.
    pub fn env_clone(&self, from: &Target, to: &Target) -> Result<()> {
        if from.org != to.org {
            bail!("can only clone environments within the same org");
        }
        let protected = self.env_protected(from)?;
        if self.project_id(&to.org, &to.project).is_err() {
            self.project_add(&to.org, &to.project)?;
        }
        self.env_add(to, protected)?;
        let src = self.env_id(from)?;
        let dst = self.env_id(to)?;
        self.conn.execute(
            "INSERT INTO sources(env_id, provider_id, position, options)
             SELECT ?2, provider_id, position, options FROM sources WHERE env_id = ?1",
            params![src, dst],
        )?;
        self.conn.execute(
            "INSERT INTO secrets(env_id, key, value, updated_at)
             SELECT ?2, key, value, ?3 FROM secrets WHERE env_id = ?1",
            params![src, dst, now()],
        )?;
        Ok(())
    }

    // ---------- sources ----------

    pub fn source_add(&self, t: &Target, provider: &str, options: &Value) -> Result<()> {
        let env_id = self.env_id(t)?;
        let p = self.provider(&t.org, provider)?;
        let pos: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(position), 0) + 1 FROM sources WHERE env_id = ?1",
            params![env_id],
            |r| r.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO sources(env_id, provider_id, position, options) VALUES (?1, ?2, ?3, ?4)",
            params![env_id, p.id, pos, serde_json::to_string(options)?],
        )?;
        Ok(())
    }

    pub fn source_remove(&self, t: &Target, provider: &str) -> Result<()> {
        let env_id = self.env_id(t)?;
        let p = self.provider(&t.org, provider)?;
        let n = self.conn.execute(
            "DELETE FROM sources WHERE env_id = ?1 AND provider_id = ?2",
            params![env_id, p.id],
        )?;
        if n == 0 {
            bail!(NotFound(format!("source '{provider}' on {t}")));
        }
        Ok(())
    }

    /// Sources for an environment, lowest priority first.
    pub fn sources(&self, t: &Target) -> Result<Vec<SourceRow>> {
        let env_id = self.env_id(t)?;
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.position, s.options, p.id, p.name, p.kind, p.config
             FROM sources s JOIN providers p ON p.id = s.provider_id
             WHERE s.env_id = ?1 ORDER BY s.position",
        )?;
        let rows = stmt
            .query_map(params![env_id], |r| {
                Ok(SourceRow {
                    id: r.get(0)?,
                    position: r.get(1)?,
                    options: parse_json(r.get::<_, String>(2)?),
                    provider: ProviderRow {
                        id: r.get(3)?,
                        org: t.org.clone(),
                        name: r.get(4)?,
                        kind: r.get(5)?,
                        config: parse_json(r.get::<_, String>(6)?),
                    },
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    // ---------- local secrets ----------

    pub fn secret_set(&self, t: &Target, key: &str, value: &str) -> Result<()> {
        validate_env_key(key)?;
        let env_id = self.env_id(t)?;
        self.conn.execute(
            "INSERT INTO secrets(env_id, key, value, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(env_id, key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![env_id, key, value, now()],
        )?;
        Ok(())
    }

    pub fn secret_unset(&self, t: &Target, key: &str) -> Result<()> {
        let env_id = self.env_id(t)?;
        let n = self.conn.execute(
            "DELETE FROM secrets WHERE env_id = ?1 AND key = ?2",
            params![env_id, key],
        )?;
        if n == 0 {
            bail!(NotFound(format!("local variable '{key}' in {t}")));
        }
        Ok(())
    }

    pub fn secrets(&self, t: &Target) -> Result<Vec<(String, String)>> {
        let env_id = self.env_id(t)?;
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM secrets WHERE env_id = ?1 ORDER BY key")?;
        let rows = stmt
            .query_map(params![env_id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    // ---------- remote value cache (for offline use) ----------

    pub fn cache_put(&self, source_id: i64, vars: &[(String, String)]) -> Result<()> {
        self.conn.execute(
            "INSERT INTO source_cache(source_id, fetched_at, data) VALUES (?1, ?2, ?3)
             ON CONFLICT(source_id) DO UPDATE SET fetched_at = excluded.fetched_at, data = excluded.data",
            params![source_id, now(), serde_json::to_string(vars)?],
        )?;
        Ok(())
    }

    /// Returns (unix fetched_at, vars).
    pub fn cache_get(&self, source_id: i64) -> Result<Option<(i64, crate::model::Vars)>> {
        let row: Option<(i64, String)> = self
            .conn
            .query_row(
                "SELECT fetched_at, data FROM source_cache WHERE source_id = ?1",
                params![source_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        Ok(row.map(|(ts, data)| (ts, serde_json::from_str(&data).unwrap_or_default())))
    }

    // ---------- overview ----------

    pub fn tree(&self) -> Result<Vec<OrgNode>> {
        let mut out = Vec::new();
        let mut orgs = self
            .conn
            .prepare("SELECT id, name FROM orgs ORDER BY name")?;
        let orgs: Vec<(i64, String)> = orgs
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (org_id, org) in orgs {
            let mut ps = self.conn.prepare(
                "SELECT name, kind, config FROM providers WHERE org_id = ?1 ORDER BY name",
            )?;
            let providers = ps
                .query_map(params![org_id], |r| {
                    Ok(ProviderNode {
                        name: r.get(0)?,
                        kind: r.get(1)?,
                        config: crate::model::redact_config(&parse_json(r.get::<_, String>(2)?)),
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;

            let mut pj = self
                .conn
                .prepare("SELECT name FROM projects WHERE org_id = ?1 ORDER BY name")?;
            let project_names: Vec<String> = pj
                .query_map(params![org_id], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            let mut projects = Vec::new();
            for project in project_names {
                let pid = self.project_id(&org, &project)?;
                let mut es = self.conn.prepare(
                    "SELECT name, protected FROM environments WHERE project_id = ?1 ORDER BY name",
                )?;
                let envs_raw: Vec<(String, i64)> = es
                    .query_map(params![pid], |r| Ok((r.get(0)?, r.get(1)?)))?
                    .collect::<rusqlite::Result<_>>()?;
                let mut envs = Vec::new();
                for (env, protected) in envs_raw {
                    let t = Target {
                        org: org.clone(),
                        project: project.clone(),
                        env: env.clone(),
                    };
                    let sources = self
                        .sources(&t)?
                        .into_iter()
                        .map(|s| SourceSummary {
                            provider: s.provider.name,
                            kind: s.provider.kind,
                            options: s.options,
                        })
                        .collect();
                    let local_keys = self.secrets(&t)?.into_iter().map(|(k, _)| k).collect();
                    envs.push(EnvNode {
                        name: env,
                        protected: protected != 0,
                        sources,
                        local_keys,
                    });
                }
                projects.push(ProjectNode {
                    name: project,
                    envs,
                });
            }
            out.push(OrgNode {
                name: org,
                providers,
                projects,
            });
        }
        Ok(out)
    }
}

fn parse_json(s: String) -> Value {
    serde_json::from_str(&s).unwrap_or(Value::Object(Default::default()))
}

fn unique_err(e: rusqlite::Error, msg: String) -> anyhow::Error {
    match &e {
        rusqlite::Error::SqliteFailure(f, _)
            if f.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            anyhow!(msg)
        }
        _ => e.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn target() -> Target {
        "chally/api/staging".parse().unwrap()
    }

    #[test]
    fn roundtrip_and_encryption() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v.db");
        let key = VaultKey::generate();
        {
            let s = Store::create(&path, &key).unwrap();
            s.org_add("chally").unwrap();
            s.project_add("chally", "api").unwrap();
            s.env_add(&target(), false).unwrap();
            s.secret_set(&target(), "DATABASE_URL", "postgres://super-secret-value")
                .unwrap();
        }
        // Plaintext must not appear on disk.
        let raw = std::fs::read(&path).unwrap();
        let needle = b"super-secret-value";
        assert!(!raw.windows(needle.len()).any(|w| w == needle));
        assert!(!raw.starts_with(b"SQLite format 3"));

        let err = Store::open(&path, &VaultKey::generate()).err().unwrap();
        assert!(err.downcast_ref::<WrongPassword>().is_some());

        let s = Store::open(&path, &key).unwrap();
        assert_eq!(
            s.secrets(&target()).unwrap(),
            vec![(
                "DATABASE_URL".into(),
                "postgres://super-secret-value".into()
            )]
        );
        let key2 = VaultKey::generate();
        s.rekey(&key2).unwrap();
        drop(s);
        assert!(Store::open(&path, &key).is_err());
        assert!(Store::open(&path, &key2).is_ok());
    }

    #[test]
    fn legacy_password_vault_migrates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v.db");
        {
            let s = Store::open_inner(&path, "old passphrase").unwrap();
            s.org_add("chally").unwrap();
        }
        let s = Store::open_legacy(&path, "old passphrase").unwrap();
        let key = VaultKey::generate();
        s.rekey(&key).unwrap();
        drop(s);
        assert!(Store::open_legacy(&path, "old passphrase").is_err());
        assert_eq!(
            Store::open(&path, &key).unwrap().tree().unwrap()[0].name,
            "chally"
        );
    }

    #[test]
    fn sources_and_tree() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::create(&dir.path().join("v.db"), &VaultKey::generate()).unwrap();
        s.org_add("chally").unwrap();
        s.provider_add(
            "chally",
            "kv",
            "azure-keyvault",
            &json!({"vault_url": "https://x.vault.azure.net", "client_secret": "shh"}),
            false,
        )
        .unwrap();
        s.project_add("chally", "api").unwrap();
        s.env_add(&target(), true).unwrap();
        s.source_add(&target(), "kv", &json!({"prefix": "staging-"}))
            .unwrap();
        let srcs = s.sources(&target()).unwrap();
        assert_eq!(srcs.len(), 1);
        assert_eq!(srcs[0].provider.kind, "azure-keyvault");

        let tree = s.tree().unwrap();
        assert_eq!(tree[0].providers[0].config["client_secret"], "********");
        assert!(tree[0].projects[0].envs[0].protected);

        let prod: Target = "chally/api/prod".parse().unwrap();
        s.env_clone(&target(), &prod).unwrap();
        assert_eq!(s.sources(&prod).unwrap().len(), 1);

        // Removing the provider cascades to its sources.
        s.provider_remove("chally", "kv").unwrap();
        assert!(s.sources(&target()).unwrap().is_empty());
        assert!(s.org_add("chally").is_err());
    }
}
