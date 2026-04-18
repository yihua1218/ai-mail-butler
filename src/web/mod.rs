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
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(&state.pool).await.unwrap_or(None);
            
        let role = if Some(&email) == state.admin_email.as_ref() { "admin".to_string() } else { "user".to_string() };
        
        if let Some(mut u) = user { 
            u.role = role;
            return Json(Some(u)); 
        }

        let new_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO users (id, email) VALUES (?, ?)")
            .bind(&new_id)
            .bind(&email)
            .execute(&state.pool).await.unwrap();
        return Json(Some(User { id: new_id, email, is_onboarded: false, preferences: None, magic_token: None, role, auto_reply: false, dry_run: true }));
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
    
    let global_stats = serde_json::json!({
        "registered_users": users_count,
        "emails_received": received_count,
        "emails_replied": replied_count,
        "emails_sent": 0,
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
}

async fn post_chat(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Json<serde_json::Value> {
    let email = payload.email;
    let message = payload.message;
    let pool = &state.pool;

    if !email.is_empty() {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(pool).await.unwrap_or(None);

        if let Some(mut user) = user {
            // AI Integration
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
                .execute(pool).await.unwrap();

            user.preferences = Some(new_pref);
            
            let reply = match OnboardingService::generate_reply(&state.ai_client, &user, &message).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Failed to generate reply: {}", e);
                    "Sorry, I am having trouble connecting to my AI brain right now.".to_string()
                }
            };

            return Json(serde_json::json!({ "reply": reply }));
        }
    }

    // Anonymous Chat
    let reply = match OnboardingService::generate_anonymous_reply(&state.ai_client, &message).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to generate anonymous reply: {}", e);
            "Sorry, I am having trouble connecting to my AI brain right now.".to_string()
        }
    };

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
}

async fn get_about() -> Json<BuildInfo> {
    Json(BuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        target: env!("BUILD_TARGET"),
        host: env!("BUILD_HOST"),
        profile: env!("BUILD_PROFILE"),
        git_commit: env!("GIT_COMMIT"),
        build_date: env!("BUILD_DATE"),
    })
}

#[derive(Deserialize)]
struct MagicLinkRequest {
    email: String,
}

use lettre::{Message, SmtpTransport, Transport};
use lettre::transport::smtp::authentication::Credentials;
use lettre::message::{header, MultiPart, SinglePart};

async fn post_magic_link(
    State(state): State<AppState>,
    Json(payload): Json<MagicLinkRequest>,
) -> Json<serde_json::Value> {
    let token = uuid::Uuid::new_v4().to_string();
    let email = payload.email;
    
    // Upsert user and set token
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(&state.pool).await.unwrap_or(None);
        
    if let Some(u) = user {
        sqlx::query("UPDATE users SET magic_token = ? WHERE id = ?")
            .bind(&token)
            .bind(&u.id)
            .execute(&state.pool).await.unwrap();
    } else {
        let new_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO users (id, email, magic_token) VALUES (?, ?, ?)")
            .bind(&new_id)
            .bind(&email)
            .bind(&token)
            .execute(&state.pool).await.unwrap();
    }
    
    let login_url = format!("http://localhost:3000/login?token={}", token);
    
    // Construct real email using Lettre
    let plain_text = format!("Welcome to AI Mail Butler! / 歡迎使用 AI Mail Butler！\n\nClick the link below to securely login without a password. / 請點擊下方連結安全無密碼登入：\n{}", login_url);
    let html_text = format!(
        "<h3>Welcome to AI Mail Butler! / 歡迎使用 AI Mail Butler！</h3>
        <p>Click the button below to securely login without a password. / 請點擊下方按鈕安全無密碼登入：</p>
        <a href=\"{}\" style=\"display: inline-block; padding: 10px 20px; background-color: #0071e3; color: white; text-decoration: none; border-radius: 5px;\">Login / 登入</a>
        <br><br>
        <p>If the button doesn't work, copy and paste this link / 如果按鈕無效，請複製貼上此連結：<br>{}</p>",
        login_url, login_url
    );

    let email_msg = Message::builder()
        .from(state.config.assistant_email.parse().unwrap())
        .to(email.parse().unwrap())
        .subject("Your AI Mail Butler Login Link / 您的登入連結")
        .multipart(
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(header::ContentType::TEXT_PLAIN)
                        .body(plain_text),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(header::ContentType::TEXT_HTML)
                        .body(html_text),
                ),
        )
        .unwrap();

    let mailer = if let Some(host) = &state.config.smtp_relay_host {
        let mut builder = SmtpTransport::relay(&host).unwrap().port(state.config.smtp_relay_port);
        if let (Some(user), Some(pass)) = (&state.config.smtp_relay_user, &state.config.smtp_relay_pass) {
            let creds = Credentials::new(user.to_string(), pass.to_string());
            builder = builder.credentials(creds);
        }
        Some(builder.build())
    } else {
        // Fallback to mock if no SMTP is configured
        None
    };

    if let Some(mailer) = mailer {
        match mailer.send(&email_msg) {
            Ok(_) => {
                tracing::info!("Magic link sent successfully to {}", email);
                return Json(serde_json::json!({ "status": "success", "message": "Magic link sent to your email" }));
            },
            Err(e) => {
                tracing::error!("Could not send email: {:?}", e);
                return Json(serde_json::json!({ "status": "error", "message": "Failed to send email" }));
            }
        }
    } else {
        tracing::warn!("No SMTP configured. Mocking email delivery to console:");
        tracing::info!("To: {}\nLink: {}", email, login_url);
        return Json(serde_json::json!({ "status": "success", "message": "Magic link sent to console mock" }));
    }
}

#[derive(Deserialize)]
struct VerifyRequest {
    token: String,
}

async fn post_verify(
    State(state): State<AppState>,
    Json(payload): Json<VerifyRequest>,
) -> Json<Option<User>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE magic_token = ?")
        .bind(&payload.token)
        .fetch_optional(&state.pool).await.unwrap_or(None);
        
    if let Some(mut u) = user {
        // Clear token
        sqlx::query("UPDATE users SET magic_token = NULL WHERE id = ?")
            .bind(&u.id)
            .execute(&state.pool).await.unwrap();
            
        u.magic_token = None;
        u.role = if Some(&u.email) == state.admin_email.as_ref() { "admin".to_string() } else { "user".to_string() };
        Json(Some(u))
    } else {
        Json(None)
    }
}

#[derive(Deserialize)]
struct SettingsRequest {
    email: String,
    auto_reply: bool,
    dry_run: bool,
}

async fn post_settings(
    State(state): State<AppState>,
    Json(payload): Json<SettingsRequest>,
) -> Json<serde_json::Value> {
    let result = sqlx::query("UPDATE users SET auto_reply = ?, dry_run = ? WHERE email = ?")
        .bind(payload.auto_reply)
        .bind(payload.dry_run)
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
