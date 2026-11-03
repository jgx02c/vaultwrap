mod project;
mod service;
mod shell;

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::{json, Map, Value};
use shell::{HookState, Shell};
use std::io::{IsTerminal, Read, Write};
use vaultwrap_core::client::Client;
use vaultwrap_core::model::{Resolved, Target};
use vaultwrap_core::protocol::{ErrorCode, Request, Status};
use vaultwrap_core::store::OrgNode;

const EXAMPLES: &str = "\
Quick start:
  vaultwrap init                                   # create the encrypted vault
  vaultwrap env add chally/api/staging             # creates org + project as needed
  vaultwrap link chally/api -e staging             # in your repo: writes .vaultwrap.toml
  vaultwrap import .env                            # pull an existing .env into the vault
  vaultwrap run -- npm run dev                     # run with secrets injected

Connect Azure Key Vault for an org:
  vaultwrap provider add chally/kv azure --vault-url https://chally-kv.vault.azure.net
  vaultwrap source add kv --prefix '{env}-'        # staging-database-url -> DATABASE_URL

Shell integration (add to ~/.zshrc):
  eval \"$(vaultwrap hook zsh)\"                     # auto-load on cd into a linked project";

#[derive(Parser)]
#[command(name = "vaultwrap", version, about = "Inject secrets from an encrypted local vault or remote stores (Azure Key Vault, .env files, any CLI) into your processes.", after_help = EXAMPLES)]
struct Cli {
    /// Environment: a name like `staging` (uses .vaultwrap.toml) or a full `org/project/env`.
    #[arg(short = 'e', long = "env", global = true, env = "VAULTWRAP_ENV")]
    env: Option<String>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a new encrypted vault. Its key is kept in your keychain.
    Init {
        /// Read the master password from stdin instead of prompting.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Unlock with the master password (also saves the key back to the keychain).
    Unlock {
        /// Read the password from stdin instead of prompting.
        #[arg(long)]
        password_stdin: bool,
        /// Unlock with the key in the keychain instead of the password.
        #[arg(long, conflicts_with = "password_stdin")]
        keychain: bool,
    },
    /// Lock the vault until you unlock it again.
    Lock,
    /// Remove the vault key from the keychain; the master password is needed next time.
    Forget,
    /// Change the master password. --reset sets a new one using your Mac password instead.
    Passwd {
        /// Forgot it? Confirm your Mac login password instead of the old master password.
        #[arg(long)]
        reset: bool,
        /// Read passwords from stdin, one per line (current unless --reset, then new).
        #[arg(long)]
        password_stdin: bool,
    },
    /// Erase the vault (moved to a backup folder) and start over.
    ResetVault {
        #[arg(long)]
        yes: bool,
    },
    /// Show daemon, vault, service and current project status.
    Status,
    /// Show all orgs, providers, projects and environments.
    Tree,
    /// Manage orgs (clients / companies).
    #[command(subcommand)]
    Org(OrgCmd),
    /// Manage secret providers (Azure Key Vault, .env files, commands) for an org.
    #[command(subcommand)]
    Provider(ProviderCmd),
    /// Manage projects within an org.
    #[command(subcommand)]
    Project(ProjectCmd),
    /// Manage environments (staging, prod, ...) within a project.
    #[command(subcommand, name = "env")]
    Environment(EnvCmd),
    /// Attach providers to an environment.
    #[command(subcommand)]
    Source(SourceCmd),
    /// Store variables locally in the vault: KEY=VALUE, or KEY to be prompted.
    Set {
        #[arg(required = true)]
        vars: Vec<String>,
    },
    /// Remove locally stored variables.
    Unset {
        #[arg(required = true)]
        keys: Vec<String>,
    },
    /// Print one resolved variable.
    Get { key: String },
    /// List resolved variables and where each comes from.
    Ls {
        /// Show values instead of masking them.
        #[arg(long)]
        show: bool,
        /// Bypass the cache and refetch remote sources.
        #[arg(long)]
        refresh: bool,
    },
    /// Import a .env file into the vault as local variables.
    Import { file: std::path::PathBuf },
    /// Print resolved variables.
    Export {
        #[arg(long, value_enum, default_value_t = ExportFormat::Dotenv)]
        format: ExportFormat,
    },
    /// Link the current directory to <org>/<project> by writing .vaultwrap.toml (default env from -e).
    Link {
        project: String,
        /// Don't auto-load in hooked shells.
        #[arg(long)]
        no_auto: bool,
    },
    /// Run a command with secrets injected: vaultwrap run -- <cmd> [args...]
    Run {
        /// Skip the confirmation for protected environments.
        #[arg(short, long)]
        yes: bool,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        command: Vec<String>,
    },
    /// Start a subshell with secrets injected (exit to leave).
    Shell {
        #[arg(short, long)]
        yes: bool,
    },
    /// Print shell code to load secrets into the current shell: eval "$(vaultwrap activate)"
    Activate {
        #[arg(long, value_enum)]
        shell: Option<Shell>,
    },
    /// Print the shell hook that auto-loads secrets on cd: eval "$(vaultwrap hook zsh)"
    Hook {
        #[arg(value_enum)]
        shell: Shell,
    },
    #[command(hide = true)]
    HookEnv {
        #[arg(long, value_enum)]
        shell: Shell,
    },
    /// Drop cached remote values so the next run refetches them.
    Refresh,
    /// Run the daemon in the foreground (normally started by the service or on demand).
    Daemon {
        /// Lock after this many idle minutes (0 = never). Only when the key isn't in the keychain.
        #[arg(long, default_value_t = 720)]
        idle_lock: u64,
        /// Replace an already-running daemon (used by the login service).
        #[arg(long)]
        takeover: bool,
    },
    /// Install/remove the background service that starts the daemon at login.
    #[command(subcommand)]
    Service(ServiceCmd),
}

#[derive(Subcommand)]
enum OrgCmd {
    Add { name: String },
    Rm { name: String },
    Ls,
}

#[derive(Clone, Copy, ValueEnum)]
enum Kind {
    #[value(name = "azure-keyvault", alias = "azure")]
    Azure,
    #[value(name = "dotenv", alias = "env-file")]
    Dotenv,
    #[value(name = "command", alias = "cmd")]
    Command,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Azure => "azure-keyvault",
            Kind::Dotenv => "dotenv",
            Kind::Command => "command",
        }
    }
}

