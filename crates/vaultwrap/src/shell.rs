//! Shell integration: `activate` (like a virtualenv), and the `hook`
//! that loads/unloads secrets automatically as you `cd` (like direnv).

use anyhow::Result;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Shell {
    Zsh,
    Bash,
    Fish,
}

impl Shell {
    pub fn detect() -> Shell {
        let sh = std::env::var("SHELL").unwrap_or_default();
        if sh.ends_with("fish") {
            Shell::Fish
        } else if sh.ends_with("bash") {
            Shell::Bash
        } else {
            Shell::Zsh
        }
    }

    pub fn quote(self, v: &str) -> String {
        match self {
            Shell::Fish => format!("'{}'", v.replace('\\', "\\\\").replace('\'', "\\'")),
            _ => format!("'{}'", v.replace('\'', "'\\''")),
        }
    }

    pub fn export(self, out: &mut String, k: &str, v: &str) {
        match self {
            Shell::Fish => writeln!(out, "set -gx {k} {};", self.quote(v)),
            _ => writeln!(out, "export {k}={};", self.quote(v)),
        }
        .unwrap();
    }

    pub fn unset(self, out: &mut String, k: &str) {
        match self {
            Shell::Fish => writeln!(out, "set -e {k};"),
            _ => writeln!(out, "unset {k};"),
        }
        .unwrap();
    }

    /// Restore a variable to its value before we touched it.
    pub fn restore(self, out: &mut String, k: &str, prev: &Option<String>) {
        match prev {
            Some(v) => self.export(out, k, v),
            None => self.unset(out, k),
        }
    }
}

/// Script for `eval "$(vaultwrap activate)"`. Defines `vaultwrap_deactivate`
/// (not `deactivate`, to stay clear of Python venvs), which puts back
/// whatever values the shell had before.
pub fn activate_script(shell: Shell, label: &str, vars: &[(String, String)]) -> String {
    let mut out = String::new();
    let mut undo = String::new();
    let mut keys: Vec<&str> = vars.iter().map(|(k, _)| k.as_str()).collect();
    keys.push("VAULTWRAP_ACTIVE");
    for k in &keys {
        shell.restore(&mut undo, k, &std::env::var(k).ok());
    }
    for (k, v) in vars {
        shell.export(&mut out, k, v);
    }
    shell.export(&mut out, "VAULTWRAP_ACTIVE", label);
    match shell {
        Shell::Fish => {
            writeln!(out, "functions -c fish_prompt _vaultwrap_old_fish_prompt;").unwrap();
            writeln!(
                out,
                "function fish_prompt; printf '(%s) ' {}; _vaultwrap_old_fish_prompt; end;",
                shell.quote(label)
            )
            .unwrap();
            writeln!(
                out,
                "function vaultwrap_deactivate;\n{undo}functions -e fish_prompt; functions -c _vaultwrap_old_fish_prompt fish_prompt; functions -e _vaultwrap_old_fish_prompt; functions -e vaultwrap_deactivate; end;"
            )
            .unwrap();
        }
        _ => {
            writeln!(
                out,
                "_VAULTWRAP_OLD_PS1=\"${{PS1-}}\"; PS1={}\"${{PS1-}}\";",
                shell.quote(&format!("({label}) "))
            )
            .unwrap();
            writeln!(
                out,
                "vaultwrap_deactivate() {{\n{undo}PS1=\"$_VAULTWRAP_OLD_PS1\"; unset _VAULTWRAP_OLD_PS1; unset -f vaultwrap_deactivate;\n}};"
            )
            .unwrap();
        }
    }
    out
}

pub fn hook_script(shell: Shell, exe: &str) -> String {
    let exe_q = shell.quote(exe);
    match shell {
        Shell::Zsh => format!(
            r#"_vaultwrap_hook() {{
  trap -- '' SIGINT
  eval "$({exe_q} hook-env --shell zsh)"
  trap - SIGINT
}}
vaultwrap_reload() {{ export _VAULTWRAP_FORCE=1; _vaultwrap_hook; unset _VAULTWRAP_FORCE; }}
typeset -ag precmd_functions
if (( ! ${{precmd_functions[(I)_vaultwrap_hook]}} )); then precmd_functions=(_vaultwrap_hook $precmd_functions); fi
typeset -ag chpwd_functions
if (( ! ${{chpwd_functions[(I)_vaultwrap_hook]}} )); then chpwd_functions=(_vaultwrap_hook $chpwd_functions); fi
"#
        ),
        Shell::Bash => format!(
            r#"_vaultwrap_hook() {{
  local previous_exit_status=$?
  trap -- '' SIGINT
  eval "$({exe_q} hook-env --shell bash)"
  trap - SIGINT
  return $previous_exit_status
}}
vaultwrap_reload() {{ export _VAULTWRAP_FORCE=1; _vaultwrap_hook; unset _VAULTWRAP_FORCE; }}
if [[ ";${{PROMPT_COMMAND[*]:-}};" != *";_vaultwrap_hook;"* ]]; then
  PROMPT_COMMAND="_vaultwrap_hook${{PROMPT_COMMAND:+;$PROMPT_COMMAND}}"
fi
"#
        ),
        Shell::Fish => format!(
            r#"function __vaultwrap_hook --on-event fish_prompt
    {exe_q} hook-env --shell fish | source
end
function vaultwrap_reload
    set -gx _VAULTWRAP_FORCE 1; __vaultwrap_hook; set -e _VAULTWRAP_FORCE
end
"#
        ),
    }
}

/// What the hook has loaded into this shell. Stored in `_VAULTWRAP_STATE`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HookState {
    /// "<project dir>|<org/project/env>"
    pub ctx: String,
    /// Values each loaded key had before we set it (None = was unset).
    #[serde(default)]
    pub prev: BTreeMap<String, Option<String>>,
    /// Waiting for the vault to be unlocked.
    #[serde(default)]
    pub pending: bool,
}

pub const STATE_VAR: &str = "_VAULTWRAP_STATE";

pub fn current_state() -> Option<HookState> {
    std::env::var(STATE_VAR)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

pub fn write_state(shell: Shell, out: &mut String, st: &HookState) -> Result<()> {
    shell.export(out, STATE_VAR, &serde_json::to_string(st)?);
    Ok(())
}
