use anyhow::Result;
use tracing::{info, warn, error};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::net::SocketAddr;
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

fn first_email_address(raw: &str) -> Option<String> {
    if let Ok(parsed) = mailparse::addrparse(raw) {
        for addr in parsed.iter() {
            match addr {
                mailparse::MailAddr::Single(info) => {
                    if !info.addr.trim().is_empty() {
                        return Some(info.addr.trim().to_ascii_lowercase());
                    }
                }
                mailparse::MailAddr::Group(group) => {
                    for single in group.addrs.iter() {
                        if !single.addr.trim().is_empty() {
                            return Some(single.addr.trim().to_ascii_lowercase());
                        }
                    }
                }
            }
        }
    }

    // Fallback for non-standard address formats.
    if let Some((_, after_lt)) = raw.split_once('<') {
        if let Some((inside, _)) = after_lt.split_once('>') {
            let inside = inside.trim();
            if inside.contains('@') {
                return Some(inside.to_ascii_lowercase());
            }
        }
    }

    let trimmed = raw.trim().trim_matches('"').trim_matches('>').trim_matches('<');
    if trimmed.contains('@') {
        Some(trimmed.to_ascii_lowercase())
    } else {
        None
    }
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

fn extension_from_mime(mime: &str) -> &'static str {
    match mime {
        "application/pdf" => "pdf",
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "text/plain" => "txt",
        "text/html" => "html",
        "application/zip" => "zip",
        "application/msword" => "doc",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "application/vnd.ms-excel" => "xls",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        _ => "bin",
    }
}

fn collect_attachment_parts<'a>(
    part: &'a mailparse::ParsedMail<'a>,
    out: &mut Vec<&'a mailparse::ParsedMail<'a>>,
) {
    if !part.subparts.is_empty() {
        for sub in part.subparts.iter() {
            collect_attachment_parts(sub, out);
        }
        return;
    }

    let disposition = part.get_content_disposition();
    let has_filename = disposition.params.contains_key("filename")
        || part.ctype.params.contains_key("name");
    let is_attachment = matches!(
        disposition.disposition,
        mailparse::DispositionType::Attachment
    );

    if has_filename || is_attachment {
        out.push(part);
    }
}

fn collect_inline_text_parts<'a>(
    part: &'a mailparse::ParsedMail<'a>,
    out: &mut Vec<&'a mailparse::ParsedMail<'a>>,
) {
    if !part.subparts.is_empty() {
        for sub in part.subparts.iter() {
            collect_inline_text_parts(sub, out);
        }
        return;
    }

    let mime = part.ctype.mimetype.to_ascii_lowercase();
    if mime != "text/plain" && mime != "text/html" {
        return;
    }

    let disposition = part.get_content_disposition();
    if matches!(disposition.disposition, mailparse::DispositionType::Attachment) {
        return;
    }

    out.push(part);
}

