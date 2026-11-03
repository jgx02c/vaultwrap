//! Secret providers. A provider belongs to an org and holds connection
//! config; an environment "source" points at a provider plus per-environment
//! options (e.g. a name prefix). String values in both may use the templates
//! `{org}`, `{project}` and `{env}`.
//!
//! Kinds:
//! - `azure-keyvault`: config `{vault_url, auth?: "cli"|"client-secret", tenant_id?, client_id?, client_secret?}`
//!   options `{prefix?, keep_prefix?, uppercase?}`. Secret names map to env vars by
//!   stripping the prefix, `-` → `_`, uppercasing. A secret tag `env_var` overrides the name.
//! - `dotenv`: config/options `{path}`.
//! - `command`: config/options `{command, format?: "dotenv"|"json"}`, run with `sh -c`.
//!   Works with any CLI: doppler, 1Password (`op`), AWS, Infisical, vault, sops...

use crate::model::Target;
use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const AZURE_API_VERSION: &str = "7.4";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

/// Read a string setting, checking per-source options before provider config.
fn setting(config: &Value, options: &Value, key: &str, t: &Target) -> Option<String> {
    options
        .get(key)
        .or_else(|| config.get(key))
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Bool(b) => Some(b.to_string()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
        .map(|s| template(&s, t))
}

fn flag(config: &Value, options: &Value, key: &str, default: bool) -> bool {
    options
        .get(key)
        .or_else(|| config.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

pub fn template(s: &str, t: &Target) -> String {
    s.replace("{org}", &t.org)
        .replace("{project}", &t.project)
        .replace("{env}", &t.env)
}

/// How long fetched values stay in the daemon's memory cache.
pub fn cache_ttl(config: &Value) -> Duration {
    Duration::from_secs(
        config
            .get("cache_ttl")
            .and_then(Value::as_u64)
            .unwrap_or(300),
    )
}

pub async fn fetch(
    kind: &str,
    config: &Value,
    options: &Value,
    t: &Target,
) -> Result<Vec<(String, String)>> {
    match kind {
        "dotenv" => fetch_dotenv(config, options, t),
        "command" => fetch_command(config, options, t).await,
        "azure-keyvault" => fetch_azure(config, options, t).await,
        other => bail!("unknown provider kind '{other}'"),
    }
}

// ---------------------------------------------------------------- dotenv

/// Parse .env content. Values are taken literally: no `$VAR` interpolation,
/// because secrets frequently contain `$`.
pub fn parse_dotenv(content: &str) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    let mut chars = content.chars().peekable();
    let mut line_no = 1;
    loop {
        // Skip whitespace and blank lines.
        while let Some(&c) = chars.peek() {
            if c == '\n' {
                line_no += 1;
                chars.next();
            } else if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }
        let Some(&first) = chars.peek() else { break };
        if first == '#' {
            skip_rest_of_line(&mut chars, &mut line_no);
            continue;
        }
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c == '\n' {
                break;
            }
            key.push(c);
            chars.next();
        }
        let key = key.trim();
        let key = key
            .strip_prefix("export ")
            .map(str::trim)
            .unwrap_or(key)
            .to_string();
        if chars.next() != Some('=') {
            bail!("line {line_no}: expected KEY=VALUE");
        }
        crate::model::validate_env_key(&key).with_context(|| format!("line {line_no}"))?;
        while matches!(chars.peek(), Some(' ' | '\t')) {
            chars.next();
        }
        let mut value = String::new();
        match chars.peek() {
            Some('\'') => {
                chars.next();
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => {
                            if c == '\n' {
                                line_no += 1;
                            }
                            value.push(c)
                        }
                        None => bail!("line {line_no}: unterminated single quote"),
                    }
                }
                skip_rest_of_line(&mut chars, &mut line_no);
            }
            Some('"') => {
                chars.next();
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some('n') => value.push('\n'),
                            Some('r') => value.push('\r'),
                            Some('t') => value.push('\t'),
                            Some(c) => value.push(c),
                            None => bail!("line {line_no}: unterminated double quote"),
                        },
                        Some(c) => {
                            if c == '\n' {
                                line_no += 1;
                            }
                            value.push(c)
                        }
                        None => bail!("line {line_no}: unterminated double quote"),
                    }
                }
                skip_rest_of_line(&mut chars, &mut line_no);
            }
            _ => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        line_no += 1;
                        break;
                    }
                    value.push(c);
                }
                // Strip an inline comment (" #...") and surrounding whitespace.
                if let Some(i) = value.find(" #") {
                    value.truncate(i);
                }
                value = value.trim().to_string();
            }
        }
        out.push((key, value));
    }
    Ok(out)
}

