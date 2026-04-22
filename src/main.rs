mod ai;
mod config;
mod db;
mod mail;
mod models;
mod services;
mod web;

use anyhow::Result;
use clap::Parser;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::info;

#[derive(Debug, Parser)]
#[command(name = "ai-mail-butler")]
struct CliArgs {
    #[arg(long, default_value = "server")]
    mode: String,
    #[arg(long, default_value = "data/mail_spool")]
    spool_dir: String,
    #[arg(long)]
    eml_file: Option<String>,
    #[arg(long)]
    watch: bool,
    #[arg(long)]
    repl: bool,
    #[arg(long)]
    report_json: Option<String>,
    #[arg(long)]
    keep_files: bool,
    #[arg(long)]
    simulate_agent: bool,
    #[arg(long)]
    simulate_rules: bool,
    #[arg(long)]
    simulate_memory: bool,
    #[arg(long)]
    as_user: Option<String>,
    #[arg(long)]
    step: bool,
    #[arg(long)]
    readonly_mode: bool,
    #[arg(long)]
    readonly_base: Option<String>,
    #[arg(long)]
    overlay_dir: Option<String>,
}

fn sqlite_url_to_path(database_url: &str) -> PathBuf {
    PathBuf::from(
        database_url
            .trim_start_matches("sqlite:")
            .trim_start_matches("//"),
    )
}

fn resolve_overlay_relative_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        PathBuf::from(path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("data.sqlite")))
    } else {
        path.to_path_buf()
    }
}

fn resolve_runtime_dir(config: &config::Config, logical_dir: &str) -> String {
    if !config.readonly_mode_enabled {
        return logical_dir.to_string();
    }

    let overlay_root = config
        .overlay_dir
        .clone()
        .unwrap_or_else(|| "data/overlay".to_string());
    let overlay_root_path = PathBuf::from(&overlay_root);
    let logical_path = PathBuf::from(logical_dir);
    if logical_path.starts_with(&overlay_root_path) {
        return logical_path.to_string_lossy().to_string();
    }

    let relative = if logical_path.is_absolute() {
        logical_path
            .strip_prefix("/")
            .unwrap_or(&logical_path)
            .to_path_buf()
    } else {
        logical_path
    };

    overlay_root_path
        .join(relative)
        .to_string_lossy()
        .to_string()
}

async fn prepare_readonly_overlay_db(config: &mut config::Config) -> Result<()> {
    if !config.readonly_mode_enabled {
        return Ok(());
    }

    let overlay_root = PathBuf::from(
        config
            .overlay_dir
            .clone()
            .unwrap_or_else(|| "data/overlay".to_string()),
    );
    tokio::fs::create_dir_all(&overlay_root).await?;

    let configured_db_path = sqlite_url_to_path(&config.database_url);
    let relative_db_path = resolve_overlay_relative_path(&configured_db_path);
    let overlay_db_path = overlay_root.join(&relative_db_path);

    if let Some(parent) = overlay_db_path.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }

    let base_db_path = if let Some(base) = config.readonly_base.as_deref() {
        PathBuf::from(base).join(&relative_db_path)
    } else {
        configured_db_path.clone()
    };

    if tokio::fs::try_exists(&base_db_path).await.unwrap_or(false)
        && !tokio::fs::try_exists(&overlay_db_path).await.unwrap_or(false)
    {
        tokio::fs::copy(&base_db_path, &overlay_db_path).await?;
    }

    config.database_url = format!("sqlite:{}", overlay_db_path.to_string_lossy());
    config.overlay_dir = Some(overlay_root.to_string_lossy().to_string());

    info!(
        "Readonly overlay mode enabled. base_db='{}', overlay_db='{}'",
        base_db_path.display(),
        overlay_db_path.display()
    );

    Ok(())
}

async fn write_cli_report(path: &str, report: &mail::CliRunReport) -> Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }
    let content = serde_json::to_string_pretty(report)?;
    tokio::fs::write(path, content).await?;
    Ok(())
}