fn infer_attachment_filename(part: &mailparse::ParsedMail<'_>, index: usize) -> String {
    let disposition = part.get_content_disposition();
    if let Some(name) = disposition.params.get("filename") {
        return sanitize_path_component(name);
    }
    if let Some(name) = part.ctype.params.get("name") {
        return sanitize_path_component(name);
    }

    let ext = extension_from_mime(&part.ctype.mimetype);
    format!("attachment_{index:02}.{ext}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_email_address_handles_common_formats() {
        assert_eq!(
            first_email_address("Alice <Alice.Example+Tag@Example.COM>"),
            Some("alice.example+tag@example.com".to_string())
        );
        assert_eq!(
            first_email_address("<bob@example.com>"),
            Some("bob@example.com".to_string())
        );
        assert_eq!(first_email_address("no-at-symbol"), None);
    }

    #[test]
    fn sanitize_path_component_replaces_unsafe_chars() {
        assert_eq!(sanitize_path_component("a/b c:?.pdf"), "a_b_c__.pdf");
        assert_eq!(sanitize_path_component("***"), "___");
    }

    #[test]
    fn extension_from_mime_maps_known_types() {
        assert_eq!(extension_from_mime("application/pdf"), "pdf");
        assert_eq!(extension_from_mime("text/html"), "html");
        assert_eq!(extension_from_mime("application/unknown"), "bin");
    }

    #[test]
    fn attachment_and_inline_part_collection_work() {
        let raw = concat!(
            "From: sender@example.com\r\n",
            "To: assistant@example.com\r\n",
            "Subject: test\r\n",
            "MIME-Version: 1.0\r\n",
            "Content-Type: multipart/mixed; boundary=\"b\"\r\n",
            "\r\n",
            "--b\r\n",
            "Content-Type: text/plain; charset=utf-8\r\n",
            "\r\n",
            "hello\r\n",
            "--b\r\n",
            "Content-Type: text/html; charset=utf-8\r\n",
            "\r\n",
            "<p>hello</p>\r\n",
            "--b\r\n",
            "Content-Type: application/pdf; name=\"r/eport.pdf\"\r\n",
            "Content-Disposition: attachment; filename=\"r/eport.pdf\"\r\n",
            "Content-Transfer-Encoding: base64\r\n",
            "\r\n",
            "SGVsbG8=\r\n",
            "--b--\r\n"
        );

        let parsed = mailparse::parse_mail(raw.as_bytes()).expect("parse mail");

        let mut attachments = Vec::new();
        collect_attachment_parts(&parsed, &mut attachments);
        assert_eq!(attachments.len(), 1);
        assert_eq!(infer_attachment_filename(attachments[0], 1), "r_eport.pdf");

        let mut inline_parts = Vec::new();
        collect_inline_text_parts(&parsed, &mut inline_parts);
        assert_eq!(inline_parts.len(), 2);
        assert!(inline_parts
            .iter()
            .any(|p| p.ctype.mimetype.eq_ignore_ascii_case("text/plain")));
        assert!(inline_parts
            .iter()
            .any(|p| p.ctype.mimetype.eq_ignore_ascii_case("text/html")));
    }
}

