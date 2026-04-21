use anyhow::Result;
use tracing::{info, warn, error};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::net::SocketAddr;
use mailparse::MailHeaderMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio::fs;
use crate::models::User;
use crate::ai::AiClient;
use sqlx::SqlitePool;
use uuid::Uuid;
use chrono::Utc;

use crate::config::Config;

pub struct MailService;

async fn log_mail_error(pool: &SqlitePool, error_type: &str, msg: &str, context: Option<&str>) {
    let _ = sqlx::query(
        "INSERT INTO mail_errors (error_type, message, context) VALUES (?, ?, ?)"
    )
    .bind(error_type)
    .bind(msg)
    .bind(context)
    .execute(pool)
    .await;
}

/// Handle a single inbound SMTP connection.
/// Saves a session transcript to `<spool_dir>/session_<id>.log` and the
/// received email body to `<spool_dir>/mail_<id>.eml`.
async fn handle_smtp_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
    spool_dir: String,
) {
    let session_id = format!(
        "{}-{}",
        Utc::now().format("%Y%m%d%H%M%S%.3f"),
        peer_addr.port()
    );

    info!("[SMTP {}] New connection from {}", session_id, peer_addr);

    let mut session_log = format!(
        "=== SMTP Session {} ===\nPeer: {}\nTime: {}\n\n",
        session_id,
        peer_addr,
        Utc::now().to_rfc3339()
    );

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Helper: send a response line, log it, bail on write error.
    macro_rules! send {
        ($resp:expr) => {{
            let r: String = $resp;
            session_log.push_str(&format!("S: {}", r));
            if let Err(e) = writer.write_all(r.as_bytes()).await {
                error!("[SMTP {}] Write error: {}", session_id, e);
                let log_path = format!("{}/session_{}.log", spool_dir, session_id);
                let _ = fs::write(&log_path, session_log.as_bytes()).await;
                return;
            }
        }};
    }

    // Server speaks first
    send!(format!("220 mail.local ESMTP AI Mail Butler\r\n"));

    let mut in_data = false;
    let mut email_data = String::new();

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                info!("[SMTP {}] Connection closed by peer", session_id);
                break;
            }
            Err(e) => {
                error!("[SMTP {}] Read error: {}", session_id, e);
                break;
            }
            Ok(_) => {}
        }

        session_log.push_str(&format!("C: {}", line));

        if in_data {
            // RFC 5321 end-of-data marker
            if line == ".\r\n" || line == ".\n" {
                in_data = false;
                let email_path = format!("{}/mail_{}.eml", spool_dir, session_id);
                match fs::write(&email_path, email_data.as_bytes()).await {
                    Ok(_) => info!("[SMTP {}] Email saved → {}", session_id, email_path),
                    Err(e) => error!("[SMTP {}] Failed to save email: {}", session_id, e),
                }
                send!(format!("250 OK: Message accepted for delivery\r\n"));
            } else {
                // Undo dot-stuffing (RFC 5321 §4.5.2)
                if line.starts_with("..") {
                    email_data.push_str(&line[1..]);
                } else {
                    email_data.push_str(&line);
                }
            }
        } else {
            let cmd_upper = line.trim_end().to_ascii_uppercase();

            let response = if cmd_upper.starts_with("EHLO") || cmd_upper.starts_with("HELO") {
                info!("[SMTP {}] {}", session_id, line.trim_end());
                format!(
                    "250-mail.local Hello {}\r\n250-SIZE 52428800\r\n250 OK\r\n",
                    peer_addr
                )
            } else if cmd_upper.starts_with("MAIL FROM") {
                info!("[SMTP {}] {}", session_id, line.trim_end());
                "250 OK\r\n".to_string()
            } else if cmd_upper.starts_with("RCPT TO") {
                info!("[SMTP {}] {}", session_id, line.trim_end());
                "250 OK\r\n".to_string()
            } else if cmd_upper.trim() == "DATA" {
                in_data = true;
                email_data.clear();
                info!("[SMTP {}] DATA phase started", session_id);
                "354 End data with <CR><LF>.<CR><LF>\r\n".to_string()
            } else if cmd_upper.trim() == "QUIT" {
                info!("[SMTP {}] QUIT", session_id);
                send!(format!("221 Bye\r\n"));
                break;
            } else if cmd_upper.trim() == "NOOP" {
                "250 OK\r\n".to_string()
            } else if cmd_upper.trim() == "RSET" {
                email_data.clear();
                "250 OK\r\n".to_string()
            } else {
                let cmd_name = line.split_whitespace().next().unwrap_or("?").to_string();
                warn!("[SMTP {}] Unknown command: {:?}", session_id, cmd_name);
                format!("502 Command not implemented: {}\r\n", cmd_name)
            };

            send!(response);
        }
    }

    // Always persist the session transcript
    let log_path = format!("{}/session_{}.log", spool_dir, session_id);
    match fs::write(&log_path, session_log.as_bytes()).await {
        Ok(_) => info!("[SMTP {}] Session log saved → {}", session_id, log_path),
        Err(e) => error!("[SMTP {}] Failed to save session log: {}", session_id, e),
    }
}

