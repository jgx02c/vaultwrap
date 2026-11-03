//! Core of vaultwrap: the encrypted vault, secret providers, and the
//! daemon/client protocol. The CLI and (future) GUI are thin clients of the
//! daemon, which is the only process that opens the vault.

pub mod client;
pub mod crypto;
pub mod daemon;
pub mod keychain;
pub mod model;
pub mod paths;
pub mod protocol;
pub mod providers;
pub mod store;
pub mod system;
pub mod vault;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
