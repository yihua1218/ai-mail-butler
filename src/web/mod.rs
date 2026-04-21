use axum::{
    extract::{State, Query},
    routing::{get, post},
    Json, Router,
};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::cors::CorsLayer;
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::info;
use anyhow::Result;
use tokio::fs;

use sqlx::SqlitePool;
use serde::{Deserialize, Serialize};

use crate::models::{User, EmailRecord};
use crate::ai::AiClient;
use crate::services::OnboardingService;

#[derive(sqlx::FromRow, Serialize)]
struct MailError {
    id: i64,
    level: String,
    error_type: String,
    message: String,
    context: Option<String>,
    user_id: Option<String>,
    user_email: Option<String>,
    occurred_at: String,
}

#[derive(sqlx::FromRow, Serialize)]
struct EmailRule {
    id: i64,
    user_id: String,
    rule_text: String,
    source: String,
    is_enabled: bool,
    created_at: Option<String>,
    updated_at: Option<String>,
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

fn looks_like_email_rule(message: &str) -> bool {
    let m = message.trim();
    if m.len() < 8 {
        return false;
    }

    let lower = m.to_lowercase();
    let keywords = [
        "轉寄", "請幫我", "遇到", "收到", "帳單", "通知", "發票", "提醒", "回覆", "處理",
        "forward", "when", "invoice", "bill", "receipt", "notify", "remind", "reply", "urgent", "important",
    ];
    keywords.iter().any(|k| lower.contains(k))
}

async fn capture_rule_from_chat(pool: &SqlitePool, user_id: &str, message: &str) {
    let cleaned = message.trim();
    if !looks_like_email_rule(cleaned) {
        return;
    }

    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM email_rules WHERE user_id = ? AND lower(rule_text) = lower(?) LIMIT 1"
    )
    .bind(user_id)
    .bind(cleaned)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if exists.is_some() {
        return;
    }

    let _ = sqlx::query(
        "INSERT INTO email_rules (user_id, rule_text, source, is_enabled) VALUES (?, ?, 'chat', 1)"
    )
    .bind(user_id)
    .bind(cleaned)
    .execute(pool)
    .await;
}