async fn log_mail_event(
    pool: &SqlitePool,
    level: &str,
    error_type: &str,
    msg: &str,
    context: Option<&str>,
    user_id: Option<&str>,
) {
    let _ = sqlx::query(
        "INSERT INTO mail_errors (level, error_type, message, context, user_id) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(level)
    .bind(error_type)
    .bind(msg)
    .bind(context)
    .bind(user_id)
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

        let listener = TcpListener::bind("0.0.0.0:25").await?;
        info!("SMTP server listening on 0.0.0.0:25  (spool: {})", spool_dir);

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
                                log_mail_event(&pool, "ERROR", "parse_error", &e.to_string(), path.to_str(), None).await;
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

                                let to_clean = first_email_address(&to_addr)
                                    .unwrap_or_else(|| to_addr.trim().to_ascii_lowercase());
                                let from_clean = first_email_address(&from_addr)
                                    .unwrap_or_else(|| from_addr.trim().to_ascii_lowercase());

                                // Forwarded email ownership is determined by the registered sender.
                                let user = sqlx::query_as::<_, User>(
                                    "SELECT * FROM users WHERE email = ?",
                                )
                                .bind(&from_clean)
                                .fetch_optional(&pool)
                                .await
                                .unwrap_or(None);

                                let passwords: Vec<String> = user
                                    .as_ref()
                                    .and_then(|u| u.pdf_passwords.as_ref())
                                    .and_then(|s| serde_json::from_str(s).ok())
                                    .unwrap_or_default();

                                let mut pdf_texts = Vec::new();

                                // Archive by sender because this assistant processes forwarded emails per sender.
                                let sender_key = if from_clean.is_empty() {
                                    "unknown_sender".to_string()
                                } else {
                                    sanitize_path_component(&from_clean)
                                };
                                let message_key = path
                                    .file_stem()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_else(|| format!("mail_{}", Utc::now().timestamp_millis()));
                                let message_dir = format!("{}/{}/{}", spool_dir, sender_key, message_key);
                                let attachments_dir = format!("{}/attachments", message_dir);
                                let decoded_parts_dir = format!("{}/decoded_parts", message_dir);

                                if let Err(e) = fs::create_dir_all(&attachments_dir).await {
                                    error!("Failed to create archive dir {}: {}", message_dir, e);
                                } else {
                                    let _ = fs::create_dir_all(&decoded_parts_dir).await;
                                    let _ = fs::write(format!("{}/raw.eml", message_dir), &contents).await;
                                    let _ = fs::write(format!("{}/body.txt", message_dir), body.as_bytes()).await;

                                    let metadata = format!(
                                        "from: {}\nto: {}\nsubject: {}\nreceived_at: {}\n",
                                        from_clean,
                                        to_clean,
                                        subject,
                                        Utc::now().to_rfc3339()
                                    );
                                    let _ = fs::write(format!("{}/meta.txt", message_dir), metadata).await;

                                    let mut attachment_parts = Vec::new();
                                    collect_attachment_parts(&parsed, &mut attachment_parts);
                                    for (idx, part) in attachment_parts.iter().enumerate() {
                                        if let Ok(data) = part.get_body_raw() {
                                            let filename = infer_attachment_filename(part, idx + 1);
                                            let attachment_path = format!(
                                                "{}/{}",
                                                attachments_dir,
                                                sanitize_path_component(&filename)
                                            );
                                            if let Err(e) = fs::write(&attachment_path, &data).await {
                                                error!("Failed to save attachment {}: {}", attachment_path, e);
                                            }

                                            if part.ctype.mimetype.eq_ignore_ascii_case("application/pdf") {
                                                match crate::services::OnboardingService::extract_pdf_text(&data, &passwords) {
                                                    Ok(text) => {
                                                        if !text.trim().is_empty() {
                                                            pdf_texts.push(text.clone());

                                                            let md_filename = if filename
                                                                .to_ascii_lowercase()
                                                                .ends_with(".pdf")
                                                            {
                                                                format!("{}.md", &filename[..filename.len() - 4])
                                                            } else {
                                                                format!("{}.md", filename)
                                                            };
                                                            let md_path = format!(
                                                                "{}/{}",
                                                                attachments_dir,
                                                                sanitize_path_component(&md_filename)
                                                            );

                                                            let md_content = format!(
                                                                "# Extracted PDF Content\n\nSource: {}\n\n{}",
                                                                filename,
                                                                text
                                                            );
                                                            if let Err(e) =
                                                                fs::write(&md_path, md_content.as_bytes()).await
                                                            {
                                                                error!(
                                                                    "Failed to save PDF markdown {}: {}",
                                                                    md_path, e
                                                                );
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        warn!(
                                                            "Failed to extract PDF text from {}: {}",
                                                            filename, e
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Decode inline text parts (including base64-encoded plain/html) and persist.
                                    let mut text_parts = Vec::new();
                                    collect_inline_text_parts(&parsed, &mut text_parts);
                                    for (idx, part) in text_parts.iter().enumerate() {
                                        match part.get_body() {
                                            Ok(decoded) => {
                                                if decoded.trim().is_empty() {
                                                    continue;
                                                }
                                                let ext = if part.ctype.mimetype.eq_ignore_ascii_case("text/html") {
                                                    "html"
                                                } else {
                                                    "txt"
                                                };
                                                let decoded_path = format!(
                                                    "{}/part_{:02}.{}",
                                                    decoded_parts_dir,
                                                    idx + 1,
                                                    ext
                                                );
                                                if let Err(e) = fs::write(&decoded_path, decoded.as_bytes()).await {
                                                    error!("Failed to save decoded text part {}: {}", decoded_path, e);
                                                }
                                            }
                                            Err(e) => {
                                                warn!("Failed to decode text MIME part: {}", e);
                                            }
                                        }
                                    }
                                }

                                if let Some(u) = user {
                                    let id = Uuid::new_v4().to_string();
                                    let preview = body.chars().take(100).collect::<String>();

                                    sqlx::query(
                                        "INSERT INTO emails (id, user_id, subject, preview, status) \
                                         VALUES (?, ?, ?, ?, 'pending')",
                                    )
                                    .bind(&id)
                                    .bind(&u.id)
                                    .bind(&subject)
                                    .bind(&preview)
                                    .execute(&pool)
                                    .await
                                    .unwrap_or_default();
                                    let pdf_context = if pdf_texts.is_empty() {
                                        None
                                    } else {
                                        Some(pdf_texts.join("\n---\n"))
                                    };

                                    let has_preference_rules = u
                                        .preferences
                                        .as_ref()
                                        .map(|p| !p.trim().is_empty())
                                        .unwrap_or(false);

                                    let enabled_rule_count: i64 = sqlx::query_scalar(
                                        "SELECT COUNT(*) FROM email_rules WHERE user_id = ? AND is_enabled = 1"
                                    )
                                    .bind(&u.id)
                                    .fetch_one(&pool)
                                    .await
                                    .unwrap_or(0);

                                    let has_rules = has_preference_rules || enabled_rule_count > 0;

                                    let enabled_rules: Vec<String> = sqlx::query_scalar(
                                        "SELECT rule_text FROM email_rules WHERE user_id = ? AND is_enabled = 1 ORDER BY updated_at DESC"
                                    )
                                    .bind(&u.id)
                                    .fetch_all(&pool)
                                    .await
                                    .unwrap_or_default();

                                    if has_rules {
                                        info!("Triggering AI for email {}", id);
                                        let memory = crate::services::OnboardingService::get_memory(
                                            &pool, &u.id,
                                        )
                                        .await;

                                        let mut ai_user = u.clone();
                                        if !enabled_rules.is_empty() {
                                            let rules_context = enabled_rules
                                                .iter()
                                                .map(|r| format!("- {}", r))
                                                .collect::<Vec<_>>()
                                                .join("\n");
                                            let merged = match ai_user.preferences.as_ref() {
                                                Some(p) if !p.trim().is_empty() => {
                                                    format!("{}\n\nActive email processing rules:\n{}", p, rules_context)
                                                }
                                                _ => format!("Active email processing rules:\n{}", rules_context),
                                            };
                                            ai_user.preferences = Some(merged);
                                        }

                                        match crate::services::OnboardingService::generate_reply(
                                            &ai_client,
                                            &ai_user,
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
                                                let user_id_for_memory = user_id.clone();
                                                let msg_clone = body.clone();
                                                let reply_clone = ai_reply.clone();
                                                tokio::spawn(async move {
                                                    let _ = crate::services::OnboardingService::update_memory(
                                                        &ai_client_clone,
                                                        &pool_clone,
                                                        &user_id_for_memory,
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
                                                        {
                                                            let preferred_language = u.preferred_language.as_str();
                                                            let assistant_name = if preferred_language == "zh-TW" {
                                                                u.assistant_name_zh.as_deref().unwrap_or("AI 郵件助理")
                                                            } else {
                                                                u.assistant_name_en.as_deref().unwrap_or("AI Mail Butler")
                                                            };
                                                            let assistant_mailbox = format!("{} <{}>", assistant_name, config.assistant_email);
                                                            mail_send::mail_builder::MessageBuilder::new()
                                                            .from(assistant_mailbox.clone())
                                                            .reply_to(assistant_mailbox)
                                                            .to(target_email
                                                                .replace('<', "")
                                                                .replace('>', ""))
                                                            .subject(format!("Re: {}", subject))
                                                            .text_body(ai_reply)
                                                        };

                                                    let is_implicit = port == 465;
                                                    let pool_clone = pool.clone();
                                                    let email_id = id.clone();
                                                    let user_id_for_log = user_id.clone();

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
                                                                    log_mail_event(
                                                                        &pool_clone,
                                                                        "ERROR",
                                                                        "smtp_send",
                                                                        &err_msg,
                                                                        Some(&email_id),
                                                                        Some(&user_id_for_log),
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
                                                                log_mail_event(
                                                                    &pool_clone,
                                                                    "ERROR",
                                                                    "smtp_connect",
                                                                    &err_msg,
                                                                    Some(&email_id),
                                                                    Some(&user_id_for_log),
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
                                                log_mail_event(
                                                    &pool,
                                                    "ERROR",
                                                    "ai_error",
                                                    &err_msg,
                                                    Some(&id),
                                                    Some(&u.id),
                                                )
                                                .await;
                                            }
                                        }
                                    } else {
                                        info!(
                                            "No processing rules for user {}; leaving email {} as pending",
                                            u.email, id
                                        );
                                    }
                                } else {
                                    let warn_msg = format!(
                                        "No user found for sender {:?} (recipient {:?}); email discarded",
                                        from_clean, to_clean
                                    );
                                    warn!(
                                        "{}",
                                        warn_msg
                                    );
                                    log_mail_event(
                                        &pool,
                                        "WARN",
                                        "unknown_sender",
                                        &warn_msg,
                                        path.to_str(),
                                        None,
                                    )
                                    .await;
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