#[derive(Subcommand)]
enum ProviderCmd {
    /// Add a provider: vaultwrap provider add <org>/<name> <kind> [options]
    Add {
        /// <org>/<name>
        name: String,
        #[arg(value_enum)]
        kind: Kind,
        /// Azure: https://<vault>.vault.azure.net
        #[arg(long)]
        vault_url: Option<String>,
        /// Azure auth: `cli` (uses `az login`, default) or `client-secret` (service principal).
        #[arg(long)]
        auth: Option<String>,
        /// Azure tenant ID.
        #[arg(long)]
        tenant: Option<String>,
        /// Azure service principal client ID.
        #[arg(long)]
        client_id: Option<String>,
        /// Prompt for an Azure service principal secret (implies --auth client-secret).
        #[arg(long)]
        client_secret: bool,
        /// dotenv: path to the file. Templates: {org} {project} {env}
        #[arg(long)]
        path: Option<String>,
        /// command: shell command printing dotenv or JSON. Templates: {org} {project} {env}
        #[arg(long)]
        command: Option<String>,
        /// command: output format, `dotenv` (default) or `json`.
        #[arg(long)]
        format: Option<String>,
        /// Seconds to cache fetched values in memory (default 300).
        #[arg(long)]
        ttl: Option<u64>,
        /// Extra config as key=value.
        #[arg(long = "set", value_name = "KEY=VALUE")]
        extra: Vec<String>,
        /// Overwrite an existing provider with the same name.
        #[arg(long)]
        replace: bool,
    },
    Rm {
        name: String,
    },
    /// List providers (credentials are redacted).
    Ls {
        org: Option<String>,
    },
    /// Fetch from a provider and list the variable names it returns.
    Test {
        name: String,
        #[arg(long)]
        prefix: Option<String>,
    },
}

#[derive(Subcommand)]
enum ProjectCmd {
    /// vaultwrap project add <org>/<project>
    Add {
        name: String,
    },
    Rm {
        name: String,
    },
    Ls {
        org: Option<String>,
    },
}

#[derive(Subcommand)]
enum EnvCmd {
    /// vaultwrap env add <org>/<project>/<env> (creates the org and project if needed)
    Add {
        target: String,
        /// Never auto-load into shells; confirm before `run`. Use for prod.
        #[arg(long)]
        protect: bool,
        /// Copy sources and local variables from another environment.
        #[arg(long)]
        from: Option<String>,
    },
    Rm {
        target: String,
    },
    Ls {
        project: Option<String>,
    },
    /// Mark an environment protected (or --off to unprotect).
    Protect {
        target: String,
        #[arg(long)]
        off: bool,
    },
}

#[derive(Subcommand)]
enum SourceCmd {
    /// Attach a provider to the environment (later sources override earlier ones; local values win).
    Add {
        provider: String,
        /// Azure: only secrets starting with this prefix (stripped). Templates: {env} {project} {org}
        #[arg(long)]
        prefix: Option<String>,
        /// Azure: keep the prefix in variable names.
        #[arg(long)]
        keep_prefix: bool,
        /// Per-environment option overrides as key=value (e.g. path=..., command=..., vault_url=...).
        #[arg(long = "opt", value_name = "KEY=VALUE")]
        opts: Vec<String>,
    },
    Rm {
        provider: String,
    },
    Ls,
}

#[derive(Subcommand)]
enum ServiceCmd {
    /// Install and start the daemon as a login service (launchd / systemd --user).
    Install {
        #[arg(long, default_value_t = 720)]
        idle_lock: u64,
    },
    Uninstall,
    Status,
}

#[derive(Clone, Copy, ValueEnum)]
enum ExportFormat {
    Dotenv,
    Json,
    Shell,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if !matches!(cli.cmd, Cmd::Daemon { .. }) {
        // Exit quietly when piped into `head` etc. (the daemon keeps Rust's default
        // of ignoring SIGPIPE so a disconnecting client can't kill it).
        #[cfg(unix)]
        unsafe {
            libc::signal(libc::SIGPIPE, libc::SIG_DFL);
        }
    }
    if let Err(e) = real_main(cli).await {
        eprintln!("vaultwrap: {e:#}");
        std::process::exit(1);
    }
}

// ------------------------------------------------------------------ helpers

#[derive(Debug)]
struct Locked;
impl std::fmt::Display for Locked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("vault is locked — run `vaultwrap unlock`")
    }
}
impl std::error::Error for Locked {}

struct Conn {
    client: Client,
    /// Prompt for the master password when the vault is locked.
    interactive: bool,
}