fn skip_rest_of_line(chars: &mut std::iter::Peekable<std::str::Chars>, line_no: &mut usize) {
    for c in chars.by_ref() {
        if c == '\n' {
            *line_no += 1;
            break;
        }
    }
}

fn fetch_dotenv(config: &Value, options: &Value, t: &Target) -> Result<Vec<(String, String)>> {
    let path = setting(config, options, "path", t)
        .ok_or_else(|| anyhow!("dotenv provider needs a 'path'"))?;
    let path = crate::paths::expand_tilde(&path);
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    parse_dotenv(&content)
}

// ---------------------------------------------------------------- command

async fn fetch_command(
    config: &Value,
    options: &Value,
    t: &Target,
) -> Result<Vec<(String, String)>> {
    let cmd = setting(config, options, "command", t)
        .ok_or_else(|| anyhow!("command provider needs a 'command'"))?;
    let format = setting(config, options, "format", t).unwrap_or_else(|| "dotenv".into());
    let mut child = tokio::process::Command::new("sh");
    child
        .arg("-c")
        .arg(&cmd)
        .env("VAULTWRAP_ORG", &t.org)
        .env("VAULTWRAP_PROJECT", &t.project)
        .env("VAULTWRAP_ENV", &t.env)
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    let out = tokio::time::timeout(COMMAND_TIMEOUT, child.output())
        .await
        .map_err(|_| {
            anyhow!(
                "command timed out after {}s: {cmd}",
                COMMAND_TIMEOUT.as_secs()
            )
        })?
        .with_context(|| format!("running: {cmd}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("command failed ({}): {cmd}\n{}", out.status, stderr.trim());
    }
    let stdout = String::from_utf8(out.stdout).context("command output is not UTF-8")?;
    match format.as_str() {
        "dotenv" | "env" => parse_dotenv(&stdout),
        "json" => {
            let v: Value =
                serde_json::from_str(&stdout).context("command output is not valid JSON")?;
            let obj = v
                .as_object()
                .ok_or_else(|| anyhow!("JSON output must be an object"))?;
            Ok(obj
                .iter()
                .filter_map(|(k, v)| {
                    let s = match v {
                        Value::String(s) => s.clone(),
                        Value::Null => return None,
                        other => other.to_string(),
                    };
                    Some((k.clone(), s))
                })
                .collect())
        }
        other => bail!("unknown command output format '{other}' (use dotenv or json)"),
    }
}

// ---------------------------------------------------------------- azure

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("vaultwrap/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("http client")
    })
}

fn token_cache() -> &'static Mutex<HashMap<String, (String, Instant)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (String, Instant)>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

