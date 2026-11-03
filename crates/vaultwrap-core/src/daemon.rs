//! The background daemon. It is the only process that opens the vault; it keeps
//! the decrypted connection in memory while unlocked, caches remote provider
//! values, and serves clients over a private Unix socket.

use crate::model::{Resolved, ResolvedVar, Target, Vars};
use crate::protocol::{ErrorCode, Request, Response, Status};
use crate::store::{NotFound, Store, WrongPassword};
use crate::vault::{self, Unlocked};
use crate::{keychain, paths, providers};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Notify;

pub struct DaemonOptions {
    /// Lock the vault after this many idle minutes (0 = never). Only applies
    /// when the vault key is not in the keychain.
    pub idle_lock_minutes: u64,
    /// Replace an already-running daemon instead of exiting. Used by the
    /// login service so it always owns the socket.
    pub takeover: bool,
}

impl Default for DaemonOptions {
    fn default() -> Self {
        DaemonOptions {
            idle_lock_minutes: 12 * 60,
            takeover: false,
        }
    }
}

struct State {
    vault: Mutex<Option<Unlocked>>,
    /// source id -> (fetched at, values)
    cache: Mutex<HashMap<i64, (Instant, Vars)>>,
    last_used: Mutex<Instant>,
    /// Locked on purpose (`vaultwrap lock`): don't silently re-unlock from the
    /// keychain until the user unlocks again.
    user_locked: AtomicBool,
    shutdown: Notify,
    opts: DaemonOptions,
}

macro_rules! log {
    ($($arg:tt)*) => { eprintln!("[vaultd] {}", format!($($arg)*)) };
}

pub async fn run(opts: DaemonOptions) -> Result<()> {
    augment_path();
    paths::ensure_home()?;
    let sock = paths::socket_path();
    if sock.exists() {
        if let Ok(mut existing) = crate::client::Client::connect().await {
            if !opts.takeover {
                log!("another daemon is already running at {}", sock.display());
                return Ok(());
            }
            log!("taking over from the running daemon");
            let _ = existing.call(&Request::Shutdown).await;
            for _ in 0..30 {
                if !sock.exists() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        std::fs::remove_file(&sock).ok();
    }
    let listener =
        UnixListener::bind(&sock).with_context(|| format!("binding {}", sock.display()))?;
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&sock, std::fs::Permissions::from_mode(0o600))?;
    }

    let state = Arc::new(State {
        vault: Mutex::new(None),
        cache: Mutex::new(HashMap::new()),
        last_used: Mutex::new(Instant::now()),
        user_locked: AtomicBool::new(false),
        shutdown: Notify::new(),
        opts,
    });
    try_keychain_unlock(&state);
    log!(
        "v{} listening on {} (pid {})",
        crate::VERSION,
        sock.display(),
        std::process::id()
    );

    // Idle auto-lock.
    let st = state.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(30));
        loop {
            tick.tick().await;
            let mins = st.opts.idle_lock_minutes;
            if mins == 0 || vault::key_in_keychain() {
                continue;
            }
            let idle = st.last_used.lock().unwrap().elapsed();
            if idle > Duration::from_secs(mins * 60) && st.vault.lock().unwrap().take().is_some() {
                st.cache.lock().unwrap().clear();
                log!("locked after {mins} idle minutes");
            }
        }
    });

    let uid = unsafe { libc::getuid() };
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = match accepted {
                    Ok(s) => s,
                    Err(e) => { log!("accept error: {e}"); continue; }
                };
                match stream.peer_cred() {
                    Ok(c) if c.uid() == uid => {}
                    _ => { log!("rejected connection from another user"); continue; }
                }
                let st = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = serve(st, stream).await {
                        log!("connection error: {e:#}");
                    }
                });
            }
            _ = state.shutdown.notified() => { log!("shutdown requested"); break; }
            _ = tokio::signal::ctrl_c() => { log!("interrupted"); break; }
            _ = sigterm.recv() => { log!("terminated"); break; }
        }
    }
    std::fs::remove_file(&sock).ok();
    Ok(())
}

fn try_keychain_unlock(state: &State) {
    if !vault::exists() {
        return;
    }
    match vault::open_with_keychain() {
        Some(Ok(u)) => {
            *state.vault.lock().unwrap() = Some(u);
            log!("unlocked with the key from the {}", keychain::describe());
        }
        Some(Err(e)) => log!("keychain key did not unlock the vault: {e:#}"),
        None => log!(
            "vault key not in the {} — waiting for the master password",
            keychain::describe()
        ),
    }
}

