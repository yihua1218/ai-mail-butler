mod ai;
mod config;
mod db;
mod firewall_agent;
mod mail;
mod models;
mod services;
mod smtp_security;
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
    #[arg(long)]
    firewall_config: Option<String>,
    #[arg(long, default_value = "health")]
    fw_action: String,
    #[arg(long)]
    socket: Option<String>,
    #[arg(long)]
    ip: Option<String>,
    #[arg(long)]
    duration: Option<String>,
    #[arg(long)]
    reason: Option<String>,
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
        PathBuf::from(
            path.file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("data.sqlite")),
        )
    } else {
        path.to_path_buf()
    }
}

async fn resolve_readonly_base_path(base: &Path, relative: &Path) -> PathBuf {
    let primary = base.join(relative);
    if tokio::fs::try_exists(&primary).await.unwrap_or(false) {
        return primary;
    }

    if let Ok(stripped) = relative.strip_prefix("data") {
        let data_root_path = base.join(stripped);
        if tokio::fs::try_exists(&data_root_path)
            .await
            .unwrap_or(false)
        {
            return data_root_path;
        }
    }

    primary
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
        resolve_readonly_base_path(&PathBuf::from(base), &relative_db_path).await
    } else {
        configured_db_path.clone()
    };

    if tokio::fs::try_exists(&base_db_path).await.unwrap_or(false)
        && !tokio::fs::try_exists(&overlay_db_path)
            .await
            .unwrap_or(false)
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

fn parse_mail_meta_field(meta: &str, field: &str) -> Option<String> {
    let prefix = format!("{}:", field);
    meta.lines()
        .find_map(|line| line.strip_prefix(&prefix).map(|v| v.trim().to_string()))
        .filter(|v| !v.is_empty())
}

fn parse_mail_timestamp(value: &str) -> Option<i64> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
        return Some(dt.timestamp());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.and_utc().timestamp());
    }
    None
}

fn sanitize_path_component(input: &str) -> String {
    let sanitized: String = input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '@' | '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();

    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}

async fn archived_content_size(dir: &Path) -> Option<(u64, String)> {
    let raw_path = dir.join("raw.eml");
    if let Ok(meta) = tokio::fs::metadata(&raw_path).await {
        if meta.len() > 0 {
            return Some((meta.len(), raw_path.to_string_lossy().to_string()));
        }
    }

    let body_path = dir.join("body.txt");
    if let Ok(meta) = tokio::fs::metadata(&body_path).await {
        if meta.len() > 0 {
            return Some((meta.len(), body_path.to_string_lossy().to_string()));
        }
    }

    None
}

async fn find_archived_mail_for_empty_row(
    config: &config::Config,
    runtime_spool_dir: &str,
    user_email: &str,
    subject: &str,
    received_at: Option<&str>,
) -> Option<(u64, String)> {
    let mut user_dirs =
        vec![PathBuf::from(runtime_spool_dir).join(sanitize_path_component(user_email))];
    if let Some(db_parent) = sqlite_url_to_path(&config.database_url).parent() {
        let sibling = db_parent
            .join("mail_spool")
            .join(sanitize_path_component(user_email));
        if !user_dirs.iter().any(|existing| existing == &sibling) {
            user_dirs.push(sibling);
        }
    }

    let target_ts = received_at.and_then(parse_mail_timestamp);
    let mut best: Option<(i64, PathBuf)> = None;
    for user_dir in user_dirs {
        let Ok(mut entries) = tokio::fs::read_dir(&user_dir).await else {
            continue;
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let meta = tokio::fs::read_to_string(path.join("meta.txt"))
                .await
                .unwrap_or_default();
            if parse_mail_meta_field(&meta, "subject").as_deref() != Some(subject) {
                continue;
            }

            let distance = match (
                target_ts,
                parse_mail_meta_field(&meta, "received_at").and_then(|v| parse_mail_timestamp(&v)),
            ) {
                (Some(target), Some(candidate)) => (target - candidate).abs(),
                _ => 0,
            };

            if best
                .as_ref()
                .map(|(best_distance, _)| distance < *best_distance)
                .unwrap_or(true)
            {
                best = Some((distance, path));
            }
        }
    }

    let (_, dir) = best?;
    archived_content_size(&dir).await
}

