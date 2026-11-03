//! `.vaultwrap.toml`: a committable file that links a directory to an
//! org/project and a default environment. It never contains secrets.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use vaultwrap_core::model::Target;

pub const FILE_NAME: &str = ".vaultwrap.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub org: String,
    pub project: String,
    /// Default environment when none is given with `-e` / `VAULTWRAP_ENV`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    /// Load automatically in shells set up with `vaultwrap hook` (default true).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto: Option<bool>,
}

pub struct Found {
    pub dir: PathBuf,
    pub file: ProjectFile,
}

pub fn find(start: &Path) -> Result<Option<Found>> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        let p = d.join(FILE_NAME);
        if p.is_file() {
            let text = std::fs::read_to_string(&p)?;
            let file: ProjectFile =
                toml::from_str(&text).with_context(|| format!("parsing {}", p.display()))?;
            return Ok(Some(Found {
                dir: d.to_path_buf(),
                file,
            }));
        }
        dir = d.parent();
    }
    Ok(None)
}

pub fn write(dir: &Path, file: &ProjectFile) -> Result<PathBuf> {
    let p = dir.join(FILE_NAME);
    let body = format!(
        "# Links this directory to a vaultwrap project. Safe to commit: no secrets here.\n{}",
        toml::to_string(file)?
    );
    std::fs::write(&p, body)?;
    Ok(p)
}

/// Work out the target environment from `-e` (an env name or full
/// `org/project/env`) and the nearest `.vaultwrap.toml`.
pub fn resolve_target(env_arg: Option<&str>) -> Result<Target> {
    if let Some(e) = env_arg {
        if e.contains('/') {
            return e.parse();
        }
    }
    let cwd = std::env::current_dir()?;
    let Some(found) = find(&cwd)? else {
        match env_arg {
            Some(e) => bail!(
                "no {FILE_NAME} found, so '{e}' is ambiguous — pass org/project/{e} or run `vaultwrap link <org>/<project>`"
            ),
            None => bail!("no {FILE_NAME} found — pass -e org/project/env or run `vaultwrap link <org>/<project>`"),
        }
    };
    target_for(&found.file, env_arg)
}

pub fn target_for(file: &ProjectFile, env_arg: Option<&str>) -> Result<Target> {
    let env = match env_arg.map(str::to_string).or_else(|| file.env.clone()) {
        Some(e) => e,
        None => bail!(
            "{FILE_NAME} for {}/{} has no default env — pass -e <env>",
            file.org,
            file.project
        ),
    };
    Ok(Target {
        org: file.org.clone(),
        project: file.project.clone(),
        env,
    })
}