async fn serve(state: Arc<State>, stream: UnixStream) -> Result<()> {
    let (r, mut w) = stream.into_split();
    let mut lines = BufReader::new(r).lines();
    while let Some(line) = lines.next_line().await? {
        let resp = match serde_json::from_str::<Request>(&line) {
            Ok(req) => {
                *state.last_used.lock().unwrap() = Instant::now();
                handle(&state, req).await
            }
            Err(e) => Response::err(ErrorCode::Invalid, format!("bad request: {e}")),
        };
        let mut out = serde_json::to_vec(&resp)?;
        out.push(b'\n');
        w.write_all(&out).await?;
    }
    Ok(())
}

fn flatten<T>(r: std::result::Result<Result<T>, tokio::task::JoinError>) -> Result<T> {
    r.map_err(|e| anyhow::anyhow!("internal error: {e}"))?
}

fn classify(e: &anyhow::Error) -> ErrorCode {
    if e.downcast_ref::<WrongPassword>().is_some() {
        ErrorCode::WrongPassword
    } else if e.downcast_ref::<NotFound>().is_some() {
        ErrorCode::NotFound
    } else {
        ErrorCode::Invalid
    }
}

fn respond(r: Result<Value>) -> Response {
    match r {
        Ok(v) => Response::ok(v),
        Err(e) => Response::err(classify(&e), format!("{e:#}")),
    }
}

/// Run `f` against the unlocked store, unlocking from the keychain if allowed.
fn with_store<T>(
    state: &State,
    f: impl FnOnce(&Store) -> Result<T>,
) -> std::result::Result<T, Response> {
    let mut guard = state.vault.lock().unwrap();
    if guard.is_none() {
        if !vault::exists() {
            return Err(Response::err(
                ErrorCode::NoVault,
                "no vault yet — run `vaultwrap init`",
            ));
        }
        if !state.user_locked.load(Ordering::Relaxed) {
            *guard = vault::open_with_keychain().and_then(|r| r.ok());
        }
    }
    match guard.as_ref() {
        None => Err(Response::err(
            ErrorCode::Locked,
            "vault is locked — run `vaultwrap unlock`",
        )),
        Some(u) => f(&u.store).map_err(|e| Response::err(classify(&e), format!("{e:#}"))),
    }
}

/// Shorthand for store operations that return nothing interesting.
fn op(state: &State, f: impl FnOnce(&Store) -> Result<()>) -> Response {
    match with_store(state, f) {
        Ok(()) => Response::ok(json!({})),
        Err(r) => r,
    }
}