async fn run_cli_repl(
    pool: &sqlx::SqlitePool,
    ai_client: &ai::AiClient,
    config: Arc<config::Config>,
    args: &CliArgs,
) -> Result<()> {
    let mut aggregate = mail::CliRunReport::default();
    let runtime_spool_dir = resolve_runtime_dir(&config, &args.spool_dir);
    let runtime_processed_dir = format!("{}/processed", runtime_spool_dir);
    let process_options = mail::CliProcessOptions {
        keep_files: args.keep_files,
        simulate_agent: args.simulate_agent,
        simulate_rules: args.simulate_rules,
        simulate_memory: args.simulate_memory,
        as_user_email: args.as_user.clone(),
        step: args.step,
    };
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    println!("CLI REPL mode. Type 'help' for commands.");
    loop {
        print!("> ");
        use std::io::Write;
        let _ = std::io::stdout().flush();

        let Some(line) = lines.next_line().await? else {
            break;
        };
        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        let mut parts = input.split_whitespace();
        let cmd = parts.next().unwrap_or_default();
        match cmd {
            "help" => {
                println!("Commands:");
                println!("  list                  List pending .eml files");
                println!("  show <index|path>     Show first 40 lines of a spool file");
                println!("  process <index|path>  Process one spool file");
                println!("  retry-unknown         Requeue unknown_sender files from logs");
                println!("  report                Show aggregate report");
                println!("  exit                  Exit REPL");
            }
            "list" => {
                let files = mail::union_list_eml_files(&config, &runtime_spool_dir).await?;
                if files.is_empty() {
                    println!("No pending .eml files in {}", runtime_spool_dir);
                } else {
                    for (idx, file) in files.iter().enumerate() {
                        println!("[{idx}] {}", file.display());
                    }
                }
            }
            "show" => {
                if let Some(target) = parts.next() {
                    let path = mail::resolve_cli_target_path(&config, &runtime_spool_dir, target).await?;
                    let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
                    for line in content.lines().take(40) {
                        println!("{}", line);
                    }
                } else {
                    println!("Usage: show <index|path>");
                }
            }
            "process" => {
                if let Some(target) = parts.next() {
                    let path = mail::resolve_cli_target_path(&config, &runtime_spool_dir, target).await?;
                    let result = mail::MailService::process_single_spool_file(
                        pool,
                        ai_client,
                        config.clone(),
                        &runtime_spool_dir,
                        &runtime_processed_dir,
                        path,
                        &process_options,
                    )
                    .await;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                    aggregate.push_result(result);
                } else {
                    println!("Usage: process <index|path>");
                }
            }
            "retry-unknown" => {
                let count = mail::requeue_unknown_sender_errors(pool, &runtime_spool_dir).await?;
                println!("Requeued {count} files from unknown_sender logs");
            }
            "report" => {
                println!("{}", serde_json::to_string_pretty(&aggregate)?);
            }
            "exit" | "quit" => break,
            _ => println!("Unknown command: {cmd}. Type 'help'."),
        }
    }

    if let Some(path) = args.report_json.as_deref() {
        write_cli_report(path, &aggregate).await?;
        println!("Report written to {}", path);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment variables
    dotenvy::dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    let args = CliArgs::parse();

    info!("Starting AI Mail Butler...");

    let mut config = config::Config::load();

    if args.readonly_mode {
        config.readonly_mode_enabled = true;
    }
    if let Some(base) = args.readonly_base.clone() {
        config.readonly_base = Some(base);
    }
    if let Some(overlay) = args.overlay_dir.clone() {
        config.overlay_dir = Some(overlay);
    }
    prepare_readonly_overlay_db(&mut config).await?;

    // 1. Initialize Database
    let pool = db::connect(&config.database_url).await?;
    info!("Database connected successfully.");
    db::run_startup_diagnostics(&pool).await?;

    // 2. Initialize AI Client
    let ai_client = ai::AiClient::new(&config);

    let config_arc = Arc::new(config);

    if args.mode.eq_ignore_ascii_case("cli") {
        info!("Running in CLI mode (no SMTP/Web server)");

        if args.repl {
            run_cli_repl(&pool, &ai_client, config_arc.clone(), &args).await?;
            return Ok(());
        }

        let runtime_spool_dir = resolve_runtime_dir(&config_arc, &args.spool_dir);
        let runtime_processed_dir = format!("{}/processed", runtime_spool_dir);

        if args.watch {
            let process_options = mail::CliProcessOptions {
                keep_files: args.keep_files,
                simulate_agent: args.simulate_agent,
                simulate_rules: args.simulate_rules,
                simulate_memory: args.simulate_memory,
                as_user_email: args.as_user.clone(),
                step: args.step,
            };
            mail::MailService::process_spool_watch(
                pool,
                ai_client,
                config_arc,
                &runtime_spool_dir,
                &runtime_processed_dir,
                process_options,
            )
            .await;
            return Ok(());
        }

        let process_options = mail::CliProcessOptions {
            keep_files: args.keep_files,
            simulate_agent: args.simulate_agent,
            simulate_rules: args.simulate_rules,
            simulate_memory: args.simulate_memory,
            as_user_email: args.as_user.clone(),
            step: args.step,
        };

        if let Some(single_file) = args.eml_file.as_deref() {
            let single_path = std::path::PathBuf::from(single_file);
            let result = mail::MailService::process_single_spool_file(
                &pool,
                &ai_client,
                config_arc,
                &runtime_spool_dir,
                &runtime_processed_dir,
                single_path,
                &process_options,
            )
            .await;
            let mut report = mail::CliRunReport::default();
            report.push_result(result);
            println!("{}", serde_json::to_string_pretty(&report)?);
            if let Some(path) = args.report_json.as_deref() {
                write_cli_report(path, &report).await?;
                println!("Report written to {}", path);
            }
            return Ok(());
        }

        let report = mail::MailService::process_spool_once(
            &pool,
            &ai_client,
            config_arc,
            &runtime_spool_dir,
            &runtime_processed_dir,
            &process_options,
        )
        .await;

        println!("{}", serde_json::to_string_pretty(&report)?);
        if let Some(path) = args.report_json.as_deref() {
            write_cli_report(path, &report).await?;
            println!("Report written to {}", path);
        }
        return Ok(());
    }

    // 3. Start SMTP Server
    tokio::spawn({
        let pool = pool.clone();
        let ai_client = ai_client.clone();
        let config_clone = config_arc.clone();
        async move {
            if let Err(e) = mail::MailService::start(pool, ai_client, config_clone).await {
                tracing::error!("Mail server failed: {}", e);
            }
        }
    });

    // 4. Start Web Server
    let admin_email = std::env::var("ADMIN_EMAIL").ok();
    let developer_email = config_arc.developer_email.clone();
    let state = web::AppState { pool, ai_client, admin_email, developer_email, config: config_arc.clone() };
    web::start_server(state.config.server_port, state).await?;

    Ok(())
}
