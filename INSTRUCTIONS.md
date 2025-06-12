# VaultWrap CLI — Usage & Installation Instructions

## 1. Running Locally (Development Mode)

From the CLI project directory (where `Cargo.toml` is):

```sh
cargo run -- <command> [args...]
```

**Examples:**
```sh
cargo run -- env list
cargo run -- set dev
cargo run -- key add dev API_KEY my-secret
```

---

## 2. Building a Release Binary

To build a standalone binary you can run from anywhere:

```sh
cargo build --release
```

- The binary will be at:
  `target/release/vaultwrap-cli`

You can run it directly:
```sh
./target/release/vaultwrap-cli env list
```

---

## 3. Installing Globally (for your user)

You can install the CLI to your `$HOME/.cargo/bin` (which should be in your PATH):

```sh
cargo install --path .
```

- This will make the `vaultwrap-cli` command available globally as `vaultwrap-cli`.

**(Optional) Rename for Convenience:**
- If you want the command to be just `vaultwrap`, rename the binary in `Cargo.toml`:
  ```toml
  [package]
  name = "vaultwrap"
  # ...
  ```
- Or, symlink it:
  ```sh
  ln -s ~/.cargo/bin/vaultwrap-cli ~/.cargo/bin/vaultwrap
  ```

---

## 4. Usage After Install

Now you can run:
```sh
vaultwrap env list
vaultwrap set dev
vaultwrap key add dev API_KEY my-secret
```

And so on, from any directory!

---

## 5. Uninstalling

To remove the CLI:
```sh
cargo uninstall vaultwrap-cli
```
(or `vaultwrap` if you renamed it)

---

## 6. For Distribution (Future)
- You can publish to crates.io for `cargo install vaultwrap`.
- Or create a Homebrew formula for `brew install vaultwrap`.
- Or package as a binary for other platforms.

---

**Need more help?**
- See the CLI source code and documentation for more details.
- Or ask for step-by-step help with packaging, distribution, or advanced usage! 