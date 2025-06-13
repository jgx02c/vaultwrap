mod config;
mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "vaultwrap")]
#[command(about = "VaultWrap CLI tool - Environment variable injection like Python venv", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Set environment variables (like 'source venv/bin/activate')
    Set { 
        env: String,
        /// Output shell commands instead of modifying current process
        #[arg(long)]
        shell_output: bool,
    },
    /// Drop environment variables (like 'deactivate')
    Drop {
        /// Output shell commands instead of modifying current process
        #[arg(long)]
        shell_output: bool,
    },
    /// Enable runtime command injection
    Enable,
    /// Disable runtime command injection
    Disable,
    /// Add a command to runtime injection list
    Add { command: String },
    /// Remove a command from runtime injection list
    Remove { command: String },
    /// Show runtime injection status
    Status,
    /// Environment management
    Env {
        #[command(subcommand)]
        sub: EnvSub,
    },
    /// Key management
    Key {
        #[command(subcommand)]
        sub: KeySub,
    },
    /// Connect to a VaultWrap server
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
    /// Connection management
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
        Commands::Set { env, shell_output } => commands::set::run(env, shell_output),
        Commands::Drop { shell_output } => commands::drop::run(shell_output),
        Commands::Enable => commands::runtime::enable(),
        Commands::Disable => commands::runtime::disable(),
        Commands::Add { command } => commands::runtime::add_command(command),
        Commands::Remove { command } => commands::runtime::remove_command(command),
        Commands::Status => commands::runtime::status(),
        Commands::Env { sub } => commands::env::run(sub),
        Commands::Key { sub } => commands::key::run(sub),
        Commands::Connect { host, username, password, save, default } => {
            commands::connections::connect(host, username, password, save, default)
        }
        Commands::Connections { sub } => commands::connections::run(sub),
    }
}