fn tty() -> bool {
    std::fs::File::open("/dev/tty").is_ok() && std::io::stderr().is_terminal()
}

fn prompt_password(msg: &str) -> Result<String> {
    rpassword::prompt_password(msg).context("reading password")
}

fn read_stdin_password() -> Result<String> {
    let mut pw = String::new();
    std::io::stdin().read_to_string(&mut pw)?;
    Ok(pw.trim_end_matches(['\n', '\r']).to_string())
}

fn confirm(msg: &str) -> bool {
    eprint!("{msg} [y/N] ");
    let _ = std::io::stderr().flush();
    let mut tty = match std::fs::File::open("/dev/tty") {
        Ok(f) => std::io::BufReader::new(f),
        Err(_) => return false,
    };
    let mut line = String::new();
    use std::io::BufRead;
    tty.read_line(&mut line).is_ok() && matches!(line.trim(), "y" | "Y" | "yes")
}

impl Conn {
    async fn new(interactive: bool) -> Result<Self> {
        let exe = std::env::current_exe()?;
        Ok(Conn {
            client: Client::connect_or_spawn(&exe).await?,
            interactive,
        })
    }

    async fn call(&mut self, req: Request) -> Result<Value> {
        let resp = self.client.call(&req).await?;
        if resp.ok {
            return Ok(resp.data);
        }
        match resp.code {
            Some(ErrorCode::Locked) if self.interactive && tty() => {
                self.unlock_prompt().await?;
                let resp = self.client.call(&req).await?;
                if resp.ok {
                    Ok(resp.data)
                } else {
                    Err(anyhow!(resp.error.unwrap_or_default()))
                }
            }
            Some(ErrorCode::Locked) => Err(Locked.into()),
            _ => Err(anyhow!(resp
                .error
                .unwrap_or_else(|| "unknown error".into()))),
        }
    }

    async fn unlock_prompt(&mut self) -> Result<()> {
        for attempt in 1..=3 {
            let password = prompt_password("vaultwrap master password: ")?;
            let resp = self.client.call(&Request::Unlock { password }).await?;
            if resp.ok {
                if let Some(w) = resp.data["warning"].as_str() {
                    eprintln!("vaultwrap: warning: {w}");
                }
                return Ok(());
            }
            if resp.code != Some(ErrorCode::WrongPassword) || attempt == 3 {
                bail!(resp.error.unwrap_or_default());
            }
            eprintln!("wrong password, try again");
        }
        unreachable!()
    }

    async fn status(&mut self) -> Result<Status> {
        Ok(serde_json::from_value(self.call(Request::Status).await?)?)
    }

    async fn resolve(&mut self, target: &Target, refresh: bool) -> Result<Resolved> {
        let r: Resolved = serde_json::from_value(
            self.call(Request::Resolve {
                target: target.clone(),
                refresh,
            })
            .await?,
        )?;
        for w in &r.warnings {
            eprintln!("vaultwrap: warning: {w}");
        }
        Ok(r)
    }

    async fn tree(&mut self) -> Result<Vec<OrgNode>> {
        Ok(serde_json::from_value(self.call(Request::Tree).await?)?)
    }
}

fn split2(s: &str, what: &str) -> Result<(String, String)> {
    match s.split_once('/') {
        Some((a, b)) if !a.is_empty() && !b.is_empty() && !b.contains('/') => {
            Ok((a.into(), b.into()))
        }
        _ => bail!("expected {what}, got '{s}'"),
    }
}

fn parse_kv(s: &str) -> Result<(String, String)> {
    s.split_once('=')
        .map(|(k, v)| (k.trim().to_string(), v.to_string()))
        .ok_or_else(|| anyhow!("expected KEY=VALUE, got '{s}'"))
}

/// Interpret `true`/`false`/numbers in key=value options.
fn kv_value(v: &str) -> Value {
    match v {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => v
            .parse::<u64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(v.into())),
    }
}

fn mask(v: &str) -> String {
    let n = v.chars().count();
    if n <= 4 {
        "•".repeat(n.max(1))
    } else {
        format!(
            "{}{}",
            v.chars().take(3).collect::<String>(),
            "•".repeat((n - 3).min(12))
        )
    }
}

/// Ignore "already exists" so `env add` can create parents on demand.
fn ignore_exists(r: Result<Value>) -> Result<bool> {
    match r {
        Ok(_) => Ok(true),
        Err(e) if e.to_string().contains("already exists") => Ok(false),
        Err(e) => Err(e),
    }
}

fn vars_of(r: &Resolved) -> Vec<(String, String)> {
    r.vars
        .iter()
        .map(|v| (v.key.clone(), v.value.clone()))
        .collect()
}

fn guard_protected(r: &Resolved, yes: bool) -> Result<()> {
    if !r.protected || yes {
        return Ok(());
    }
    if tty()
        && confirm(&format!(
            "warning: {} is a protected environment. Continue?",
            r.target
        ))
    {
        return Ok(());
    }
    bail!("{} is protected; pass --yes to confirm", r.target)
}

// ------------------------------------------------------------------ commands

