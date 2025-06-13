mod config;
mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "vaultwrap")]
#[command(about = "VaultWrap CLI tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Set { env: String },
    Drop,
    Env {
        #[command(subcommand)]
        sub: EnvSub,
    },
    Key {
        #[command(subcommand)]
        sub: KeySub,
    },
    Connect {
        host: String,
        #[arg(long)]
        username: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(long)]
        save: Option<String>,
        #[arg(long)]
        default: bool,
    },
    Connections {
        #[command(subcommand)]
        sub: ConnectionsSub,
    },
}

#[derive(Subcommand)]
pub enum EnvSub {
    List,
    Add { env: String },
    Delete { env: String },
    Keys { env: String },
}

#[derive(Subcommand)]
pub enum KeySub {
    Add { env: String, key: String, value: String },
    Delete { env: String, key: String },
    Update { env: String, old_key: String, new_key: String, new_value: String },
}

#[derive(Subcommand)]
pub enum ConnectionsSub {
    List,
    Use { name: String },
    Remove { name: String },
    Show,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Set { env } => commands::set::run(env),
        Commands::Drop => commands::drop::run(),
        Commands::Env { sub } => commands::env::run(sub),
        Commands::Key { sub } => commands::key::run(sub),
        Commands::Connect { host, username, password, save, default } => {
            commands::connections::connect(host, username, password, save, default)
        }
        Commands::Connections { sub } => commands::connections::run(sub),
    }
}
