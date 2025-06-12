# VaultWrap CLI (Rust) — Project Scaffold & Structure

This document outlines the initial project structure and design for the new Rust-based `vaultwrap-cli` tool.

---

## Directory Structure

```
vaultwrap-cli/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── set.rs
│   │   ├── drop.rs
│   │   ├── env.rs
│   │   ├── key.rs
│   │   └── connections.rs
│   └── config.rs
└── README.md
```

---

## Main Dependencies
- [`clap`](https://crates.io/crates/clap) — for argument parsing and subcommands
- [`serde`](https://crates.io/crates/serde), [`serde_json`] or [`toml`] — for config file handling
- [`dirs`] — for finding home/config directories
- [`reqwest`] or [`ureq`] — for HTTP/TCP communication with the VaultWrap server

---

## Config File
- Location: `~/.vaultwrap/config.json` (or `.toml`)
- Stores saved connections, default connection, and user preferences
- Example:
```json
{
  "default": "acme",
  "connections": {
    "acme": { "host": "acme.example.com", "port": 4000, "username": "alice", "password": "..." },
    "local": { "host": "127.0.0.1", "port": 4000 }
  }
}
```

---

## Command Structure (clap)

- Top-level binary: `vaultwrap`
- Subcommands:
  - `connect`, `connections list`, `connections use`, `connections remove`, `connections show`
  - `env list`, `env add`, `env delete`, `env keys`
  - `set`, `drop`
  - `key add`, `key delete`, `key update`

### Example `main.rs` Skeleton
```rust
fn main() {
    let matches = clap::Command::new("vaultwrap")
        .about("VaultWrap CLI tool")
        .subcommand_required(true)
        .subcommand(
            clap::Command::new("set")
                .about("Set (activate) an environment")
                .arg(clap::arg!(<ENV> "Environment name")),
        )
        .subcommand(
            clap::Command::new("drop")
                .about("Drop (deactivate) the current environment"),
        )
        // ... other subcommands ...
        .get_matches();

    match matches.subcommand() {
        Some(("set", sub_m)) => { /* ... */ }
        Some(("drop", _)) => { /* ... */ }
        // ...
        _ => unreachable!(),
    }
}
```

---

## Shell Integration for `set`
- The `set` command outputs `export KEY=VALUE` lines for the selected environment.
- Usage:
  ```sh
  eval "$(vaultwrap set dev)"
  ```
- The `drop` command outputs `unset KEY` lines for the current environment.

---

## Next Steps
- Initialize the Rust project (`cargo new vaultwrap-cli`)
- Add dependencies to `Cargo.toml`
- Implement config file loading/saving
- Scaffold the main command structure
- Implement connection management and the `set`/`drop` commands first

---

This scaffold ensures the CLI will be installable, user-friendly, and ready for future expansion! 