async fn real_main(cli: Cli) -> Result<()> {
    let env = cli.env.as_deref();
    match cli.cmd {
        Cmd::Daemon {
            idle_lock,
            takeover,
        } => {
            vaultwrap_core::daemon::run(vaultwrap_core::daemon::DaemonOptions {
                idle_lock_minutes: idle_lock,
                takeover,
            })
            .await
        }
        Cmd::Service(sc) => match sc {
            ServiceCmd::Install { idle_lock } => {
                // Stop an on-demand daemon so the service owns the socket.
                if let Ok(mut c) = Client::connect().await {
                    let _ = c.call(&Request::Shutdown).await;
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                }
                service::install(idle_lock)
            }
            ServiceCmd::Uninstall => service::uninstall(),
            ServiceCmd::Status => {
                match service::installed()? {
                    Some(s) => println!("service: installed — {s}"),
                    None => println!("service: not installed (run `vaultwrap service install`)"),
                }
                Ok(())
            }
        },
        Cmd::Hook { shell } => {
            let exe = std::env::current_exe()?.display().to_string();
            print!("{}", shell::hook_script(shell, &exe));
            Ok(())
        }
        Cmd::HookEnv { shell } => hook_env(shell, env).await,
        Cmd::Link { project, no_auto } => {
            let (org, project) = split2(&project, "<org>/<project>")?;
            let file = project::ProjectFile {
                org,
                project,
                env: env.map(str::to_string),
                auto: if no_auto { Some(false) } else { None },
            };
            let p = project::write(&std::env::current_dir()?, &file)?;
            println!("wrote {}", p.display());
            if let Ok(mut c) = Conn::new(false).await {
                if let Some(e) = &file.env {
                    let t = project::target_for(&file, Some(e))?;
                    if let Err(err) = c.resolve(&t, false).await {
                        if err.to_string().contains("not found") {
                            eprintln!("note: {t} doesn't exist yet — create it with `vaultwrap env add {t}`");
                        }
                    }
                }
            }
            Ok(())
        }
        cmd => with_daemon(cmd, env).await,
    }
}