async fn list_empty_db_rows_with_archive(
    pool: &sqlx::SqlitePool,
    config: &config::Config,
    runtime_spool_dir: &str,
) -> Result<()> {
    let rows: Vec<(String, String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT e.id, u.email, COALESCE(e.subject, ''), CAST(e.received_at AS TEXT), e.status
         FROM emails e
         JOIN users u ON u.id = e.user_id
         WHERE length(coalesce(e.preview,'')) = 0
           AND length(coalesce(e.stored_content,'')) = 0
           AND length(coalesce(e.plain_content,'')) = 0
           AND length(coalesce(e.html_content,'')) = 0
         ORDER BY e.received_at DESC",
    )
    .fetch_all(pool)
    .await?;

    let mut found = 0usize;
    for (id, user_email, subject, received_at, status) in rows {
        if subject.trim().is_empty() {
            continue;
        }
        if let Some((size, path)) = find_archived_mail_for_empty_row(
            config,
            runtime_spool_dir,
            &user_email,
            &subject,
            received_at.as_deref(),
        )
        .await
        {
            found += 1;
            println!(
                "{} | {} | {} | {} bytes | {} | {}",
                id,
                status.unwrap_or_else(|| "-".to_string()),
                received_at.unwrap_or_else(|| "-".to_string()),
                size,
                subject,
                path
            );
        }
    }

    println!("Found {found} empty DB email rows with archived content.");
    Ok(())
}