async fn azure_token(config: &Value, t: &Target) -> Result<String> {
    let tenant = setting(config, &Value::Null, "tenant_id", t);
    let client_id = setting(config, &Value::Null, "client_id", t);
    let client_secret = setting(config, &Value::Null, "client_secret", t);
    let auth = setting(config, &Value::Null, "auth", t).unwrap_or_else(|| {
        if client_secret.is_some() {
            "client-secret".into()
        } else {
            "cli".into()
        }
    });
    let cache_key = format!(
        "{auth}|{}|{}",
        tenant.as_deref().unwrap_or(""),
        client_id.as_deref().unwrap_or("")
    );
    if let Some((tok, exp)) = token_cache().lock().unwrap().get(&cache_key) {
        if *exp > Instant::now() + Duration::from_secs(60) {
            return Ok(tok.clone());
        }
    }

    let (token, ttl) = match auth.as_str() {
        "client-secret" => {
            let tenant =
                tenant.ok_or_else(|| anyhow!("azure client-secret auth needs tenant_id"))?;
            let client_id =
                client_id.ok_or_else(|| anyhow!("azure client-secret auth needs client_id"))?;
            let secret = client_secret
                .ok_or_else(|| anyhow!("azure client-secret auth needs client_secret"))?;
            let url = format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token");
            let resp = http()
                .post(url)
                .form(&[
                    ("grant_type", "client_credentials"),
                    ("client_id", client_id.as_str()),
                    ("client_secret", secret.as_str()),
                    ("scope", "https://vault.azure.net/.default"),
                ])
                .send()
                .await
                .context("requesting Azure AD token")?;
            let status = resp.status();
            let body: Value = resp
                .json()
                .await
                .context("reading Azure AD token response")?;
            if !status.is_success() {
                bail!(
                    "Azure AD token request failed ({status}): {}",
                    body.get("error_description")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown error")
                );
            }
            let tok = body["access_token"]
                .as_str()
                .ok_or_else(|| anyhow!("no access_token in response"))?;
            (tok.to_string(), body["expires_in"].as_u64().unwrap_or(3000))
        }
        "cli" => {
            let mut cmd = tokio::process::Command::new("az");
            cmd.args([
                "account",
                "get-access-token",
                "--resource",
                "https://vault.azure.net",
                "--output",
                "json",
            ]);
            if let Some(tenant) = &tenant {
                cmd.args(["--tenant", tenant]);
            }
            let out = tokio::time::timeout(COMMAND_TIMEOUT, cmd.output())
                .await
                .map_err(|_| anyhow!("`az account get-access-token` timed out"))?
                .context("running `az` (is the Azure CLI installed and on PATH?)")?;
            if !out.status.success() {
                bail!(
                    "`az account get-access-token` failed — run `az login`{}\n{}",
                    tenant
                        .as_ref()
                        .map(|t| format!(" --tenant {t}"))
                        .unwrap_or_default(),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            let body: Value = serde_json::from_slice(&out.stdout).context("parsing az output")?;
            let tok = body["accessToken"]
                .as_str()
                .ok_or_else(|| anyhow!("no accessToken in az output"))?;
            (tok.to_string(), 1800)
        }
        other => bail!("unknown azure auth '{other}' (use cli or client-secret)"),
    };
    token_cache().lock().unwrap().insert(
        cache_key,
        (token.clone(), Instant::now() + Duration::from_secs(ttl)),
    );
    Ok(token)
}

async fn azure_get(url: &str, token: &str) -> Result<Value> {
    let resp = http()
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .with_context(|| format!("GET {url}"))?;
    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        let msg = body["error"]["message"].as_str().unwrap_or("unknown error");
        bail!("Key Vault request failed ({status}): {msg}");
    }
    Ok(body)
}

/// Map a Key Vault secret name to an env var name.
pub fn azure_env_name(
    secret: &str,
    prefix: &str,
    keep_prefix: bool,
    uppercase: bool,
) -> Option<String> {
    let lower_secret = secret.to_ascii_lowercase();
    if !lower_secret.starts_with(&prefix.to_ascii_lowercase()) {
        return None;
    }
    let base = if keep_prefix {
        secret
    } else {
        &secret[prefix.len()..]
    };
    if base.is_empty() {
        return None;
    }
    let name = base.replace('-', "_");
    Some(if uppercase {
        name.to_ascii_uppercase()
    } else {
        name
    })
}

async fn fetch_azure(config: &Value, options: &Value, t: &Target) -> Result<Vec<(String, String)>> {
    let vault = setting(config, options, "vault_url", t)
        .ok_or_else(|| anyhow!("azure-keyvault needs vault_url"))?;
    let vault = vault.trim_end_matches('/').to_string();
    let prefix = setting(config, options, "prefix", t).unwrap_or_default();
    let keep_prefix = flag(config, options, "keep_prefix", false);
    let uppercase = flag(config, options, "uppercase", true);
    let token = azure_token(config, t).await?;

    // List secret names (paged), honoring prefix, enabled flag and env_var tags.
    let mut wanted: Vec<(String, String)> = Vec::new(); // (env name, secret url)
    let mut next = Some(format!(
        "{vault}/secrets?api-version={AZURE_API_VERSION}&maxresults=25"
    ));
    while let Some(url) = next.take() {
        let page = azure_get(&url, &token).await?;
        for item in page["value"].as_array().into_iter().flatten() {
            if item["attributes"]["enabled"].as_bool() == Some(false) {
                continue;
            }
            let Some(id) = item["id"].as_str() else {
                continue;
            };
            let secret_name = id.rsplit('/').next().unwrap_or_default();
            let tag = item["tags"]["env_var"].as_str();
            let env_name = match azure_env_name(secret_name, &prefix, keep_prefix, uppercase) {
                Some(n) => tag.map(str::to_string).unwrap_or(n),
                None => continue,
            };
            wanted.push((
                env_name,
                format!("{vault}/secrets/{secret_name}?api-version={AZURE_API_VERSION}"),
            ));
        }
        next = page["nextLink"].as_str().map(str::to_string);
    }

    // Fetch values with bounded concurrency.
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(8));
    let mut set = tokio::task::JoinSet::new();
    for (i, (name, url)) in wanted.into_iter().enumerate() {
        let token = token.clone();
        let sem = sem.clone();
        set.spawn(async move {
            let _permit = sem.acquire_owned().await;
            let body = azure_get(&url, &token).await?;
            let value = body["value"].as_str().unwrap_or_default().to_string();
            Ok::<_, anyhow::Error>((i, name, value))
        });
    }
    let mut out = Vec::new();
    while let Some(res) = set.join_next().await {
        out.push(res??);
    }
    out.sort_by_key(|(i, _, _)| *i);
    Ok(out.into_iter().map(|(_, k, v)| (k, v)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn t() -> Target {
        "chally/api/staging".parse().unwrap()
    }

    #[test]
    fn azure_names() {
        assert_eq!(
            azure_env_name("staging-database-url", "staging-", false, true).as_deref(),
            Some("DATABASE_URL")
        );
        assert_eq!(
            azure_env_name("Staging-Api-Key", "staging-", false, true).as_deref(),
            Some("API_KEY")
        );
        assert_eq!(
            azure_env_name("prod-database-url", "staging-", false, true),
            None
        );
        assert_eq!(
            azure_env_name("api-key", "", false, false).as_deref(),
            Some("api_key")
        );
    }

    #[test]
    fn templates() {
        assert_eq!(template("{env}-", &t()), "staging-");
        assert_eq!(
            template("~/clients/{org}/{project}/.env.{env}", &t()),
            "~/clients/chally/api/.env.staging"
        );
    }

    #[tokio::test]
    async fn command_provider() {
        let cfg = json!({"command": "printf 'A=1\\nB=\"two words\"\\n'; echo ENV=$VAULTWRAP_ENV"});
        let vars = fetch("command", &cfg, &json!({}), &t()).await.unwrap();
        assert_eq!(
            vars,
            vec![
                ("A".into(), "1".into()),
                ("B".into(), "two words".into()),
                ("ENV".into(), "staging".into())
            ]
        );
        let cfg = json!({"command": "echo '{\"X\": \"y\", \"N\": 3}'", "format": "json"});
        let mut vars = fetch("command", &cfg, &json!({}), &t()).await.unwrap();
        vars.sort();
        assert_eq!(
            vars,
            vec![("N".into(), "3".into()), ("X".into(), "y".into())]
        );
        assert!(
            fetch("command", &json!({"command": "exit 3"}), &json!({}), &t())
                .await
                .is_err()
        );
    }

    #[test]
    fn dotenv_parsing() {
        let src = "# comment\nexport A=1\nB = plain value # note\nC='pa$$word # not comment'\nD=\"line1\\nline2 \\\"q\\\"\"\nE=\nF='multi\nline'\nG=a$b\n";
        let vars = parse_dotenv(src).unwrap();
        assert_eq!(
            vars,
            vec![
                ("A".into(), "1".into()),
                ("B".into(), "plain value".into()),
                ("C".into(), "pa$$word # not comment".into()),
                ("D".into(), "line1\nline2 \"q\"".into()),
                ("E".into(), "".into()),
                ("F".into(), "multi\nline".into()),
                ("G".into(), "a$b".into()),
            ]
        );
        assert!(parse_dotenv("NOEQUALS\n").is_err());
        assert!(parse_dotenv("X='open\n").is_err());
    }

    #[test]
    fn dotenv_provider() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".env.staging"),
            "# c\nFOO=bar\nexport BAZ='q'\n",
        )
        .unwrap();
        let cfg = json!({"path": format!("{}/.env.{{env}}", dir.path().display())});
        let vars = fetch_dotenv(&cfg, &json!({}), &t()).unwrap();
        assert_eq!(
            vars,
            vec![("FOO".into(), "bar".into()), ("BAZ".into(), "q".into())]
        );
    }
}