async fn handle(state: &State, req: Request) -> Response {
    match req {
        Request::Ping => Response::ok("pong"),
        Request::Status => {
            let s = Status {
                version: crate::VERSION.into(),
                pid: std::process::id(),
                home: paths::home().display().to_string(),
                vault_exists: vault::exists(),
                unlocked: state.vault.lock().unwrap().is_some(),
                user_locked: state.user_locked.load(Ordering::Relaxed),
                key_in_keychain: vault::key_in_keychain(),
                keychain: keychain::describe(),
                vault_id: vault::vault_id(),
                idle_lock_minutes: state.opts.idle_lock_minutes,
            };
            Response::ok(s)
        }
        Request::Init { password } => {
            let r = tokio::task::spawn_blocking(move || vault::create(&password)).await;
            match flatten(r) {
                Ok((u, warn)) => {
                    *state.vault.lock().unwrap() = Some(u);
                    state.user_locked.store(false, Ordering::Relaxed);
                    log!("vault created");
                    Response::ok(json!({ "warning": warn }))
                }
                Err(e) => Response::err(classify(&e), format!("{e:#}")),
            }
        }
        Request::Unlock { password } => {
            if !vault::exists() {
                return Response::err(ErrorCode::NoVault, "no vault yet — run `vaultwrap init`");
            }
            let r = tokio::task::spawn_blocking(move || vault::open_with_password(&password)).await;
            match flatten(r) {
                Ok((u, warn)) => {
                    *state.vault.lock().unwrap() = Some(u);
                    state.user_locked.store(false, Ordering::Relaxed);
                    log!("unlocked with the master password");
                    Response::ok(json!({ "warning": warn }))
                }
                Err(e) => Response::err(classify(&e), format!("{e:#}")),
            }
        }
        Request::UnlockKeychain => match vault::open_with_keychain() {
            Some(Ok(u)) => {
                *state.vault.lock().unwrap() = Some(u);
                state.user_locked.store(false, Ordering::Relaxed);
                log!("unlocked from the keychain");
                Response::ok(json!({}))
            }
            Some(Err(e)) => Response::err(ErrorCode::Invalid, format!("{e:#}")),
            None => Response::err(
                ErrorCode::NotFound,
                "the vault key is not in the keychain — unlock with your master password",
            ),
        },
        Request::VerifyPassword { password } => {
            let r = tokio::task::spawn_blocking(move || vault::verify_password(&password)).await;
            respond(flatten(r).map(|_| json!({})))
        }
        Request::Lock => {
            state.vault.lock().unwrap().take();
            state.cache.lock().unwrap().clear();
            state.user_locked.store(true, Ordering::Relaxed);
            log!("locked");
            Response::ok(json!({}))
        }
        Request::Forget => respond(vault::forget_keychain().map(|_| {
            log!("vault key removed from the keychain");
            json!({})
        })),
        Request::SetPassword { new_password } => {
            let guard = state.vault.lock().unwrap();
            match guard.as_ref() {
                None => Response::err(
                    ErrorCode::Locked,
                    "vault is locked — run `vaultwrap unlock`",
                ),
                Some(u) => respond(vault::set_password(u, &new_password).map(|_| {
                    log!("master password changed");
                    json!({})
                })),
            }
        }
        Request::EraseVault => {
            state.vault.lock().unwrap().take();
            state.cache.lock().unwrap().clear();
            state.user_locked.store(false, Ordering::Relaxed);
            respond(vault::erase().map(|backup| {
                log!("vault erased; backup at {}", backup.display());
                json!({ "backup": backup.display().to_string() })
            }))
        }
        Request::Tree => match with_store(state, |s| s.tree()) {
            Ok(t) => Response::ok(t),
            Err(r) => r,
        },
        Request::OrgAdd { name } => op(state, |s| s.org_add(&name)),
        Request::OrgRemove { name } => op(state, |s| s.org_remove(&name)),
        Request::ProviderAdd {
            org,
            name,
            kind,
            config,
            replace,
        } => {
            let r = op(state, |s| {
                s.provider_add(&org, &name, &kind, &config, replace)
            });
            state.cache.lock().unwrap().clear();
            r
        }
        Request::ProviderRemove { org, name } => op(state, |s| s.provider_remove(&org, &name)),
        Request::ProviderTest {
            org,
            name,
            options,
            target,
        } => {
            let p = match with_store(state, |s| s.provider(&org, &name)) {
                Ok(p) => p,
                Err(r) => return r,
            };
            let t = target.unwrap_or(Target {
                org: org.clone(),
                project: "_".into(),
                env: "_".into(),
            });
            match providers::fetch(&p.kind, &p.config, &options, &t).await {
                Ok(vars) => Response::ok(
                    json!({ "keys": vars.into_iter().map(|(k, _)| k).collect::<Vec<_>>() }),
                ),
                Err(e) => Response::err(ErrorCode::Provider, format!("{e:#}")),
            }
        }
        Request::ProjectAdd { org, name } => op(state, |s| s.project_add(&org, &name)),
        Request::ProjectRemove { org, name } => op(state, |s| s.project_remove(&org, &name)),
        Request::EnvAdd { target, protected } => op(state, |s| s.env_add(&target, protected)),
        Request::EnvRemove { target } => op(state, |s| s.env_remove(&target)),
        Request::EnvProtect { target, protected } => {
            op(state, |s| s.env_set_protected(&target, protected))
        }
        Request::EnvClone { from, to } => op(state, |s| s.env_clone(&from, &to)),
        Request::SourceAdd {
            target,
            provider,
            options,
        } => op(state, |s| s.source_add(&target, &provider, &options)),
        Request::SourceRemove { target, provider } => {
            op(state, |s| s.source_remove(&target, &provider))
        }
        Request::SecretSet { target, vars } => op(state, |s| {
            for (k, v) in &vars {
                s.secret_set(&target, k, v)?;
            }
            Ok(())
        }),
        Request::SecretUnset { target, keys } => op(state, |s| {
            for k in &keys {
                s.secret_unset(&target, k)?;
            }
            Ok(())
        }),
        Request::Resolve { target, refresh } => match resolve(state, &target, refresh).await {
            Ok(r) => Response::ok(r),
            Err(r) => r,
        },
        Request::Refresh => {
            state.cache.lock().unwrap().clear();
            Response::ok(json!({}))
        }
        Request::Shutdown => {
            state.shutdown.notify_one();
            Response::ok(json!({}))
        }
    }
}