async fn get_user_id_by_email(pool: &SqlitePool, email: &str) -> Option<String> {
    sqlx::query_scalar("SELECT id FROM users WHERE email = ?")
        .bind(email)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct DataDeletionSnapshot {
    email_count: i64,
    rule_count: i64,
    log_count: i64,
    memory_count: i64,
    activity_row_count: i64,
    activity_event_total: i64,
    chat_log_count: i64,
    file_count: i64,
    total_file_bytes: i64,
}

#[derive(sqlx::FromRow)]
struct DataDeletionRequestRow {
    id: String,
    user_id: String,
    user_email: String,
    status: String,
    snapshot_json: String,
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

async fn collect_dir_stats(base_dir: &str) -> (i64, i64) {
    let mut total_files = 0_i64;
    let mut total_bytes = 0_i64;
    let mut stack = vec![PathBuf::from(base_dir)];

    while let Some(dir) = stack.pop() {
        let mut entries = match fs::read_dir(&dir).await {
            Ok(v) => v,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(meta) = entry.metadata().await {
                if meta.is_dir() {
                    stack.push(entry.path());
                } else if meta.is_file() {
                    total_files += 1;
                    total_bytes += meta.len() as i64;
                }
            }
        }
    }

    (total_files, total_bytes)
}

async fn build_user_data_snapshot(pool: &SqlitePool, user_id: &str, user_email: &str) -> DataDeletionSnapshot {
    let sender_dir = format!("data/mail_spool/{}", sanitize_path_component(&user_email.to_ascii_lowercase()));
    let (file_count, total_file_bytes) = collect_dir_stats(&sender_dir).await;

    let email_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM emails WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let rule_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM email_rules WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let log_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mail_errors WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let memory_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_memories WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let activity_row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_activity_stats WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let activity_event_total: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(count), 0) FROM user_activity_stats WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let chat_log_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chat_logs WHERE user_email = ?")
        .bind(user_email)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    DataDeletionSnapshot {
        email_count,
        rule_count,
        log_count,
        memory_count,
        activity_row_count,
        activity_event_total,
        chat_log_count,
        file_count,
        total_file_bytes,
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub ai_client: AiClient,
    pub admin_email: Option<String>,
    pub config: std::sync::Arc<crate::config::Config>,
}

#[derive(Deserialize)]
struct AuthQuery {
    email: Option<String>,
}

async fn get_me(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<Option<User>> {
    if let Some(email) = query.email {
        if email.is_empty() { return Json(None); }
        
        // Use UPSERT logic or separate check to handle concurrency/re-registrations
        let role = if Some(&email) == state.admin_email.as_ref() { "admin".to_string() } else { "user".to_string() };
        let new_id = uuid::Uuid::new_v4().to_string();
        
        let _ = sqlx::query("INSERT OR IGNORE INTO users (id, email) VALUES (?, ?)")
            .bind(&new_id)
            .bind(&email)
            .execute(&state.pool).await;

        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(&state.pool).await.unwrap_or(None);
            
        if let Some(mut u) = user { 
            u.role = role;
            return Json(Some(u)); 
        }
    }
    Json(None)
}

async fn get_dashboard(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<serde_json::Value> {
    let pool = &state.pool;
    
    // Always fetch global stats
    let users_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(pool).await.unwrap_or(0);
    let received_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM emails").fetch_one(pool).await.unwrap_or(0);
    let replied_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM emails WHERE status = 'replied'").fetch_one(pool).await.unwrap_or(0);
    let ai_replies_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chat_logs").fetch_one(pool).await.unwrap_or(0);
    
    let global_stats = serde_json::json!({
        "registered_users": users_count,
        "emails_received": received_count,
        "emails_replied": replied_count,
        "emails_sent": 0,
        "ai_replies": ai_replies_count,
    });
    
    if let Some(email) = query.email {
        if !email.is_empty() {
            let is_admin = Some(&email) == state.admin_email.as_ref();
            
            let user_id: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE email = ?")
                .bind(&email)
                .fetch_optional(pool).await.unwrap_or(None);
                
            if let Some((uid,)) = user_id {
                let personal_emails = sqlx::query_as::<_, EmailRecord>("SELECT id, subject, preview, status, CAST(received_at AS TEXT) as received_at FROM emails WHERE user_id = ? ORDER BY received_at DESC")
                    .bind(&uid)
                    .fetch_all(pool).await.unwrap_or(vec![]);
                    
                let p_received: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM emails WHERE user_id = ?").bind(&uid).fetch_one(pool).await.unwrap_or(0);
                let p_replied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM emails WHERE user_id = ? AND status = 'replied'").bind(&uid).fetch_one(pool).await.unwrap_or(0);
                
                let personal_stats = serde_json::json!({
                    "emails_received": p_received,
                    "emails_replied": p_replied,
                });

                if is_admin {
                    return Json(serde_json::json!({ 
                        "type": "admin", 
                        "global_stats": global_stats,
                        "personal_stats": personal_stats,
                        "personal_emails": personal_emails
                    }));
                } else {
                    return Json(serde_json::json!({ 
                        "type": "personal", 
                        "global_stats": global_stats,
                        "personal_stats": personal_stats,
                        "personal_emails": personal_emails
                    }));
                }
            }
        }
    }
    
    // Anonymous
    Json(serde_json::json!({
        "type": "anonymous",
        "global_stats": global_stats
    }))
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    email: String,
    guest_name: Option<String>,
}

async fn post_chat(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Json<serde_json::Value> {
    let email = payload.email;
    let message = payload.message;
    let guest_name = payload.guest_name;
    let pool = &state.pool;

    let chat_res = if !email.is_empty() {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(pool).await.unwrap_or(None);

        if let Some(mut user) = user {
            // Update preferences
            let new_pref = match OnboardingService::extract_preferences(&state.ai_client, &user, &message).await {
                Ok(p) => p,
                Err(_) => user.preferences.clone().unwrap_or_default()
            };
            sqlx::query("UPDATE users SET preferences = ?, is_onboarded = true WHERE id = ?")
                .bind(&new_pref).bind(&user.id).execute(pool).await.ok();
            user.preferences = Some(new_pref);

            let memory = OnboardingService::get_memory(pool, &user.id).await;
            let mut res = match OnboardingService::generate_reply(&state.ai_client, &user, &message, &memory, &state.config.assistant_email, None).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Failed to generate reply: {}", e);
                    crate::ai::ChatResult { content: "Sorry, I am having trouble connecting to my AI brain.".to_string(), total_tokens: 0, duration_ms: 0, finish_reason: None }
                }
            };

            // Detect repetitive questions (simple keyword for now)
            let lower_msg = message.to_lowercase();
            if lower_msg.contains("forward") || lower_msg.contains("email") || lower_msg.contains("信箱") || lower_msg.contains("轉寄") {
                let _ = OnboardingService::log_activity(pool, &user.id, "ask_forwarding_info").await;
            }

            capture_rule_from_chat(pool, &user.id, &message).await;

            // Append Onboarding Question if needed
            if user.onboarding_step < 3 {
                if let Some(question) = OnboardingService::get_next_onboarding_question(&user).await {
                    res.content = format!("{}\n\n---\n💡 [Onboarding] {}", res.content, question);
                }
                // Advance onboarding step
                sqlx::query("UPDATE users SET onboarding_step = onboarding_step + 1 WHERE id = ?")
                    .bind(&user.id).execute(pool).await.ok();
            }

            let ai_client_clone = state.ai_client.clone();
            let pool_clone = state.pool.clone();
            let user_id = user.id.clone();
            let msg_clone = message.clone();
            let reply_clone = res.content.clone();
            tokio::spawn(async move {
                let _ = OnboardingService::update_memory(&ai_client_clone, &pool_clone, &user_id, &msg_clone, &reply_clone).await;
            });

            res
        } else {
            OnboardingService::generate_anonymous_reply(&state.ai_client, &message, guest_name, &state.config.assistant_email).await
                .unwrap_or_else(|_| crate::ai::ChatResult { content: "Error connecting to AI.".to_string(), total_tokens: 0, duration_ms: 0, finish_reason: None })
        }
    } else {
        OnboardingService::generate_anonymous_reply(&state.ai_client, &message, guest_name, &state.config.assistant_email).await
            .unwrap_or_else(|_| crate::ai::ChatResult { content: "Error connecting to AI.".to_string(), total_tokens: 0, duration_ms: 0, finish_reason: None })
    };

    // Record this AI reply in chat_logs
    let log_email = if email.is_empty() { None } else { Some(email.clone()) };
    sqlx::query("INSERT INTO chat_logs (user_email) VALUES (?)")
        .bind(&log_email).execute(pool).await.ok();

    Json(serde_json::json!({ 
        "reply": chat_res.content, 
        "total_tokens": chat_res.total_tokens,
        "duration_ms": chat_res.duration_ms,
        "finish_reason": chat_res.finish_reason,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

#[derive(Serialize)]
struct BuildInfo {
    version: &'static str,
    target: &'static str,
    host: &'static str,
    profile: &'static str,
    git_commit: &'static str,
    build_date: &'static str,
    // Build-machine hardware fingerprint
    build_cpu_cores: &'static str,
    build_cpu_model: &'static str,
    build_ram: &'static str,
    build_disk: &'static str,
    assistant_email: String,
}

async fn get_about(State(state): State<AppState>) -> Json<BuildInfo> {
    Json(BuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        target: env!("BUILD_TARGET"),
        host: env!("BUILD_HOST"),
        profile: env!("BUILD_PROFILE"),
        git_commit: env!("GIT_COMMIT"),
        build_date: env!("BUILD_DATE"),
        build_cpu_cores: env!("BUILD_CPU_CORES"),
        build_cpu_model: env!("BUILD_CPU_MODEL"),
        build_ram: env!("BUILD_RAM"),
        build_disk: env!("BUILD_DISK"),
        assistant_email: state.config.assistant_email.clone(),
    })
}

#[derive(Deserialize)]
struct MagicLinkRequest {
    email: String,
}

use mail_send::{SmtpClientBuilder, mail_builder::MessageBuilder};
use lettre::message::{Message, SinglePart};
use lettre::{SmtpTransport, Transport};


async fn post_magic_link(
    State(state): State<AppState>,
    Json(payload): Json<MagicLinkRequest>,
) -> Json<serde_json::Value> {
    let email = payload.email.trim().to_lowercase();
    let token = uuid::Uuid::new_v4().to_string();

    // Use a true UPSERT to avoid UNIQUE constraint race conditions:
    // If email already exists, just update the token; otherwise insert.
    let new_id = uuid::Uuid::new_v4().to_string();
    let result = sqlx::query(
        "INSERT INTO users (id, email, magic_token) VALUES (?, ?, ?)
         ON CONFLICT(email) DO UPDATE SET magic_token = excluded.magic_token"
    )
    .bind(&new_id)
    .bind(&email)
    .bind(&token)
    .execute(&state.pool).await;

    if let Err(e) = result {
        tracing::error!("DB error during magic link upsert: {:?}", e);
        return Json(serde_json::json!({ "status": "error", "message": "Internal server error" }));
    }

    // Determine the public base URL for the login link
    let base_url = std::env::var("PUBLIC_URL")
        .unwrap_or_else(|_| format!("http://localhost:{}", state.config.server_port));
    let login_url = format!("{}/login?token={}", base_url, token);

    // ALWAYS log the magic link to terminal so it's usable even without SMTP
    let log_box = format!(
        "\n\
        ╔══════════════════════════════════════════════════════════════════════════════════════════╗\n\
        ║  MAGIC LOGIN LINK (debug)                                                                ║\n\
        ║  To : {:<82} ║\n\
        ║  URL: {:<82} ║\n\
        ╚══════════════════════════════════════════════════════════════════════════════════════════╝",
        email, login_url
    );
    tracing::info!("{}", log_box);

    let plain_text = format!(
        "Welcome to AI Mail Butler! / 歡迎使用 AI Mail Butler！\n\nClick the link below to securely login without a password. / 請點擊下方連結安全無密碼登入：\n{}",
        login_url
    );

    // Check if SMTP is configured with real values (not placeholder)
    let smtp_ready = state.config.smtp_relay_host.as_deref()
        .map(|h| !h.is_empty() && h != "smtp.your-server-address")
        .unwrap_or(false);

    let from_addr: lettre::message::Mailbox = state.config.assistant_email.parse().unwrap_or_else(|_| "noreply@example.com".parse().unwrap());
    let to_addr: lettre::message::Mailbox = email.parse().unwrap_or_else(|_| "noreply@example.com".parse().unwrap());
    let subject = "Your AI Mail Butler Login Link / 您的登入連結";
    if smtp_ready {
        let host = state.config.smtp_relay_host.as_ref().unwrap();
        let port = state.config.smtp_relay_port;
        let user = state.config.smtp_relay_user.clone().unwrap_or_default();
        let pass = state.config.smtp_relay_pass.clone().unwrap_or_default();

        let user_pref: String = sqlx::query_scalar("SELECT email_format FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(&state.pool).await
            .unwrap_or(Some("both".to_string()))
            .unwrap_or("both".to_string());

        let html_body = format!(
            r#"<!DOCTYPE html>
<html>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; padding: 40px; background-color: #f5f5f7; color: #1d1d1f;">
    <div style="max-width: 600px; margin: 0 auto; background-color: #ffffff; padding: 32px; border-radius: 20px; box-shadow: 0 4px 12px rgba(0,0,0,0.05);">
        <h1 style="font-size: 24px; font-weight: 600; margin-bottom: 16px;">AI Mail Butler</h1>
        <p style="font-size: 16px; line-height: 1.5; margin-bottom: 24px;">您好！請點擊下方的按鈕以登入您的 AI 郵件助理管理後台。</p>
        <div style="text-align: center; margin: 32px 0;">
            <a href="{}" style="display: inline-block; padding: 14px 32px; background-color: #007aff; color: #ffffff; text-decoration: none; border-radius: 12px; font-weight: 500; font-size: 16px;">登入管理後台</a>
        </div>
        <p style="font-size: 14px; color: #86868b; margin-top: 24px;">如果按鈕無法運作，請複製以下連結至瀏覽器：<br>
        <span style="word-break: break-all; color: #007aff;">{}</span></p>
        <hr style="border: none; border-top: 1px solid #d2d2d7; margin: 32px 0;">
        <p style="font-size: 12px; color: #86868b;">如果您沒有要求此連結，請忽略此郵件。為了您的帳號安全，請勿將此連結轉寄給他人。</p>
    </div>
</body>
</html>"#,
            login_url, login_url
        );

        // Build message using mail-send's builder
        let mut builder = MessageBuilder::new()
            .from(from_addr.to_string())
            .to(to_addr.to_string())
            .subject(subject);

        match user_pref.as_str() {
            "html" => { builder = builder.html_body(html_body); },
            "plain" => { builder = builder.text_body(plain_text.clone()); },
            _ => { 
                builder = builder.text_body(plain_text.clone()).html_body(html_body);
            }
        }
        
        let message = builder;

        let is_implicit = port == 465;
        tracing::debug!(">>> [SMTP] Connecting to {}:{} (Implicit TLS: {}, Format: {})", host, port, is_implicit, user_pref);

        let send_task = async move {
            let mut builder = SmtpClientBuilder::new(host.as_str(), port);
            builder = builder.implicit_tls(is_implicit);
            
            if !user.is_empty() {
                builder = builder.credentials((user.as_str(), pass.as_str()));
            }
            
            let mut client = builder.connect().await?;
            client.send(message).await
        };

        match send_task.await {
            Ok(_) => {
                tracing::info!("Magic link email sent successfully via mail-send to {}", email);
                Json(serde_json::json!({ "status": "success", "message": "Magic link sent to your email" }))
            }
            Err(e) => {
                let err_msg = format!("{:?}", e);
                tracing::error!("mail-send delivery failed: {}. Login URL is in server console.", err_msg);
                let user_id = get_user_id_by_email(&state.pool, &email).await;
                log_mail_event(&state.pool, "ERROR", "smtp_send", &err_msg, Some(&email), user_id.as_deref()).await;
                Json(serde_json::json!({ "status": "ok_debug", "message": "Email delivery failed; login URL logged to console" }))
            }
        }
    } else {
        // No relay configured — try direct MX delivery to the recipient's mail server.
        // This may land in spam but it works without any relay setup.
        let domain = email.split('@').nth(1).unwrap_or("").to_string();
        let mx_host = lookup_mx_host(&domain).await;

        if let Some(mx) = mx_host {
            tracing::info!("No SMTP relay configured. Attempting direct MX delivery to {} ({})", mx, domain);
            match Message::builder().from(from_addr).to(to_addr).subject(subject).singlepart(SinglePart::plain(plain_text)) {
                Ok(email_msg) => {
                    // Direct SMTP: connect to MX on port 25, STARTTLS if available
                    match SmtpTransport::relay(&mx) {
                        Ok(t_builder) => {
                            let mailer = t_builder.port(25).build();
                            match mailer.send(&email_msg) {
                                Ok(_) => {
                                    tracing::info!("Direct MX delivery succeeded to {} via {}", email, mx);
                                    return Json(serde_json::json!({ "status": "success", "message": "Magic link sent via direct delivery (may be in spam)" }));
                                }
                                Err(e) => {
                                    tracing::warn!("Direct MX delivery failed: {:?}. Login URL is in server console.", e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Could not build direct SMTP transport: {:?}", e);
                        }
                    }
                }
                Err(e) => tracing::warn!("Email build error for direct delivery: {:?}", e),
            }
        } else {
            tracing::warn!("Could not resolve MX for domain '{}'. Login URL is in server console.", domain);
        }

        Json(serde_json::json!({ "status": "ok_debug", "message": "No SMTP relay configured — login URL printed to server console" }))
    }
}

/// Look up the highest-priority MX record for a domain using `dig`.
/// Returns the MX hostname (without trailing dot) or None if lookup fails.
async fn lookup_mx_host(domain: &str) -> Option<String> {
    tracing::debug!("Looking up MX records for domain: {}", domain);
    // Try `dig +short MX {domain}` — available on Linux and macOS
    match tokio::process::Command::new("dig")
        .args(["+short", "MX", domain])
        .output()
        .await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.trim().is_empty() {
                    tracing::warn!("dig returned empty results for MX lookup of {}", domain);
                    return None;
                }
                parse_mx_records(&stdout)
            },
            Err(e) => {
                tracing::error!("Failed to execute 'dig' command: {}. Make sure 'dnsutils' or 'bind9-host' is installed.", e);
                None
            }
        }
}

/// Parse `dig +short MX` output lines like "10 alt1.gmail-smtp-in.l.google.com."
/// Returns the hostname of the lowest-priority (most preferred) MX record.
fn parse_mx_records(output: &str) -> Option<String> {
    let mut records: Vec<(u32, String)> = output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.trim().splitn(2, ' ').collect();
            if parts.len() == 2 {
                let priority: u32 = parts[0].parse().ok()?;
                let host = parts[1].trim_end_matches('.').to_string();
                if !host.is_empty() { Some((priority, host)) } else { None }
            } else {
                None
            }
        })
        .collect();

    records.sort_by_key(|(p, _)| *p);
    records.into_iter().next().map(|(_, host)| host)
}

#[derive(Deserialize)]
struct VerifyRequest {
    token: String,
}

async fn post_verify(
    State(state): State<AppState>,
    Json(payload): Json<VerifyRequest>,
) -> Json<Option<User>> {
    let token = payload.token.trim();
    tracing::info!(">>> [AUTH] Attempting to verify token: '{}'", token);
    
    // Debug: check total tokens in DB
    let total_tokens: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE magic_token IS NOT NULL")
        .fetch_one(&state.pool).await.unwrap_or(0);
    tracing::info!(">>> [AUTH] System current has {} active magic tokens in DB.", total_tokens);

    let query_result = sqlx::query_as::<_, User>("SELECT * FROM users WHERE magic_token = ?")
        .bind(token)
        .fetch_optional(&state.pool).await;
        
    match query_result {
        Ok(Some(mut u)) => {
            tracing::info!(">>> [AUTH] SUCCESS: Token match found for user: {}", u.email);
            // Clear token after use
            let clear_res = sqlx::query("UPDATE users SET magic_token = NULL WHERE id = ?")
                .bind(&u.id)
                .execute(&state.pool).await;
            
            if let Err(e) = clear_res {
                tracing::error!(">>> [AUTH] FAILED to clear token for user {}: {:?}", u.email, e);
            }
                
            u.magic_token = None;
            u.role = if Some(&u.email) == state.admin_email.as_ref() { "admin".to_string() } else { "user".to_string() };
            Json(Some(u))
        },
        Ok(None) => {
            tracing::warn!(">>> [AUTH] FAILED: No user found with token '{}'.", token);
            Json(None)
        },
        Err(e) => {
            tracing::error!(">>> [AUTH] DATABASE ERROR during verification: {:?}", e);
            Json(None)
        }
    }
}

#[derive(Deserialize)]
struct SettingsRequest {
    email: String,
    auto_reply: bool,
    dry_run: bool,
    email_format: String,
    timezone: Option<String>,
    display_name: Option<String>,
    assistant_name_zh: Option<String>,
    assistant_name_en: Option<String>,
    assistant_tone_zh: Option<String>,
    assistant_tone_en: Option<String>,
    pdf_passwords: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct CreateRuleRequest {
    email: String,
    rule_text: String,
}

#[derive(Deserialize)]
struct UpdateRuleRequest {
    email: String,
    id: i64,
    rule_text: String,
}

#[derive(Deserialize)]
struct ToggleRuleRequest {
    email: String,
    id: i64,
    is_enabled: bool,
}

#[derive(Deserialize)]
struct DataDeletionRequestPayload {
    email: String,
}

#[derive(Deserialize)]
struct DataDeletionSummaryQuery {
    token: String,
}

#[derive(Deserialize)]
struct DataDeletionConfirmPayload {
    token: String,
    confirm: bool,
}

async fn post_settings(
    State(state): State<AppState>,
    Json(payload): Json<SettingsRequest>,
) -> Json<serde_json::Value> {
    let pdf_passwords_json = payload.pdf_passwords.as_ref().and_then(|v| serde_json::to_string(v).ok());
    let timezone = payload
        .timezone
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("UTC")
        .to_string();
    
    let result = sqlx::query("UPDATE users SET auto_reply = ?, dry_run = ?, email_format = ?, timezone = ?, display_name = ?, \
                              assistant_name_zh = ?, assistant_name_en = ?, assistant_tone_zh = ?, assistant_tone_en = ?, pdf_passwords = ? WHERE email = ?")
        .bind(payload.auto_reply)
        .bind(payload.dry_run)
        .bind(&payload.email_format)
        .bind(&timezone)
        .bind(&payload.display_name)
        .bind(&payload.assistant_name_zh)
        .bind(&payload.assistant_name_en)
        .bind(&payload.assistant_tone_zh)
        .bind(&payload.assistant_tone_en)
        .bind(pdf_passwords_json)
        .bind(&payload.email)
        .execute(&state.pool).await;

        
    if result.is_ok() {
        let user_id = &payload.email; // Using email as user_id for stats for now
        let _ = OnboardingService::log_activity(&state.pool, user_id, "change_settings").await;
        if payload.auto_reply { let _ = OnboardingService::log_activity(&state.pool, user_id, "enable_auto_reply").await; }
        if payload.dry_run { let _ = OnboardingService::log_activity(&state.pool, user_id, "enable_dry_run").await; }
    }


        
    match result {
        Ok(_) => Json(serde_json::json!({ "status": "success" })),
        Err(e) => {
            tracing::error!("Failed to update settings: {}", e);
            Json(serde_json::json!({ "status": "error" }))
        }
    }
}

async fn get_admin_errors(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<serde_json::Value> {
    let is_admin = query.email.as_deref()
        .map(|e| Some(e) == state.admin_email.as_deref())
        .unwrap_or(false);

    if !is_admin {
        return Json(serde_json::json!({ "error": "Unauthorized" }));
    }

    let errors = sqlx::query_as::<_, MailError>(
        "SELECT m.id, m.level, m.error_type, m.message, m.context, m.user_id, u.email as user_email, CAST(m.occurred_at AS TEXT) as occurred_at \
         FROM mail_errors m \
         LEFT JOIN users u ON u.id = m.user_id \
         ORDER BY m.occurred_at DESC LIMIT 200"
    )
    .fetch_all(&state.pool).await.unwrap_or_default();

    Json(serde_json::json!({ "errors": errors }))
}

async fn get_user_errors(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<serde_json::Value> {
    let Some(email) = query.email else {
        return Json(serde_json::json!({ "error": "Missing email" }));
    };

    let Some(user_id) = get_user_id_by_email(&state.pool, &email).await else {
        return Json(serde_json::json!({ "error": "User not found" }));
    };

    let errors = sqlx::query_as::<_, MailError>(
        "SELECT m.id, m.level, m.error_type, m.message, m.context, m.user_id, u.email as user_email, CAST(m.occurred_at AS TEXT) as occurred_at \
         FROM mail_errors m \
         LEFT JOIN users u ON u.id = m.user_id \
         WHERE m.user_id = ? \
         ORDER BY m.occurred_at DESC LIMIT 200"
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({ "errors": errors }))
}

async fn get_rules(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<serde_json::Value> {
    let Some(email) = query.email else {
        return Json(serde_json::json!({ "status": "error", "message": "Missing email" }));
    };

    let Some(user_id) = get_user_id_by_email(&state.pool, &email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let rules = sqlx::query_as::<_, EmailRule>(
        "SELECT id, user_id, rule_text, source, is_enabled, CAST(created_at AS TEXT) as created_at, CAST(updated_at AS TEXT) as updated_at \
         FROM email_rules WHERE user_id = ? ORDER BY is_enabled DESC, updated_at DESC"
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({ "status": "success", "rules": rules }))
}

async fn post_create_rule(
    State(state): State<AppState>,
    Json(payload): Json<CreateRuleRequest>,
) -> Json<serde_json::Value> {
    let Some(user_id) = get_user_id_by_email(&state.pool, &payload.email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let rule_text = payload.rule_text.trim();
    if rule_text.is_empty() {
        return Json(serde_json::json!({ "status": "error", "message": "Rule cannot be empty" }));
    }

    let result = sqlx::query(
        "INSERT INTO email_rules (user_id, rule_text, source, is_enabled) VALUES (?, ?, 'manual', 1)"
    )
    .bind(&user_id)
    .bind(rule_text)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({ "status": "success" })),
        Err(e) => {
            tracing::error!("Failed to create rule: {}", e);
            Json(serde_json::json!({ "status": "error", "message": "Create failed" }))
        }
    }
}

async fn post_update_rule(
    State(state): State<AppState>,
    Json(payload): Json<UpdateRuleRequest>,
) -> Json<serde_json::Value> {
    let Some(user_id) = get_user_id_by_email(&state.pool, &payload.email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let rule_text = payload.rule_text.trim();
    if rule_text.is_empty() {
        return Json(serde_json::json!({ "status": "error", "message": "Rule cannot be empty" }));
    }

    let result = sqlx::query(
        "UPDATE email_rules SET rule_text = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?"
    )
    .bind(rule_text)
    .bind(payload.id)
    .bind(&user_id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({ "status": "success" })),
        Err(e) => {
            tracing::error!("Failed to update rule: {}", e);
            Json(serde_json::json!({ "status": "error", "message": "Update failed" }))
        }
    }
}

async fn post_toggle_rule(
    State(state): State<AppState>,
    Json(payload): Json<ToggleRuleRequest>,
) -> Json<serde_json::Value> {
    let Some(user_id) = get_user_id_by_email(&state.pool, &payload.email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let result = sqlx::query(
        "UPDATE email_rules SET is_enabled = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?"
    )
    .bind(payload.is_enabled)
    .bind(payload.id)
    .bind(&user_id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({ "status": "success" })),
        Err(e) => {
            tracing::error!("Failed to toggle rule: {}", e);
            Json(serde_json::json!({ "status": "error", "message": "Toggle failed" }))
        }
    }
}

async fn send_data_deletion_confirmation_email(
    state: &AppState,
    user_email: &str,
    snapshot: &DataDeletionSnapshot,
    confirm_url: &str,
) -> bool {
    let smtp_ready = state.config.smtp_relay_host.as_deref()
        .map(|h| !h.is_empty() && h != "smtp.your-server-address")
        .unwrap_or(false);

    if !smtp_ready {
        tracing::warn!("SMTP relay is not configured. Data deletion confirmation URL: {}", confirm_url);
        return false;
    }

    let host = state.config.smtp_relay_host.clone().unwrap_or_default();
    let port = state.config.smtp_relay_port;
    let smtp_user = state.config.smtp_relay_user.clone().unwrap_or_default();
    let pass = state.config.smtp_relay_pass.clone().unwrap_or_default();

    let text_body = format!(
        "You requested to delete all your data from AI Mail Butler.\n\nCurrent data snapshot:\n- Emails: {}\n- Rules: {}\n- Logs: {}\n- Memories: {}\n- Activity rows: {}\n- Activity event total: {}\n- Chat logs: {}\n- Files: {}\n- Total file size: {} bytes\n\nImportant: Cached or already-overwritten database pages may not be recoverable after deletion. The system can only assist with deletion and cannot restore deleted data.\n\nStep 1: Open this confirmation link to review your report again:\n{}\n\nAfter opening, you must still do a second final confirmation on the report page.",
        snapshot.email_count,
        snapshot.rule_count,
        snapshot.log_count,
        snapshot.memory_count,
        snapshot.activity_row_count,
        snapshot.activity_event_total,
        snapshot.chat_log_count,
        snapshot.file_count,
        snapshot.total_file_bytes,
        confirm_url,
    );

    let html_body = format!(
        "<h2>Data Deletion Confirmation</h2><p>You requested deletion of all your data.</p><ul><li>Emails: {}</li><li>Rules: {}</li><li>Logs: {}</li><li>Memories: {}</li><li>Activity rows: {}</li><li>Activity event total: {}</li><li>Chat logs: {}</li><li>Files: {}</li><li>Total file size: {} bytes</li></ul><p><strong>Important:</strong> Cached or overwritten database pages may not be recoverable after deletion. The system can only assist with deletion and cannot restore deleted data.</p><p><a href=\"{}\">Open confirmation report</a></p><p>After opening the report, you still need a second final confirmation to delete.</p>",
        snapshot.email_count,
        snapshot.rule_count,
        snapshot.log_count,
        snapshot.memory_count,
        snapshot.activity_row_count,
        snapshot.activity_event_total,
        snapshot.chat_log_count,
        snapshot.file_count,
        snapshot.total_file_bytes,
        confirm_url,
    );

    let message = MessageBuilder::new()
        .from(state.config.assistant_email.clone())
        .to(user_email.to_string())
        .subject("AI Mail Butler - Confirm Your Data Deletion Request")
        .text_body(text_body)
        .html_body(html_body);

    let is_implicit = port == 465;
    let mut builder = SmtpClientBuilder::new(host.as_str(), port).implicit_tls(is_implicit);
    if !smtp_user.is_empty() {
        builder = builder.credentials((smtp_user.as_str(), pass.as_str()));
    }

    match builder.connect().await {
        Ok(mut client) => client.send(message).await.is_ok(),
        Err(e) => {
            tracing::error!("Failed to connect SMTP for data deletion confirmation: {:?}", e);
            false
        }
    }
}

async fn post_request_data_deletion(
    State(state): State<AppState>,
    Json(payload): Json<DataDeletionRequestPayload>,
) -> Json<serde_json::Value> {
    let email = payload.email.trim().to_ascii_lowercase();
    let user_row: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None);

    let Some((user_id,)) = user_row else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let snapshot = build_user_data_snapshot(&state.pool, &user_id, &email).await;
    let snapshot_json = serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".to_string());
    let token = uuid::Uuid::new_v4().to_string();
    let req_id = uuid::Uuid::new_v4().to_string();

    let _ = sqlx::query(
        "INSERT INTO data_deletion_requests (id, user_id, token, status, snapshot_json) VALUES (?, ?, ?, 'requested', ?)"
    )
    .bind(&req_id)
    .bind(&user_id)
    .bind(&token)
    .bind(&snapshot_json)
    .execute(&state.pool)
    .await;

    let base_url = std::env::var("PUBLIC_URL")
        .unwrap_or_else(|_| format!("http://localhost:{}", state.config.server_port));
    let confirm_url = format!("{}/gdpr-delete?token={}", base_url, token);

    let delivered = send_data_deletion_confirmation_email(&state, &email, &snapshot, &confirm_url).await;
    if !delivered {
        let msg = format!("Failed to deliver data deletion confirmation email to {}", email);
        log_mail_event(&state.pool, "ERROR", "gdpr_email_send", &msg, Some(&confirm_url), Some(&user_id)).await;
    }

    Json(serde_json::json!({
        "status": "success",
        "delivered": delivered,
        "message": "Deletion request recorded. Please check your email for the confirmation link.",
    }))
}

async fn get_data_deletion_summary(
    State(state): State<AppState>,
    Query(query): Query<DataDeletionSummaryQuery>,
) -> Json<serde_json::Value> {
    let row = sqlx::query_as::<_, DataDeletionRequestRow>(
        "SELECT r.id, r.user_id, u.email as user_email, r.status, r.snapshot_json \
         FROM data_deletion_requests r \
         JOIN users u ON u.id = r.user_id \
         WHERE r.token = ?"
    )
    .bind(query.token.trim())
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let Some(row) = row else {
        return Json(serde_json::json!({ "status": "error", "message": "Invalid or expired token" }));
    };

    if row.status == "finalized" {
        return Json(serde_json::json!({ "status": "finalized", "message": "Deletion already completed" }));
    }

    if row.status == "requested" {
        let _ = sqlx::query("UPDATE data_deletion_requests SET status = 'email_confirmed', email_confirmed_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(&row.id)
            .execute(&state.pool)
            .await;
    }

    let snapshot: DataDeletionSnapshot = serde_json::from_str(&row.snapshot_json).unwrap_or_default();

    Json(serde_json::json!({
        "status": "ready",
        "email": row.user_email,
        "snapshot": snapshot,
        "warning": "Cached/overwritten database pages may not be recoverable. The system can only assist deletion and cannot restore deleted data.",
        "require_second_confirmation": true
    }))
}

async fn post_confirm_data_deletion(
    State(state): State<AppState>,
    Json(payload): Json<DataDeletionConfirmPayload>,
) -> Json<serde_json::Value> {
    if !payload.confirm {
        return Json(serde_json::json!({ "status": "error", "message": "Final confirmation is required" }));
    }

    let row = sqlx::query_as::<_, DataDeletionRequestRow>(
        "SELECT r.id, r.user_id, u.email as user_email, r.status, r.snapshot_json \
         FROM data_deletion_requests r \
         JOIN users u ON u.id = r.user_id \
         WHERE r.token = ?"
    )
    .bind(payload.token.trim())
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let Some(row) = row else {
        return Json(serde_json::json!({ "status": "error", "message": "Invalid or expired token" }));
    };

    if row.status == "finalized" {
        return Json(serde_json::json!({ "status": "finalized", "message": "Deletion already completed" }));
    }

    let sender_dir = format!("data/mail_spool/{}", sanitize_path_component(&row.user_email.to_ascii_lowercase()));

    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            return Json(serde_json::json!({ "status": "error", "message": format!("Cannot start deletion transaction: {}", e) }));
        }
    };

    let _ = sqlx::query("DELETE FROM emails WHERE user_id = ?").bind(&row.user_id).execute(&mut *tx).await;
    let _ = sqlx::query("DELETE FROM email_rules WHERE user_id = ?").bind(&row.user_id).execute(&mut *tx).await;
    let _ = sqlx::query("DELETE FROM user_memories WHERE user_id = ?").bind(&row.user_id).execute(&mut *tx).await;
    let _ = sqlx::query("DELETE FROM user_activity_stats WHERE user_id = ?").bind(&row.user_id).execute(&mut *tx).await;
    let _ = sqlx::query("DELETE FROM mail_errors WHERE user_id = ?").bind(&row.user_id).execute(&mut *tx).await;
    let _ = sqlx::query("DELETE FROM chat_logs WHERE user_email = ?").bind(&row.user_email).execute(&mut *tx).await;
    let _ = sqlx::query("UPDATE data_deletion_requests SET status = 'finalized', finalized_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&row.id)
        .execute(&mut *tx)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE id = ?").bind(&row.user_id).execute(&mut *tx).await;

    if let Err(e) = tx.commit().await {
        return Json(serde_json::json!({ "status": "error", "message": format!("Deletion failed: {}", e) }));
    }

    let _ = fs::remove_dir_all(&sender_dir).await;

    Json(serde_json::json!({
        "status": "success",
        "message": "All removable user data has been deleted. Cached/overwritten database pages are not recoverable.",
    }))
}

pub async fn start_server(port: u16, state: AppState) -> Result<()> {
    let api_router = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/about", get(get_about))
        .route("/auth/magic-link", post(post_magic_link))
        .route("/auth/verify", post(post_verify))
        .route("/me", get(get_me))
        .route("/settings", post(post_settings))
        .route("/data-deletion/request", post(post_request_data_deletion))
        .route("/data-deletion/summary", get(get_data_deletion_summary))
        .route("/data-deletion/confirm", post(post_confirm_data_deletion))
        .route("/rules", get(get_rules))
        .route("/rules/create", post(post_create_rule))
        .route("/rules/update", post(post_update_rule))
        .route("/rules/toggle", post(post_toggle_rule))
        .route("/dashboard", get(get_dashboard))
        .route("/errors", get(get_user_errors))
        .route("/admin/errors", get(get_admin_errors))
        .route("/chat", post(post_chat));

    let app = Router::new()
        .nest("/api", api_router)
        .fallback_service(
            ServeDir::new("frontend/dist")
                .not_found_service(ServeFile::new("frontend/dist/index.html"))
        )
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Web server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