#[cfg(not(test))]
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
                println!(
                    "  list-empty-archive    List DB-empty emails with archived raw/body content"
                );
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
                    let path =
                        mail::resolve_cli_target_path(&config, &runtime_spool_dir, target).await?;
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
                    let path =
                        mail::resolve_cli_target_path(&config, &runtime_spool_dir, target).await?;
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
            "list-empty-archive" => {
                list_empty_db_rows_with_archive(pool, &config, &runtime_spool_dir).await?;
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

#[cfg(not(test))]
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment variables
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args = CliArgs::parse();

    info!("Starting AI Mail Butler...");

    if args.mode.eq_ignore_ascii_case("firewall-agent") {
        let fw_config =
            firewall_agent::FirewallAgentConfig::load(args.firewall_config.as_deref()).await;
        firewall_agent::FirewallAgent::new(fw_config)
            .await
            .run()
            .await?;
        return Ok(());
    }

    if args.mode.eq_ignore_ascii_case("fw") {
        let socket_path = PathBuf::from(
            args.socket
                .clone()
                .unwrap_or_else(|| "/run/ai-mail-butler/firewall-agent.sock".to_string()),
        );
        let action = match args.fw_action.as_str() {
            "block" | "block_ip" => "block_ip",
            "unblock" | "unblock_ip" => "unblock_ip",
            "list" | "list_blocks" => "list_blocks",
            "health" => "health",
            other => other,
        }
        .to_string();
        let request = firewall_agent::FirewallRequest {
            action,
            ip: args.ip.clone(),
            duration: args.duration.clone(),
            reason: args
                .reason
                .clone()
                .or_else(|| Some("manual CLI request".to_string())),
            source: Some("ai-mail-butler-fw".to_string()),
        };
        let response = firewall_agent::send_request(&socket_path, &request).await?;
        println!("{}", serde_json::to_string_pretty(&response)?);
        return Ok(());
    }

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
    if let Some(saved_model) = sqlx::query_scalar::<_, String>(
        "SELECT value FROM app_settings WHERE key = 'ai_model_name'",
    )
    .fetch_optional(&pool)
    .await?
    .filter(|value| !value.trim().is_empty())
    {
        ai_client.set_model_name(saved_model).await;
    }

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
    let state = web::AppState {
        pool,
        ai_client,
        admin_email,
        developer_email,
        config: config_arc.clone(),
    };
    web::start_server(state.config.server_port, state).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(database_url: impl Into<String>) -> config::Config {
        config::Config {
            database_url: database_url.into(),
            server_port: 3000,
            ai_api_key: String::new(),
            developer_email: None,
            smtp_relay_host: None,
            smtp_relay_port: 587,
            smtp_relay_user: None,
            smtp_relay_pass: None,
            assistant_email: "assistant@example.com".to_string(),
            docs_whitelist: vec![],
            readonly_mode_enabled: false,
            readonly_block_writes: false,
            readonly_base: None,
            overlay_dir: None,
            remote_debug_sshfs_enabled: false,
            remote_debug_mode: "readonly".to_string(),
            remote_debug_access_mode: "readonly".to_string(),
            remote_debug_remote: None,
            remote_debug_mount_point: None,
            remote_debug_overlay_dir: None,
            cloudflare_zone_id: None,
            cloudflare_api_token: None,
        }
    }

    #[test]
    fn sqlite_url_to_path_handles_file_and_relative_urls() {
        assert_eq!(
            sqlite_url_to_path("sqlite:data/data.sqlite"),
            PathBuf::from("data/data.sqlite")
        );
        assert_eq!(
            sqlite_url_to_path("sqlite:/app/data/data.sqlite"),
            PathBuf::from("/app/data/data.sqlite")
        );
        assert_eq!(
            sqlite_url_to_path("sqlite://data/data.sqlite"),
            PathBuf::from("data/data.sqlite")
        );
    }

    #[test]
    fn resolve_runtime_dir_uses_overlay_in_readonly_mode() {
        let mut config = test_config("sqlite:data/data.sqlite");
        assert_eq!(
            resolve_runtime_dir(&config, "data/mail_spool"),
            "data/mail_spool"
        );

        config.readonly_mode_enabled = true;
        config.overlay_dir = Some("data/overlay".to_string());
        assert_eq!(
            resolve_runtime_dir(&config, "data/mail_spool"),
            "data/overlay/data/mail_spool"
        );
        assert_eq!(
            resolve_runtime_dir(&config, "data/overlay/data/mail_spool"),
            "data/overlay/data/mail_spool"
        );
        assert_eq!(
            resolve_runtime_dir(&config, "/app/data/mail_spool"),
            "data/overlay/app/data/mail_spool"
        );
    }

    #[test]
    fn parse_mail_meta_and_timestamp_are_conservative() {
        let meta = "subject: Hello\nreceived_at: 2026-05-12T10:20:30Z\nempty:   \n";
        assert_eq!(
            parse_mail_meta_field(meta, "subject").as_deref(),
            Some("Hello")
        );
        assert_eq!(parse_mail_meta_field(meta, "empty"), None);
        assert_eq!(parse_mail_meta_field(meta, "missing"), None);
        assert_eq!(
            parse_mail_timestamp("2026-05-12T10:20:30Z"),
            Some(1_778_581_230)
        );
        assert_eq!(
            parse_mail_timestamp("2026-05-12 10:20:30"),
            Some(1_778_581_230)
        );
        assert_eq!(parse_mail_timestamp("not a timestamp"), None);
    }

    #[test]
    fn sanitize_path_component_replaces_unsafe_chars() {
        assert_eq!(
            sanitize_path_component("alice+bob@example.com/path"),
            "alice_bob@example.com_path"
        );
        assert_eq!(sanitize_path_component(""), "unknown");
    }

    #[tokio::test]
    async fn archived_content_size_prefers_raw_then_body() {
        let root =
            std::env::temp_dir().join(format!("ai-mail-butler-main-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&root)
            .await
            .expect("create temp dir");

        tokio::fs::write(root.join("body.txt"), "body")
            .await
            .expect("write body");
        let body = archived_content_size(&root).await.expect("body fallback");
        assert_eq!(body.0, 4);
        assert!(body.1.ends_with("body.txt"));

        tokio::fs::write(root.join("raw.eml"), "raw mail")
            .await
            .expect("write raw");
        let raw = archived_content_size(&root).await.expect("raw preferred");
        assert_eq!(raw.0, 8);
        assert!(raw.1.ends_with("raw.eml"));

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn find_archived_mail_for_empty_row_picks_closest_received_at() {
        let root = std::env::temp_dir().join(format!(
            "ai-mail-butler-archive-test-{}",
            uuid::Uuid::new_v4()
        ));
        let user_dir = root.join("spool").join("user@example.com");
        let older = user_dir.join("older");
        let newer = user_dir.join("newer");
        tokio::fs::create_dir_all(&older)
            .await
            .expect("create older");
        tokio::fs::create_dir_all(&newer)
            .await
            .expect("create newer");
        tokio::fs::write(
            older.join("meta.txt"),
            "subject: Receipt\nreceived_at: 2026-05-12 09:00:00\n",
        )
        .await
        .expect("write older meta");
        tokio::fs::write(older.join("raw.eml"), "old")
            .await
            .expect("write old");
        tokio::fs::write(
            newer.join("meta.txt"),
            "subject: Receipt\nreceived_at: 2026-05-12 10:00:00\n",
        )
        .await
        .expect("write newer meta");
        tokio::fs::write(newer.join("raw.eml"), "newer")
            .await
            .expect("write newer");

        let config = test_config("sqlite::memory:");
        let found = find_archived_mail_for_empty_row(
            &config,
            root.join("spool").to_string_lossy().as_ref(),
            "user@example.com",
            "Receipt",
            Some("2026-05-12 09:55:00"),
        )
        .await
        .expect("find archived mail");

        assert_eq!(found.0, 5);
        assert!(found.1.contains("newer"));

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn prepare_readonly_overlay_db_copies_base_db_and_rewrites_config() {
        let root = std::env::temp_dir().join(format!(
            "ai-mail-butler-overlay-test-{}",
            uuid::Uuid::new_v4()
        ));
        let base = root.join("base");
        let overlay = root.join("overlay");
        let base_db = base.join("data.sqlite");
        tokio::fs::create_dir_all(&base).await.expect("create base");
        tokio::fs::write(&base_db, "sqlite bytes")
            .await
            .expect("write base db");

        let mut config = test_config("sqlite:/app/data/data.sqlite");
        config.readonly_mode_enabled = true;
        config.readonly_base = Some(base.to_string_lossy().to_string());
        config.overlay_dir = Some(overlay.to_string_lossy().to_string());

        prepare_readonly_overlay_db(&mut config)
            .await
            .expect("prepare overlay db");

        let overlay_db = overlay.join("data.sqlite");
        assert!(overlay_db.exists());
        assert_eq!(
            tokio::fs::read_to_string(&overlay_db)
                .await
                .expect("read overlay"),
            "sqlite bytes"
        );
        assert_eq!(
            config.database_url,
            format!("sqlite:{}", overlay_db.to_string_lossy())
        );
        assert_eq!(
            config.overlay_dir.as_deref(),
            Some(overlay.to_str().unwrap())
        );

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn readonly_base_resolution_supports_data_root_fallback_and_missing_base() {
        let root = std::env::temp_dir().join(format!(
            "ai-mail-butler-base-resolve-test-{}",
            uuid::Uuid::new_v4()
        ));
        let base = root.join("base");
        tokio::fs::create_dir_all(&base).await.expect("create base");
        tokio::fs::write(base.join("data.sqlite"), "db")
            .await
            .expect("write db");

        let resolved = resolve_readonly_base_path(&base, &PathBuf::from("data/data.sqlite")).await;
        assert_eq!(resolved, base.join("data.sqlite"));

        let missing = resolve_readonly_base_path(&base, &PathBuf::from("missing.sqlite")).await;
        assert_eq!(missing, base.join("missing.sqlite"));

        let mut config = test_config("sqlite:data/data.sqlite");
        prepare_readonly_overlay_db(&mut config)
            .await
            .expect("non-readonly is no-op");
        assert_eq!(config.database_url, "sqlite:data/data.sqlite");

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn archive_lookup_uses_db_sibling_and_skips_empty_or_mismatched_archives() {
        let root = std::env::temp_dir().join(format!(
            "ai-mail-butler-archive-sibling-test-{}",
            uuid::Uuid::new_v4()
        ));
        let data = root.join("data");
        let user_dir = data
            .join("mail_spool")
            .join(sanitize_path_component("user@example.test"));
        let empty = user_dir.join("empty");
        let wrong_subject = user_dir.join("wrong");
        let matched = user_dir.join("matched");
        tokio::fs::create_dir_all(&empty)
            .await
            .expect("create empty");
        tokio::fs::create_dir_all(&wrong_subject)
            .await
            .expect("create wrong");
        tokio::fs::create_dir_all(&matched)
            .await
            .expect("create matched");
        tokio::fs::write(
            empty.join("meta.txt"),
            "subject: Receipt\nreceived_at: 2026-05-13 10:00:00\n",
        )
        .await
        .expect("write empty meta");
        tokio::fs::write(empty.join("body.txt"), "")
            .await
            .expect("write empty body");
        tokio::fs::write(
            wrong_subject.join("meta.txt"),
            "subject: Newsletter\nreceived_at: 2026-05-13 10:00:00\n",
        )
        .await
        .expect("write wrong meta");
        tokio::fs::write(wrong_subject.join("raw.eml"), "wrong")
            .await
            .expect("write wrong raw");
        tokio::fs::write(
            matched.join("meta.txt"),
            "subject: Receipt\nreceived_at: 2026-05-13 10:05:00\n",
        )
        .await
        .expect("write matched meta");
        tokio::fs::write(matched.join("body.txt"), "body content")
            .await
            .expect("write matched body");

        let config = test_config(format!("sqlite:{}", data.join("data.sqlite").display()));
        let found = find_archived_mail_for_empty_row(
            &config,
            root.join("unrelated-spool").to_string_lossy().as_ref(),
            "user@example.test",
            "Receipt",
            Some("2026-05-13 10:05:00"),
        )
        .await
        .expect("find sibling archive");
        assert_eq!(found.0, 12);
        assert!(found.1.ends_with("body.txt"));

        assert!(find_archived_mail_for_empty_row(
            &config,
            root.join("unrelated-spool").to_string_lossy().as_ref(),
            "user@example.test",
            "Missing",
            None,
        )
        .await
        .is_none());

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn list_empty_db_rows_with_archive_handles_matches_and_nonmatches() {
        let root = std::env::temp_dir().join(format!(
            "ai-mail-butler-empty-rows-test-{}",
            uuid::Uuid::new_v4()
        ));
        let pool = db::connect("sqlite::memory:").await.expect("connect db");
        sqlx::query(
            "INSERT INTO users (id, email, is_onboarded, preferences)
             VALUES ('user-1', 'user@example.test', 1, '{}')",
        )
        .execute(&pool)
        .await
        .expect("insert user");
        sqlx::query(
            "INSERT INTO emails (id, user_id, subject, preview, stored_content, plain_content, html_content, status, received_at)
             VALUES
             ('email-1', 'user-1', 'Receipt', '', '', '', '', 'pending', '2026-05-13 10:00:00'),
             ('email-2', 'user-1', '', '', '', '', '', 'pending', '2026-05-13 10:00:00'),
             ('email-3', 'user-1', 'Has content', 'preview', '', '', '', 'processed', '2026-05-13 10:00:00')",
        )
        .execute(&pool)
        .await
        .expect("insert emails");

        let archive_dir = root
            .join("spool")
            .join(sanitize_path_component("user@example.test"))
            .join("receipt");
        tokio::fs::create_dir_all(&archive_dir)
            .await
            .expect("create archive");
        tokio::fs::write(
            archive_dir.join("meta.txt"),
            "subject: Receipt\nreceived_at: 2026-05-13 10:00:00\n",
        )
        .await
        .expect("write meta");
        tokio::fs::write(archive_dir.join("raw.eml"), "raw receipt")
            .await
            .expect("write raw");

        let config = test_config("sqlite::memory:");
        list_empty_db_rows_with_archive(
            &pool,
            &config,
            root.join("spool").to_string_lossy().as_ref(),
        )
        .await
        .expect("list empty rows");

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn write_cli_report_creates_parent_and_writes_json() {
        let root = std::env::temp_dir().join(format!(
            "ai-mail-butler-report-test-{}",
            uuid::Uuid::new_v4()
        ));
        let report_path = root.join("nested").join("report.json");
        let mut report = mail::CliRunReport::default();
        report.push_result(mail::CliFileResult {
            file_path: "mail.eml".to_string(),
            status: "processed".to_string(),
            message: "ok".to_string(),
            error_type: None,
            processed_path: Some("processed/mail.eml".to_string()),
            simulation_logs: vec!["step".to_string()],
        });

        write_cli_report(report_path.to_string_lossy().as_ref(), &report)
            .await
            .expect("write report");

        let content = tokio::fs::read_to_string(&report_path)
            .await
            .expect("read report");
        assert!(content.contains("\"processed\": 1"));
        assert!(content.contains("\"file_path\": \"mail.eml\""));

        let _ = tokio::fs::remove_dir_all(&root).await;
    }
}
