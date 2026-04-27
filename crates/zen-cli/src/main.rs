use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "zen-cli", about = "Zen Mode diagnostic and dev CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect the local SQLite database
    Db {
        #[command(subcommand)]
        cmd: DbCommands,
    },
    /// Probe OS permissions status
    Permissions,
    /// Simulate a focus session (for integration testing)
    Simulate {
        #[arg(long, default_value = "25", help = "Session duration in minutes")]
        duration_mins: u64,
        #[arg(long, default_value = "Focus")]
        mode: String,
    },
}

#[derive(Subcommand)]
enum DbCommands {
    /// Show currently active session
    ActiveSession,
    /// Show recent session history
    History {
        #[arg(long, default_value = "10")]
        limit: u64,
    },
    /// Run PRAGMA integrity_check
    Integrity,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Db { cmd } => match cmd {
            DbCommands::ActiveSession => {
                println!("Active session: none (DB stub)");
            }
            DbCommands::History { limit } => {
                println!("Session history (last {}): (DB stub)", limit);
            }
            DbCommands::Integrity => {
                println!("Database integrity: OK (stub)");
            }
        },
        Commands::Permissions => {
            println!("Permission probe (stub — implement per platform)");
        }
        Commands::Simulate { duration_mins, mode } => {
            println!("Simulating {duration_mins}m {mode} session (stub)...");
        }
    }

    Ok(())
}