async fn with_daemon(cmd: Cmd, env: Option<&str>) -> Result<()> {
    let mut c = Conn::new(true).await?;
    match cmd {
        Cmd::Init { password_stdin } => {
            let st = c.status().await?;
            if st.vault_exists {
                bail!("a vault already exists in {}", st.home);
            }
            let pw = if password_stdin {
                read_stdin_password()?
            } else {
                eprintln!("Creating a new vault in {}", st.home);
                eprintln!(
                    "Its key is stored in your {}, so it unlocks with your computer login.",
                    st.keychain
                );
                eprintln!(
                    "The master password recovers it if that entry is ever lost. Keep it safe."
                );
                let pw = prompt_password("master password: ")?;
                if pw != prompt_password("confirm: ")? {
                    bail!("passwords don't match");
                }
                pw
            };
            let r = c.call(Request::Init { password: pw }).await?;
            if let Some(w) = r["warning"].as_str() {
                eprintln!("vaultwrap: warning: {w}");
            }
            println!("vault created");
            println!("next: vaultwrap env add <org>/<project>/<env>");
        }
        Cmd::Unlock {
            password_stdin,
            keychain,
        } => {
            if keychain {
                c.call(Request::UnlockKeychain).await?;
            } else if password_stdin {
                let r = c
                    .call(Request::Unlock {
                        password: read_stdin_password()?,
                    })
                    .await?;
                if let Some(w) = r["warning"].as_str() {
                    eprintln!("vaultwrap: warning: {w}");
                }
            } else {
                c.unlock_prompt().await?;
            }
            println!("unlocked");
        }
        Cmd::Lock => {
            c.call(Request::Lock).await?;
            println!("locked — unlock with `vaultwrap unlock`");
        }
        Cmd::Forget => {
            c.call(Request::Forget).await?;
            println!("removed the vault key from the keychain");
            println!("the next unlock needs your master password, then it is stored again");
        }
        Cmd::Passwd {
            reset,
            password_stdin,
        } => {
            let mut lines = Vec::new();
            if password_stdin {
                let mut all = String::new();
                std::io::stdin().read_to_string(&mut all)?;
                lines = all.lines().map(str::to_string).collect();
                lines.reverse();
            }
            let mut next = |prompt: &str| -> Result<String> {
                if password_stdin {
                    lines
                        .pop()
                        .ok_or_else(|| anyhow!("not enough lines on stdin"))
                } else {
                    prompt_password(prompt)
                }
            };
            c.call(Request::Tree).await?; // unlocks (prompting) if needed
            if reset {
                vaultwrap_core::system::confirm_computer_password(
                    "Vaultwrap wants to reset your vault's master password.",
                )?;
            } else {
                let current = next("current master password: ")?;
                c.call(Request::VerifyPassword { password: current })
                    .await?;
            }
            let pw = next("new master password: ")?;
            if !password_stdin && pw != next("confirm: ")? {
                bail!("passwords don't match");
            }
            c.call(Request::SetPassword { new_password: pw }).await?;
            println!("master password changed");
        }
        Cmd::ResetVault { yes } => {
            let st = c.status().await?;
            if !st.vault_exists {
                bail!("there is no vault to erase");
            }
            if !yes {
                eprintln!(
                    "This moves your vault to a backup folder and removes its keychain entry."
                );
                eprint!("Type ERASE to continue: ");
                let _ = std::io::stderr().flush();
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;
                if line.trim() != "ERASE" {
                    bail!("cancelled");
                }
            }
            let r = c.call(Request::EraseVault).await?;
            println!(
                "vault erased; backup kept at {}",
                r["backup"].as_str().unwrap_or("?")
            );
            println!("create a new one with `vaultwrap init`");
        }
        Cmd::Status => {
            let st = c.status().await?;
            println!("daemon   running (pid {}, v{})", st.pid, st.version);
            println!(
                "vault    {}",
                if !st.vault_exists {
                    "not created — run `vaultwrap init`".to_string()
                } else {
                    if st.unlocked {
                        "unlocked".to_string()
                    } else {
                        "locked".to_string()
                    }
                }
            );
            if st.vault_exists {
                println!(
                    "keychain {}",
                    if st.key_in_keychain {
                        format!("vault key stored in {}", st.keychain)
                    } else {
                        format!(
                            "key not in {} — unlock with your master password",
                            st.keychain
                        )
                    }
                );
            }
            println!("home     {}", st.home);
            println!(
                "service  {}",
                service::installed()?
                    .unwrap_or_else(|| "not installed (`vaultwrap service install`)".into())
            );
            match project::find(&std::env::current_dir()?)? {
                Some(f) => println!(
                    "project  {}/{} (default env: {}) — {}",
                    f.file.org,
                    f.file.project,
                    f.file.env.as_deref().unwrap_or("none"),
                    f.dir.join(project::FILE_NAME).display()
                ),
                None => println!("project  none linked here"),
            }
            if let Ok(a) = std::env::var("VAULTWRAP_ACTIVE") {
                println!("active   {a}");
            }
        }
        Cmd::Tree => print_tree(&c.tree().await?, None),
        Cmd::Org(o) => match o {
            OrgCmd::Add { name } => {
                c.call(Request::OrgAdd { name: name.clone() }).await?;
                println!("org {name}");
            }
            OrgCmd::Rm { name } => {
                if !confirm(&format!(
                    "Delete org '{name}' and ALL its projects, environments and providers?"
                )) {
                    bail!("cancelled");
                }
                c.call(Request::OrgRemove { name: name.clone() }).await?;
                println!("removed org {name}");
            }
            OrgCmd::Ls => {
                for o in c.tree().await? {
                    println!("{}", o.name);
                }
            }
        },
        Cmd::Provider(p) => provider_cmd(&mut c, p, env).await?,
        Cmd::Project(p) => match p {
            ProjectCmd::Add { name } => {
                let (org, name) = split2(&name, "<org>/<project>")?;
                c.call(Request::ProjectAdd {
                    org: org.clone(),
                    name: name.clone(),
                })
                .await?;
                println!("project {org}/{name}");
            }
            ProjectCmd::Rm { name } => {
                let (org, name) = split2(&name, "<org>/<project>")?;
                if !confirm(&format!(
                    "Delete project '{org}/{name}' and all its environments?"
                )) {
                    bail!("cancelled");
                }
                c.call(Request::ProjectRemove { org, name }).await?;
                println!("removed");
            }
            ProjectCmd::Ls { org } => {
                for o in c.tree().await? {
                    if org.as_ref().is_some_and(|x| x != &o.name) {
                        continue;
                    }
                    for p in &o.projects {
                        println!("{}/{}", o.name, p.name);
                    }
                }
            }
        },
        Cmd::Environment(e) => match e {
            EnvCmd::Add {
                target,
                protect,
                from,
            } => {
                let t: Target = target.parse()?;
                if let Some(from) = from {
                    let from: Target = from.parse()?;
                    c.call(Request::EnvClone {
                        from: from.clone(),
                        to: t.clone(),
                    })
                    .await?;
                    if protect {
                        c.call(Request::EnvProtect {
                            target: t.clone(),
                            protected: true,
                        })
                        .await?;
                    }
                    println!("environment {t} (copied from {from})");
                } else {
                    if ignore_exists(
                        c.call(Request::OrgAdd {
                            name: t.org.clone(),
                        })
                        .await,
                    )? {
                        println!("org {}", t.org);
                    }
                    if ignore_exists(
                        c.call(Request::ProjectAdd {
                            org: t.org.clone(),
                            name: t.project.clone(),
                        })
                        .await,
                    )? {
                        println!("project {}/{}", t.org, t.project);
                    }
                    c.call(Request::EnvAdd {
                        target: t.clone(),
                        protected: protect,
                    })
                    .await?;
                    println!(
                        "environment {t}{}",
                        if protect { " (protected)" } else { "" }
                    );
                }
            }
            EnvCmd::Rm { target } => {
                let t: Target = target.parse()?;
                if !confirm(&format!(
                    "Delete environment '{t}' and its local variables?"
                )) {
                    bail!("cancelled");
                }
                c.call(Request::EnvRemove { target: t }).await?;
                println!("removed");
            }
            EnvCmd::Ls { project } => {
                let filter = project.map(|p| split2(&p, "<org>/<project>")).transpose()?;
                for o in c.tree().await? {
                    for p in &o.projects {
                        if filter
                            .as_ref()
                            .is_some_and(|(fo, fp)| fo != &o.name || fp != &p.name)
                        {
                            continue;
                        }
                        for e in &p.envs {
                            println!(
                                "{}/{}/{}{}",
                                o.name,
                                p.name,
                                e.name,
                                if e.protected { "  [protected]" } else { "" }
                            );
                        }
                    }
                }
            }
            EnvCmd::Protect { target, off } => {
                let t: Target = target.parse()?;
                c.call(Request::EnvProtect {
                    target: t.clone(),
                    protected: !off,
                })
                .await?;
                println!("{t} {}", if off { "unprotected" } else { "protected" });
            }
        },
        Cmd::Source(s) => {
            let t = project::resolve_target(env)?;
            match s {
                SourceCmd::Add {
                    provider,
                    prefix,
                    keep_prefix,
                    opts,
                } => {
                    let mut o = Map::new();
                    if let Some(p) = prefix {
                        o.insert("prefix".into(), p.into());
                    }
                    if keep_prefix {
                        o.insert("keep_prefix".into(), true.into());
                    }
                    for kv in opts {
                        let (k, v) = parse_kv(&kv)?;
                        o.insert(k, kv_value(&v));
                    }
                    c.call(Request::SourceAdd {
                        target: t.clone(),
                        provider: provider.clone(),
                        options: Value::Object(o),
                    })
                    .await?;
                    println!("{t} ← {provider}");
                    match c.resolve(&t, true).await {
                        Ok(r) => {
                            let n = r.vars.iter().filter(|v| v.source == provider).count();
                            println!(
                                "  {n} variable{} in effect from {provider}",
                                if n == 1 { "" } else { "s" }
                            );
                        }
                        Err(e) => eprintln!("  warning: could not fetch yet: {e:#}"),
                    }
                }
                SourceCmd::Rm { provider } => {
                    c.call(Request::SourceRemove {
                        target: t.clone(),
                        provider: provider.clone(),
                    })
                    .await?;
                    println!("detached {provider} from {t}");
                }
                SourceCmd::Ls => {
                    for o in c.tree().await? {
                        for p in o
                            .projects
                            .iter()
                            .filter(|p| o.name == t.org && p.name == t.project)
                        {
                            for e in p.envs.iter().filter(|e| e.name == t.env) {
                                for (i, s) in e.sources.iter().enumerate() {
                                    println!(
                                        "{}. {} ({}) {}",
                                        i + 1,
                                        s.provider,
                                        s.kind,
                                        fmt_opts(&s.options)
                                    );
                                }
                                println!(
                                    "{}. local ({} variables) — always wins",
                                    e.sources.len() + 1,
                                    e.local_keys.len()
                                );
                            }
                        }
                    }
                }
            }
        }
        Cmd::Set { vars } => {
            let t = project::resolve_target(env)?;
            let mut pairs = Vec::new();
            for v in vars {
                if v.contains('=') {
                    pairs.push(parse_kv(&v)?);
                } else {
                    let value = if std::io::stdin().is_terminal() {
                        prompt_password(&format!("{v}: "))?
                    } else {
                        let mut s = String::new();
                        std::io::stdin().read_to_string(&mut s)?;
                        s.trim_end_matches(['\n', '\r']).to_string()
                    };
                    pairs.push((v, value));
                }
            }
            let keys: Vec<String> = pairs.iter().map(|(k, _)| k.clone()).collect();
            c.call(Request::SecretSet {
                target: t.clone(),
                vars: pairs,
            })
            .await?;
            println!("set {} in {t}", keys.join(", "));
        }
        Cmd::Unset { keys } => {
            let t = project::resolve_target(env)?;
            c.call(Request::SecretUnset {
                target: t.clone(),
                keys: keys.clone(),
            })
            .await?;
            println!("removed {} from {t}", keys.join(", "));
        }
        Cmd::Get { key } => {
            let t = project::resolve_target(env)?;
            let r = c.resolve(&t, false).await?;
            match r.vars.iter().find(|v| v.key == key) {
                Some(v) => println!("{}", v.value),
                None => bail!("{key} is not set in {t}"),
            }
        }
        Cmd::Ls { show, refresh } => {
            let t = project::resolve_target(env)?;
            let r = c.resolve(&t, refresh).await?;
            println!("{t}{}", if r.protected { "  [protected]" } else { "" });
            if r.vars.is_empty() {
                println!(
                    "  (no variables — try `vaultwrap set KEY=value` or `vaultwrap import .env`)"
                );
            }
            let kw = r.vars.iter().map(|v| v.key.len()).max().unwrap_or(0);
            let sw = r.vars.iter().map(|v| v.source.len()).max().unwrap_or(0);
            let mut vars = r.vars.clone();
            vars.sort_by(|a, b| a.key.cmp(&b.key));
            for v in vars {
                let val = if show {
                    v.value.clone()
                } else {
                    mask(&v.value)
                };
                println!("  {:kw$}  {:sw$}  {}", v.key, v.source, val);
            }
        }
        Cmd::Import { file } => {
            let t = project::resolve_target(env)?;
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let vars = vaultwrap_core::providers::parse_dotenv(&text)?;
            let n = vars.len();
            c.call(Request::SecretSet {
                target: t.clone(),
                vars,
            })
            .await?;
            println!("imported {n} variables into {t}");
            println!(
                "  you can now delete {} — run with `vaultwrap run -- <cmd>`",
                file.display()
            );
        }
        Cmd::Export { format } => {
            let t = project::resolve_target(env)?;
            let r = c.resolve(&t, false).await?;
            match format {
                ExportFormat::Json => {
                    let m: Map<String, Value> = r
                        .vars
                        .into_iter()
                        .map(|v| (v.key, Value::String(v.value)))
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&m)?);
                }
                ExportFormat::Dotenv => {
                    for v in r.vars {
                        println!("{}={}", v.key, dotenv_quote(&v.value));
                    }
                }
                ExportFormat::Shell => {
                    let mut out = String::new();
                    for v in r.vars {
                        Shell::Bash.export(&mut out, &v.key, &v.value);
                    }
                    print!("{out}");
                }
            }
        }
        Cmd::Run { yes, command } => {
            let t = project::resolve_target(env)?;
            let r = c.resolve(&t, false).await?;
            guard_protected(&r, yes)?;
            drop(c);
            let mut cmd = std::process::Command::new(&command[0]);
            cmd.args(&command[1..])
                .envs(vars_of(&r))
                .env("VAULTWRAP_ACTIVE", t.to_string());
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                // Replace this process so signals and exit codes pass straight through.
                let err = cmd.exec();
                bail!("could not run '{}': {err}", command[0]);
            }
            #[cfg(not(unix))]
            {
                let status = cmd.status()?;
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Cmd::Shell { yes } => {
            let t = project::resolve_target(env)?;
            let r = c.resolve(&t, false).await?;
            guard_protected(&r, yes)?;
            let sh = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
            eprintln!(
                "vaultwrap: entering {t} ({} vars) — `exit` to leave",
                r.vars.len()
            );
            let status = std::process::Command::new(&sh)
                .envs(vars_of(&r))
                .env("VAULTWRAP_ACTIVE", t.to_string())
                .status()?;
            eprintln!("vaultwrap: left {t}");
            std::process::exit(status.code().unwrap_or(0));
        }
        Cmd::Activate { shell } => {
            if let Ok(a) = std::env::var("VAULTWRAP_ACTIVE") {
                bail!("{a} is already active in this shell — run `vaultwrap_deactivate` first");
            }
            let t = project::resolve_target(env)?;
            let r = c.resolve(&t, false).await?;
            guard_protected(&r, false)?;
            print!(
                "{}",
                shell::activate_script(
                    shell.unwrap_or_else(Shell::detect),
                    &t.to_string(),
                    &vars_of(&r)
                )
            );
            eprintln!(
                "vaultwrap: activated {t} ({} vars) — `vaultwrap_deactivate` to undo",
                r.vars.len()
            );
        }
        Cmd::Refresh => {
            c.call(Request::Refresh).await?;
            println!("cache cleared");
        }
        Cmd::Daemon { .. }
        | Cmd::Service(_)
        | Cmd::Hook { .. }
        | Cmd::HookEnv { .. }
        | Cmd::Link { .. } => {
            unreachable!()
        }
    }
    Ok(())
}

