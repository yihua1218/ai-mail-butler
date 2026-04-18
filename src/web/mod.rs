use axum::{
    extract::{State, Query},
    routing::{get, post},
    Json, Router,
};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::cors::CorsLayer;
use std::net::SocketAddr;
use tracing::info;
use anyhow::Result;
use sqlx::SqlitePool;
use serde::{Deserialize, Serialize};

use crate::models::{User, EmailRecord};
use crate::ai::AiClient;
use crate::services::OnboardingService;

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
                    let all_emails = sqlx::query_as::<_, EmailRecord>("SELECT id, subject, preview, status, CAST(received_at AS TEXT) as received_at FROM emails ORDER BY received_at DESC")
                        .fetch_all(pool).await.unwrap_or(vec![]);
                        
                    return Json(serde_json::json!({ 
                        "type": "admin", 
                        "global_stats": global_stats,
                        "personal_stats": personal_stats,
                        "personal_emails": personal_emails,
                        "all_emails": all_emails
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

    let reply = if !email.is_empty() {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(pool).await.unwrap_or(None);

        if let Some(mut user) = user {
            let new_pref = match OnboardingService::extract_preferences(&state.ai_client, &user, &message).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Failed to extract preferences: {}", e);
                    user.preferences.clone().unwrap_or_default()
                }
            };
            sqlx::query("UPDATE users SET preferences = ?, is_onboarded = true WHERE id = ?")
                .bind(&new_pref)
                .bind(&user.id)
                .execute(pool).await.ok();
            user.preferences = Some(new_pref);

            match OnboardingService::generate_reply(&state.ai_client, &user, &message).await {
                Ok(r) => r,
                Err(e) => { tracing::error!("Failed to generate reply: {}", e); "Sorry, I am having trouble connecting to my AI brain right now.".to_string() }
            }
        } else {
            match OnboardingService::generate_anonymous_reply(&state.ai_client, &message, guest_name).await {
                Ok(r) => r,
                Err(e) => { tracing::error!("Failed to generate anonymous reply: {}", e); "Sorry, I am having trouble connecting to my AI brain right now.".to_string() }
            }
        }
    } else {
        match OnboardingService::generate_anonymous_reply(&state.ai_client, &message, guest_name).await {
            Ok(r) => r,
            Err(e) => { tracing::error!("Failed to generate anonymous reply: {}", e); "Sorry, I am having trouble connecting to my AI brain right now.".to_string() }
        }
    };

    // Record this AI reply in chat_logs (NULL email for anonymous)
    let log_email = if email.is_empty() { None } else { Some(email.clone()) };
    sqlx::query("INSERT INTO chat_logs (user_email) VALUES (?)")
        .bind(&log_email)
        .execute(pool).await.ok();

    Json(serde_json::json!({ "reply": reply }))
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
}

async fn get_about() -> Json<BuildInfo> {
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
    })
}

#[derive(Deserialize)]
struct MagicLinkRequest {
    email: String,
}

use lettre::message::Message;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport};
use lettre::message::SinglePart;
use mail_send::SmtpClientBuilder;
use mail_send::mail_builder::MessageBuilder;

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

        // Build message using mail-send's builder (it handles MIME better)
        let message = MessageBuilder::new()
            .from(from_addr.to_string())
            .to(to_addr.to_string())
            .subject(subject)
            .text_body(plain_text.clone());

        let is_implicit = port == 465;
        tracing::debug!(">>> [SMTP] Connecting to {}:{} (Implicit TLS: {})", host, port, is_implicit);

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
                tracing::error!("mail-send delivery failed: {:?}. Login URL is in server console.", e);
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
                        Ok(builder) => {
                            match builder.port(25).build().send(&email_msg) {
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
    display_name: Option<String>,
}

async fn post_settings(
    State(state): State<AppState>,
    Json(payload): Json<SettingsRequest>,
) -> Json<serde_json::Value> {
    let result = sqlx::query("UPDATE users SET auto_reply = ?, dry_run = ?, display_name = ? WHERE email = ?")
        .bind(payload.auto_reply)
        .bind(payload.dry_run)
        .bind(payload.display_name)
        .bind(&payload.email)
        .execute(&state.pool).await;
        
    match result {
        Ok(_) => Json(serde_json::json!({ "status": "success" })),
        Err(e) => {
            tracing::error!("Failed to update settings: {}", e);
            Json(serde_json::json!({ "status": "error" }))
        }
    }
}

pub async fn start_server(port: u16, state: AppState) -> Result<()> {
    let api_router = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/about", get(get_about))
        .route("/auth/magic-link", post(post_magic_link))
        .route("/auth/verify", post(post_verify))
        .route("/me", get(get_me))
        .route("/settings", post(post_settings))
        .route("/dashboard", get(get_dashboard))
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