async fn resolve(
    state: &State,
    t: &Target,
    refresh: bool,
) -> std::result::Result<Resolved, Response> {
    let (sources, locals, protected) = with_store(state, |s| {
        Ok((s.sources(t)?, s.secrets(t)?, s.env_protected(t)?))
    })?;

    let mut vars: Vec<ResolvedVar> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut warnings = Vec::new();
    let mut put = |key: String, value: String, source: &str| match index.get(&key) {
        Some(&i) => {
            vars[i] = ResolvedVar {
                key,
                value,
                source: source.into(),
            }
        }
        None => {
            index.insert(key.clone(), vars.len());
            vars.push(ResolvedVar {
                key,
                value,
                source: source.into(),
            });
        }
    };

    for src in &sources {
        let ttl = providers::cache_ttl(&src.provider.config);
        let cached = if refresh {
            None
        } else {
            state
                .cache
                .lock()
                .unwrap()
                .get(&src.id)
                .filter(|(at, _)| at.elapsed() < ttl)
                .map(|(_, v)| v.clone())
        };
        let values = match cached {
            Some(v) => v,
            None => {
                match providers::fetch(&src.provider.kind, &src.provider.config, &src.options, t)
                    .await
                {
                    Ok(v) => {
                        state
                            .cache
                            .lock()
                            .unwrap()
                            .insert(src.id, (Instant::now(), v.clone()));
                        let _ = with_store(state, |s| s.cache_put(src.id, &v));
                        v
                    }
                    Err(e) => {
                        // Fall back to the last values we fetched, if any.
                        match with_store(state, |s| s.cache_get(src.id)) {
                            Ok(Some((ts, v))) => {
                                warnings.push(format!(
                                    "{}: {e:#} — using values cached {}",
                                    src.provider.name,
                                    ago(ts)
                                ));
                                v
                            }
                            _ => {
                                return Err(Response::err(
                                    ErrorCode::Provider,
                                    format!("{}: {e:#}", src.provider.name),
                                ))
                            }
                        }
                    }
                }
            }
        };
        for (k, v) in values {
            put(k, v, &src.provider.name);
        }
    }
    for (k, v) in locals {
        put(k, v, "local");
    }
    Ok(Resolved {
        target: t.clone(),
        protected,
        vars,
        warnings,
    })
}

fn ago(ts: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(ts);
    let s = (now - ts).max(0);
    match s {
        0..=119 => format!("{s}s ago"),
        120..=7199 => format!("{}m ago", s / 60),
        7200..=172_799 => format!("{}h ago", s / 3600),
        _ => format!("{}d ago", s / 86400),
    }
}

/// Services start with a bare PATH (/usr/bin:/bin...), so tools like `az`,
/// `op` or `doppler` wouldn't be found. Merge in the user's login-shell PATH
/// and common install locations.
fn augment_path() {
    // Our own PATH first (it may be deliberately customized), then the login
    // shell's, then common install locations.
    let mut parts: Vec<String> = std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .map(str::to_string)
        .collect();
    let shell = std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(user_shell);
    if let Some(shell) = shell {
        if let Some(p) = login_shell_path(&shell) {
            parts.extend(p.split(':').map(str::to_string));
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    for extra in [
        "/opt/homebrew/bin",
        "/opt/homebrew/sbin",
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin",
    ] {
        parts.push(extra.into());
    }
    if !home.is_empty() {
        parts.push(format!("{home}/.local/bin"));
        parts.push(format!("{home}/.cargo/bin"));
    }
    let mut seen = std::collections::HashSet::new();
    let merged: Vec<String> = parts
        .into_iter()
        .filter(|p| !p.is_empty() && seen.insert(p.clone()))
        .collect();
    std::env::set_var("PATH", merged.join(":"));
}

fn user_shell() -> Option<String> {
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() || (*pw).pw_shell.is_null() {
            return None;
        }
        Some(
            std::ffi::CStr::from_ptr((*pw).pw_shell)
                .to_string_lossy()
                .into_owned(),
        )
    }
}

/// `$SHELL -l -c 'printf %s "$PATH"'`, giving up after 3 seconds.
fn login_shell_path(shell: &str) -> Option<String> {
    use std::process::{Command, Stdio};
    let mut child = Command::new(shell)
        .args(["-l", "-c", "printf %s \"$PATH\""])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(st)) if st.success() => break,
            Ok(Some(_)) => return None,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                return None;
            }
        }
    }
    let mut out = String::new();
    std::io::Read::read_to_string(&mut child.stdout.take()?, &mut out).ok()?;
    // Ignore anything a noisy profile printed before PATH.
    out.lines()
        .last()
        .map(str::to_string)
        .filter(|p| p.contains('/'))
}