impl MailService {
    pub async fn start(pool: SqlitePool, ai_client: AiClient, config: Arc<Config>) -> Result<()> {
        let spool_dir = "data/mail_spool";
        let processed_dir = "data/mail_spool/processed";
        fs::create_dir_all(spool_dir).await?;
        fs::create_dir_all(processed_dir).await?;

        let listener = TcpListener::bind("0.0.0.0:2525").await?;
        info!("SMTP server listening on 0.0.0.0:2525  (spool: {})", spool_dir);

        let spool_owned = spool_dir.to_string();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        let spool = spool_owned.clone();
                        tokio::spawn(async move {
                            handle_smtp_connection(stream, peer_addr, spool).await;
                        });
                    }
                    Err(e) => {
                        error!("SMTP accept error: {}", e);
                    }
                }
            }
        });

        tokio::spawn(async move {
            Self::process_spool(pool, ai_client, config, spool_dir, processed_dir).await;
        });

        Ok(())
    }

    async fn process_spool(
        pool: SqlitePool,
        ai_client: AiClient,
        config: Arc<Config>,
        spool_dir: &str,
        processed_dir: &str,
    ) {
        info!("Spool processor watching {} every 3 s", spool_dir);
        loop {
            if let Ok(mut entries) = fs::read_dir(spool_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();

                    // Only process .eml files; skip subdirectories and .log files
                    let is_eml = path
                        .extension()
                        .map(|e| e == "eml")
                        .unwrap_or(false);
                    if !path.is_file() || !is_eml {
                        continue;
                    }

                    info!("Processing spool file: {}", path.display());

                    if let Ok(contents) = fs::read(&path).await {
                        match mailparse::parse_mail(&contents) {
                            Err(e) => {
                                error!("Failed to parse mail {:?}: {}", path.file_name(), e);
                                log_mail_error(&pool, "parse_error", &e.to_string(), path.to_str()).await;
                            }
                            Ok(parsed) => {
                                let subject = parsed.headers.iter()
                                    .find(|h| h.get_key().eq_ignore_ascii_case("Subject"))
                                    .map(|h| h.get_value())
                                    .unwrap_or_else(|| "No Subject".to_string());

                                let to_addr = parsed.headers.iter()
                                    .find(|h| h.get_key().eq_ignore_ascii_case("To"))
                                    .map(|h| h.get_value())
                                    .unwrap_or_default();

                                let from_addr = parsed.headers.iter()
                                    .find(|h| h.get_key().eq_ignore_ascii_case("From"))
                                    .map(|h| h.get_value())
                                    .unwrap_or_default();

                                let body = parsed.get_body().unwrap_or_default();

                                info!(
                                    "Parsed email from={:?} to={:?} subject={:?}",
                                    from_addr, to_addr, subject
                                );

                                // Strip angle brackets / whitespace from To
                                let to_clean = to_addr
                                    .trim()
                                    .trim_start_matches('<')
                                    .trim_end_matches('>')
                                    .to_string();

                                let user = sqlx::query_as::<_, User>(
                                    "SELECT * FROM users WHERE email = ?",
                                )
                                .bind(&to_clean)
                                .fetch_optional(&pool)
                                .await
                                .unwrap_or(None);

                                if let Some(u) = user {
                                    let id = Uuid::new_v4().to_string();
                                    let preview = body.chars().take(100).collect::<String>();

                                    sqlx::query(
                                        "INSERT INTO emails (id, user_id, subject, preview, status) \
                                         VALUES (?, ?, ?, ?, 'received')",
                                    )
                                    .bind(&id)
                                    .bind(&u.id)
                                    .bind(&subject)
                                    .bind(&preview)
                                    .execute(&pool)
                                    .await
                                    .unwrap_or_default();

                                    let mut pdf_texts = Vec::new();
                                    let passwords: Vec<String> = u
                                        .pdf_passwords
                                        .as_ref()
                                        .and_then(|s| serde_json::from_str(s).ok())
                                        .unwrap_or_default();

                                    for part in parsed.subparts.iter() {
                                        if part
                                            .get_headers()
                                            .get_first_header("Content-Type")
                                            .map(|h| h.get_value())
                                            .unwrap_or_default()
                                            .contains("application/pdf")
                                        {
                                            if let Ok(data) = part.get_body_raw() {
                                                if let Ok(text) =
                                                    crate::services::OnboardingService::extract_pdf_text(
                                                        &data, &passwords,
                                                    )
                                                {
                                                    pdf_texts.push(text);
                                                }
                                            }
                                        }
                                    }
                                    let pdf_context = if pdf_texts.is_empty() {
                                        None
                                    } else {
                                        Some(pdf_texts.join("\n---\n"))
                                    };

                                    if u.is_onboarded {
                                        info!("Triggering AI for email {}", id);
                                        let memory = crate::services::OnboardingService::get_memory(
                                            &pool, &u.id,
                                        )
                                        .await;
                                        match crate::services::OnboardingService::generate_reply(
                                            &ai_client,
                                            &u,
                                            &body,
                                            &memory,
                                            &config.assistant_email,
                                            pdf_context,
                                        )
                                        .await
                                        {
                                            Ok(res) => {
                                                let ai_reply = res.content;
                                                let ai_client_clone = ai_client.clone();
                                                let pool_clone = pool.clone();
                                                let user_id = u.id.clone();
                                                let msg_clone = body.clone();
                                                let reply_clone = ai_reply.clone();
                                                tokio::spawn(async move {
                                                    let _ = crate::services::OnboardingService::update_memory(
                                                        &ai_client_clone,
                                                        &pool_clone,
                                                        &user_id,
                                                        &msg_clone,
                                                        &reply_clone,
                                                    )
                                                    .await;
                                                });

                                                let target_email = if u.dry_run {
                                                    u.email.clone()
                                                } else {
                                                    from_addr.clone()
                                                };

                                                if u.dry_run || u.auto_reply {
                                                    info!(
                                                        "Sending AI reply to {} (dry_run={}, auto_reply={})",
                                                        target_email, u.dry_run, u.auto_reply
                                                    );
                                                    let host = config
                                                        .smtp_relay_host
                                                        .clone()
                                                        .unwrap_or_default();
                                                    let port = config.smtp_relay_port;
                                                    let smtp_user = config
                                                        .smtp_relay_user
                                                        .clone()
                                                        .unwrap_or_default();
                                                    let pass = config
                                                        .smtp_relay_pass
                                                        .clone()
                                                        .unwrap_or_default();

                                                    let message =
                                                        mail_send::mail_builder::MessageBuilder::new()
                                                            .from(config.assistant_email.clone())
                                                            .to(target_email
                                                                .replace('<', "")
                                                                .replace('>', ""))
                                                            .subject(format!("Re: {}", subject))
                                                            .text_body(ai_reply);

                                                    let is_implicit = port == 465;
                                                    let pool_clone = pool.clone();
                                                    let email_id = id.clone();

                                                    tokio::spawn(async move {
                                                        let mut builder =
                                                            mail_send::SmtpClientBuilder::new(
                                                                host.as_str(),
                                                                port,
                                                            );
                                                        builder =
                                                            builder.implicit_tls(is_implicit);
                                                        if !smtp_user.is_empty() {
                                                            builder = builder.credentials((
                                                                smtp_user.as_str(),
                                                                pass.as_str(),
                                                            ));
                                                        }
                                                        match builder.connect().await {
                                                            Ok(mut client) => {
                                                                if let Err(e) =
                                                                    client.send(message).await
                                                                {
                                                                    let err_msg =
                                                                        format!("{:?}", e);
                                                                    error!(
                                                                        "Failed to send AI reply: {}",
                                                                        err_msg
                                                                    );
                                                                    log_mail_error(
                                                                        &pool_clone,
                                                                        "smtp_send",
                                                                        &err_msg,
                                                                        Some(&email_id),
                                                                    )
                                                                    .await;
                                                                } else {
                                                                    sqlx::query(
                                                                        "UPDATE emails SET status = 'replied' WHERE id = ?",
                                                                    )
                                                                    .bind(&email_id)
                                                                    .execute(&pool_clone)
                                                                    .await
                                                                    .unwrap_or_default();
                                                                }
                                                            }
                                                            Err(e) => {
                                                                let err_msg = format!("{:?}", e);
                                                                error!(
                                                                    "Failed to connect for AI reply: {}",
                                                                    err_msg
                                                                );
                                                                log_mail_error(
                                                                    &pool_clone,
                                                                    "smtp_connect",
                                                                    &err_msg,
                                                                    Some(&email_id),
                                                                )
                                                                .await;
                                                            }
                                                        }
                                                    });
                                                } else {
                                                    info!(
                                                        "Skipping reply for {} (dry_run=false, auto_reply=false)",
                                                        id
                                                    );
                                                    sqlx::query(
                                                        "UPDATE emails SET status = 'drafted' WHERE id = ?",
                                                    )
                                                    .bind(&id)
                                                    .execute(&pool)
                                                    .await
                                                    .unwrap_or_default();
                                                }
                                            }
                                            Err(e) => {
                                                let err_msg = format!("{}", e);
                                                error!("AI reply failed: {}", err_msg);
                                                log_mail_error(
                                                    &pool,
                                                    "ai_error",
                                                    &err_msg,
                                                    Some(&id),
                                                )
                                                .await;
                                            }
                                        }
                                    }
                                } else {
                                    warn!(
                                        "No user found for recipient {:?}; email discarded",
                                        to_clean
                                    );
                                }
                            }
                        }
                    }

                    // Move processed file into processed/ so it's preserved for inspection
                    if let Some(fname) = path.file_name() {
                        let dest = format!("{}/{}", processed_dir, fname.to_string_lossy());
                        if let Err(e) = fs::rename(&path, &dest).await {
                            error!("Failed to move {:?} to processed/: {}", path, e);
                            let _ = fs::remove_file(&path).await;
                        }
                    }
                }
            }
            sleep(Duration::from_secs(3)).await;
        }
    }
}