fn dotenv_quote(v: &str) -> String {
    if !v.is_empty()
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:@,+".contains(c))
    {
        v.to_string()
    } else if !v.contains('\'') {
        format!("'{v}'")
    } else {
        format!(
            "\"{}\"",
            v.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
        )
    }
}

fn fmt_opts(o: &Value) -> String {
    o.as_object()
        .map(|m| {
            m.iter()
                .map(|(k, v)| {
                    format!(
                        "{k}={}",
                        v.as_str().map(str::to_string).unwrap_or(v.to_string())
                    )
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn print_tree(tree: &[OrgNode], only_org: Option<&str>) {
    if tree.is_empty() {
        println!("(empty — start with `vaultwrap env add <org>/<project>/<env>`)");
    }
    for o in tree {
        if only_org.is_some_and(|x| x != o.name) {
            continue;
        }
        println!("{}", o.name);
        if !o.providers.is_empty() {
            println!("  providers");
            for p in &o.providers {
                println!("    {}  ({})  {}", p.name, p.kind, fmt_opts(&p.config));
            }
        }
        for p in &o.projects {
            println!("  {}", p.name);
            for e in &p.envs {
                let mut parts: Vec<String> = e
                    .sources
                    .iter()
                    .map(|s| {
                        let o = fmt_opts(&s.options);
                        if o.is_empty() {
                            s.provider.clone()
                        } else {
                            format!("{} [{o}]", s.provider)
                        }
                    })
                    .collect();
                if !e.local_keys.is_empty() || parts.is_empty() {
                    parts.push(format!("local: {} vars", e.local_keys.len()));
                }
                println!(
                    "    {}{}  ← {}",
                    e.name,
                    if e.protected { " [protected]" } else { "" },
                    parts.join(" + ")
                );
            }
        }
    }
}

async fn provider_cmd(c: &mut Conn, p: ProviderCmd, env: Option<&str>) -> Result<()> {
    match p {
        ProviderCmd::Add {
            name,
            kind,
            vault_url,
            mut auth,
            tenant,
            client_id,
            client_secret,
            path,
            command,
            format,
            ttl,
            extra,
            replace,
        } => {
            let (org, name) = split2(&name, "<org>/<name>")?;
            let mut cfg = Map::new();
            let mut put = |k: &str, v: Option<String>| {
                if let Some(v) = v {
                    cfg.insert(k.into(), Value::String(v));
                }
            };
            match kind {
                Kind::Azure => {
                    let url = vault_url
                        .ok_or_else(|| anyhow!("--vault-url is required for azure-keyvault"))?;
                    if !url.starts_with("https://") {
                        bail!("--vault-url should look like https://<name>.vault.azure.net");
                    }
                    if client_secret {
                        auth = Some("client-secret".into());
                    }
                    put("vault_url", Some(url));
                    put("tenant_id", tenant);
                    put("client_id", client_id);
                    if client_secret {
                        put(
                            "client_secret",
                            Some(prompt_password("Azure client secret: ")?),
                        );
                    }
                    put("auth", auth);
                }
                Kind::Dotenv => {
                    let path = path.ok_or_else(|| anyhow!("--path is required for dotenv"))?;
                    // Store absolute paths: the daemon doesn't share our working directory.
                    let abs = if path.starts_with('/') || path.starts_with('~') {
                        path
                    } else {
                        std::env::current_dir()?.join(path).display().to_string()
                    };
                    put("path", Some(abs));
                }
                Kind::Command => {
                    put(
                        "command",
                        Some(command.ok_or_else(|| anyhow!("--command is required for command"))?),
                    );
                    put("format", format);
                }
            }
            if let Some(ttl) = ttl {
                cfg.insert("cache_ttl".into(), ttl.into());
            }
            for kv in extra {
                let (k, v) = parse_kv(&kv)?;
                cfg.insert(k, kv_value(&v));
            }
            if ignore_exists(c.call(Request::OrgAdd { name: org.clone() }).await)? {
                println!("org {org}");
            }
            c.call(Request::ProviderAdd {
                org: org.clone(),
                name: name.clone(),
                kind: kind.as_str().into(),
                config: Value::Object(cfg),
                replace,
            })
            .await?;
            println!("provider {org}/{name} ({})", kind.as_str());
            println!("  test it:   vaultwrap provider test {org}/{name}");
            println!("  attach it: vaultwrap source add {name} -e {org}/<project>/<env>");
        }
        ProviderCmd::Rm { name } => {
            let (org, name) = split2(&name, "<org>/<name>")?;
            c.call(Request::ProviderRemove { org, name }).await?;
            println!("removed");
        }
        ProviderCmd::Ls { org } => {
            for o in c.tree().await? {
                if org.as_ref().is_some_and(|x| x != &o.name) {
                    continue;
                }
                for p in &o.providers {
                    println!(
                        "{}/{}  ({})  {}",
                        o.name,
                        p.name,
                        p.kind,
                        fmt_opts(&p.config)
                    );
                }
            }
        }
        ProviderCmd::Test { name, prefix } => {
            let (org, name) = split2(&name, "<org>/<name>")?;
            let target = project::resolve_target(env).ok().filter(|t| t.org == org);
            let options = prefix.map(|p| json!({ "prefix": p })).unwrap_or(json!({}));
            let r = c
                .call(Request::ProviderTest {
                    org,
                    name: name.clone(),
                    options,
                    target,
                })
                .await?;
            let keys: Vec<String> = serde_json::from_value(r["keys"].clone())?;
            println!("{name} returned {} variables", keys.len());
            for k in keys {
                println!("  {k}");
            }
        }
    }
    Ok(())
}

// ------------------------------------------------------------------ hook

/// Called by the shell hook on every prompt. Must be fast and quiet when
/// nothing changed, and must never prompt.
async fn hook_env(shell: Shell, env: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let desired = match project::find(&cwd) {
        Ok(Some(f)) if f.file.auto != Some(false) => match project::target_for(&f.file, env) {
            Ok(t) => Some((format!("{}|{}", f.dir.display(), t), t)),
            Err(_) => None,
        },
        _ => None,
    };
    let current = shell::current_state();
    let force = std::env::var_os("_VAULTWRAP_FORCE").is_some();
    let same =
        current.as_ref().map(|s| s.ctx.as_str()) == desired.as_ref().map(|(c, _)| c.as_str());
    if same && !force && !current.as_ref().is_some_and(|s| s.pending) {
        return Ok(());
    }

    let mut out = String::new();
    // Undo what we loaded before, computing the environment as it will be afterwards.
    let mut effective: std::collections::HashMap<String, Option<String>> = Default::default();
    if let Some(cur) = &current {
        for (k, prev) in &cur.prev {
            shell.restore(&mut out, k, prev);
            effective.insert(k.clone(), prev.clone());
        }
    }
    let Some((ctx, t)) = desired else {
        if current.is_some_and(|c| !c.prev.is_empty()) {
            eprintln!("vaultwrap: unloaded");
        }
        shell.unset(&mut out, shell::STATE_VAR);
        shell.unset(&mut out, "VAULTWRAP_ACTIVE");
        print!("{out}");
        return Ok(());
    };

    let was_pending = current.as_ref().is_some_and(|s| s.pending && s.ctx == ctx);
    let mut state = HookState {
        ctx: ctx.clone(),
        ..Default::default()
    };
    let result = async {
        let mut c = Conn::new(false).await?;
        c.resolve(&t, false).await
    }
    .await;
    match result {
        Ok(r) if r.protected => {
            eprintln!("vaultwrap: {t} is protected — not auto-loading (use `vaultwrap run` or `vaultwrap shell`)");
            shell.unset(&mut out, "VAULTWRAP_ACTIVE");
        }
        Ok(r) => {
            let mut keys: Vec<String> = r.vars.iter().map(|v| v.key.clone()).collect();
            keys.push("VAULTWRAP_ACTIVE".into());
            for k in &keys {
                let prev = effective
                    .get(k)
                    .cloned()
                    .unwrap_or_else(|| std::env::var(k).ok());
                state.prev.insert(k.clone(), prev);
            }
            for v in &r.vars {
                shell.export(&mut out, &v.key, &v.value);
            }
            shell.export(&mut out, "VAULTWRAP_ACTIVE", &t.to_string());
            eprintln!("vaultwrap: loaded {t} ({} vars)", r.vars.len());
        }
        Err(e) if e.downcast_ref::<Locked>().is_some() => {
            if !was_pending {
                eprintln!("vaultwrap: vault is locked — run `vaultwrap unlock` to load {t}");
            }
            state.pending = true;
            shell.unset(&mut out, "VAULTWRAP_ACTIVE");
        }
        Err(e) => {
            eprintln!("vaultwrap: could not load {t}: {e:#}");
            shell.unset(&mut out, "VAULTWRAP_ACTIVE");
        }
    }
    shell::write_state(shell, &mut out, &state)?;
    print!("{out}");
    Ok(())
}
