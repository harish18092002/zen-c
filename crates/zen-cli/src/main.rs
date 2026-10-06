//! `zen-cli` — diagnostic command-line tool for the Zen Mode local database.
//!
//! All subcommands operate read-only against the same SQLite file the desktop
//! app uses (or one explicitly passed via `--db`). Useful for debugging, QA
//! audits, and triaging support tickets without launching the UI.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use sqlx::Row;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "zen-cli", about = "Zen Mode diagnostic CLI")]
struct Cli {
    /// Path to the Zen Mode SQLite database. Defaults to the OS-specific
    /// app data directory (matching the desktop app).
    #[arg(long, global = true)]
    db: Option<PathBuf>,

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
}

#[derive(Subcommand)]
enum DbCommands {
    /// Show currently active session (if any)
    ActiveSession,
    /// Show recent session history
    History {
        #[arg(long, default_value_t = 10)]
        limit: i64,
    },
    /// List configured profiles
    Profiles,
    /// Show recent tamper events
    Tamper {
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },
    /// Run PRAGMA integrity_check
    Integrity,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let db_path = resolve_db_path(cli.db)?;

    match cli.command {
        Commands::Db { cmd } => run_db(db_path, cmd).await?,
        Commands::Permissions => run_permissions(),
    }
    Ok(())
}

fn resolve_db_path(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    let dir = directories_path()?;
    Ok(dir.join("zen.db"))
}

#[cfg(target_os = "macos")]
fn directories_path() -> anyhow::Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("$HOME not set"))?;
    Ok(PathBuf::from(home).join("Library/Application Support/com.zenmode.app"))
}

#[cfg(target_os = "windows")]
fn directories_path() -> anyhow::Result<PathBuf> {
    let appdata = std::env::var("APPDATA").map_err(|_| anyhow::anyhow!("%APPDATA% not set"))?;
    Ok(PathBuf::from(appdata).join("com.zenmode.app"))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn directories_path() -> anyhow::Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("$HOME not set"))?;
    Ok(PathBuf::from(home).join(".config/zenmode"))
}

async fn run_db(db_path: PathBuf, cmd: DbCommands) -> anyhow::Result<()> {
    if !db_path.exists() {
        anyhow::bail!("database not found at {}", db_path.display());
    }
    let pool = zen_db::connection::connect_file(&db_path).await?;

    match cmd {
        DbCommands::ActiveSession => {
            let row = sqlx::query(
                "SELECT id, profile_id, mode, state, planned_duration_secs, created_at
                 FROM sessions
                 WHERE state IN ('Preparing','Active','Paused','Completing','Recovering')
                 ORDER BY created_at DESC LIMIT 1",
            )
            .fetch_optional(&pool)
            .await?;
            match row {
                None => println!("No active session."),
                Some(r) => print_session_row(&r)?,
            }
        }
        DbCommands::History { limit } => {
            let rows = sqlx::query(
                "SELECT id, profile_id, mode, state, planned_duration_secs, created_at
                 FROM sessions
                 ORDER BY created_at DESC LIMIT ?1",
            )
            .bind(limit)
            .fetch_all(&pool)
            .await?;
            if rows.is_empty() {
                println!("No sessions recorded yet.");
            }
            for r in &rows {
                print_session_row(r)?;
            }
        }
        DbCommands::Profiles => {
            let rows = sqlx::query(
                "SELECT id, name, revision, created_at, updated_at FROM profiles
                 ORDER BY created_at ASC",
            )
            .fetch_all(&pool)
            .await?;
            if rows.is_empty() {
                println!("No profiles configured.");
            }
            for r in &rows {
                let id: String = r.try_get("id")?;
                let name: String = r.try_get("name")?;
                let revision: i64 = r.try_get("revision")?;
                let rule_count: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM block_rules WHERE profile_id = ?1")
                        .bind(&id)
                        .fetch_one(&pool)
                        .await?;
                println!("{name:<30} rev={revision:<3} rules={rule_count:<4} id={id}");
            }
        }
        DbCommands::Tamper { limit } => {
            let rows = sqlx::query(
                "SELECT event_type, detail, detected_at FROM tamper_events
                 ORDER BY detected_at DESC LIMIT ?1",
            )
            .bind(limit)
            .fetch_all(&pool)
            .await?;
            if rows.is_empty() {
                println!("No tamper events.");
            }
            for r in &rows {
                let kind: String = r.try_get("event_type")?;
                let detail: Option<String> = r.try_get("detail")?;
                let at: String = r.try_get("detected_at")?;
                println!("{at:<25} {kind:<25} {}", detail.unwrap_or_default());
            }
        }
        DbCommands::Integrity => {
            let result: String = sqlx::query_scalar("PRAGMA integrity_check")
                .fetch_one(&pool)
                .await?;
            println!("Database integrity: {result}");
        }
    }
    Ok(())
}

fn print_session_row(r: &sqlx::sqlite::SqliteRow) -> anyhow::Result<()> {
    let id: String = r.try_get("id")?;
    let profile_id: String = r.try_get("profile_id")?;
    let mode: String = r.try_get("mode")?;
    let state: String = r.try_get("state")?;
    let planned: i64 = r.try_get("planned_duration_secs")?;
    let created: String = r.try_get("created_at")?;
    println!(
        "{created:<25} {state:<11} {mode:<6} planned={planned:<6}s profile={profile_id} id={id}"
    );
    Ok(())
}

fn run_permissions() {
    #[cfg(target_os = "macos")]
    {
        use zen_os::permissions::PermissionProbe;
        let probe = zen_macos_probe();
        let status = probe.check();
        println!("{}: {:?}", probe.name(), status);
    }
    #[cfg(target_os = "windows")]
    {
        use zen_os::permissions::PermissionProbe;
        let probe = zen_windows::permissions::ServicePermission;
        let status = probe.check();
        println!("{}: {:?}", probe.name(), status);
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        println!("No platform permission probes registered.");
    }
}

#[cfg(target_os = "macos")]
fn zen_macos_probe() -> impl zen_os::permissions::PermissionProbe {
    // We don't depend on zen-macos in zen-cli's Cargo.toml on purpose
    // (that would pull sysinfo etc into the CLI binary). For a no-op probe
    // wrapper, we re-implement the AX check inline.
    AccessibilityProbe
}

#[cfg(target_os = "macos")]
struct AccessibilityProbe;

#[cfg(target_os = "macos")]
impl zen_os::permissions::PermissionProbe for AccessibilityProbe {
    fn name(&self) -> &str {
        "Accessibility"
    }
    fn check(&self) -> zen_domain::entities::PermissionStatus {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrusted() -> u8;
        }
        if unsafe { AXIsProcessTrusted() } != 0 {
            zen_domain::entities::PermissionStatus::Healthy
        } else {
            zen_domain::entities::PermissionStatus::PermissionDenied
        }
    }
}
